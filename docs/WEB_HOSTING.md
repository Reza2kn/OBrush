# OBrush.App hosting

The browser build is a static application. Sculpting and file processing run on
the visitor's device; the web server serves the Rust/WASM bundle and assets.
Hosting this bundle does not add a remote model upload endpoint.

The public application is live at [OBrush.App](https://obrush.app/) and
[www.obrush.app](https://www.obrush.app/). The initial VPS deployment uses clean
source `a115aac55bf300c98866ad5ba7f992aa52117705` and the O icon. The
[retained hosting evidence](evidence/obrush-app-hosting-2026-10-20.json) records
exact asset hashes and fresh Chrome checks for the normal SIMD route and the
portable scalar fallback: startup, a short default Draw stroke, valid project
download, blocked-download warning and canceled leave-page navigation passed.
Both domains returned HTTPS 200 with certificate verification. This deployment
does not resolve the existing Draw plateau, worked-surface quality or dense-mesh
latency issues. The public `v0.1.20-alpha.1` archives remain unchanged.

## DNS and server prerequisites

Point the apex `obrush.app` A record at the chosen VPS IPv4 address, and set
`www` to a CNAME for `obrush.app`. Remove conflicting A/AAAA records. Both names
must resolve exclusively to the VPS before activation. The `.app` domain needs
working HTTPS.

The existing Ubuntu VPS uses Caddy, with this import already present in its main
configuration:

```caddyfile
import /etc/caddy/sites-enabled/*
```

Keep existing site files and applications intact. Deployment adds only the
managed `obrush.caddy` site and `/var/www/obrush`. It reloads Caddy after checking
the complete configuration. It does not restart other applications or prune
data to create disk space. Free space must cover the uploaded bundle plus
128 MiB; leave additional operating headroom on a shared server.

## Deploy a verified build

Use existing SSH authentication. Credentials belong in the SSH agent or another
local credential store, never in this repository or deployment manifests.

```sh
scripts/build-web.sh
python3 scripts/deploy-web-vps.py --host root@YOUR_VPS --expected-ip YOUR_IPV4 --stage-only
```

The checkout must be clean and match the build's source commit and Cargo.lock.
The script verifies all three WASM hashes, then streams the bundle over SSH.
Every uploaded file is checked against a SHA-256 manifest before the release
directory is finalized. `--stage-only` uploads a versioned release without
changing the active site. Once DNS is correct, run the same command without
`--stage-only` to activate it.

The deployment keeps prior releases in `/var/www/obrush/releases`, atomically
switches `/var/www/obrush/current`, validates the full Caddy configuration, and
reloads it. An activation error restores the previous symlink and site file.
Incomplete uploads are retained for inspection. No automatic pruning occurs.
Caddy obtains and renews certificates using its existing configuration.

## Verify the consumer route

Check the public HTTPS certificate and both domain names, then fetch
`/build-info.json` and compare the source commit and three WASM hashes with the
local bundle. Download and hash the actual served WASM files, icon and entry
page. Open the production URL in a fresh browser profile, verify startup,
sculpting, project download and the unsaved-work warning. Save a screenshot and
the numerical/browser receipt with the source and artifact hashes.

A successful upload or Caddy reload alone does not establish that the public
application runs correctly. Do not claim deployment while DNS or HTTPS is still
pending.

For rollback, select a retained release directory, switch `current` with an
atomic symlink rename, and reload only if the Caddy site configuration changed.
Verify the public build identity again. Published alpha archives remain
immutable.
