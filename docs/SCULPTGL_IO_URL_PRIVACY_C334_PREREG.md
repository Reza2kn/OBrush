# C334: Startup model URL privacy

This is an isolated candidate, pending full integration checks. The pinned
SculptGL inventory's `SGL-IO-URL-MODEL` gate requires valid and failed URL
loads without credential logging. OBrush already has bounded OBJ/PLY/STL/SGL
URL startup, but its public diagnostic metrics previously serialized the
literal request URL twice: in `SourceStartupSettings` and in the pending
`ModelRequest`. Native curl failure diagnostics could also echo that URL, as
could implementation-defined browser exception strings. A URL containing
`?token=...` or `user:password@` therefore had a disclosure route.

The candidate replaces both diagnostic URL values with `[redacted]`, reports
a failed native curl exit status without raw stderr, and allows only fixed
credential-free browser transport errors into the UI. The private captured
URL remains intact for transport. This is necessarily a host-boundary change:
the shared `obrush-core` mesh parser, document installation, history, response
limits, and native/WASM geometry behavior are unchanged. The two new Rust
tests check both public metrics fields; `scripts/check-startup-model-privacy.mjs`
checks browser transport error redaction and byte-exact successful delivery
without starting a browser. That Node check passed in the isolated worktree;
Cargo and normal browser checks have not run because another agent owns that
lane.

## Frozen validation gate

1. Run `cargo fmt --all -- --check`, complete workspace tests, strict native
   and `wasm32-unknown-unknown` Clippy, and official web/macOS build scripts
   on the integrated candidate. Run
   `node scripts/check-startup-model-privacy.mjs` again from that source.
2. In normal browser and native host, serve an independently authored
   four-vertex OBJ from localhost at a URL with a synthetic query sentinel.
   Inspect public startup metrics while GET is pending and after completion:
   no query sentinel or userinfo may appear, while import and exact Undo/Redo
   match the current baseline. Trigger 404 and malformed OBJ with the same
   sentinel: no successful import, no scene/history mutation, and no sentinel
   in UI diagnostics or captured output. Do not retain real credentials in
   fixtures or receipts.

Cross-origin CORS, redirect chains, large URLs, native physical GUI and
sustained responsiveness remain open. This fixes a concrete disclosure path;
it does not complete the URL-model workflow or any SculptGL/ZBrush parity row.
