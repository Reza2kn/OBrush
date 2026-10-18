# C199: one retained-state match per prepared Source dab

The normal browser's prepared Source geometry dab previously matched the same
retained mesh twice: once while preparing a ray contact, then again inside
`PreparedSourceGeometryDab::apply` under the same exclusive `Document` borrow.
The prepared value now carries a private boolean that is true only when the
retained state matched during contact preparation. The apply gate reuses that
proof for the same borrowed mesh; a missing/failed retained match and the
separate public dab entry still run their complete checks. Mesh convention,
state policy, stroke binding, domain, limits and history gates remain in place.
The unit regression observes one retained match on a warm prepared dab; the
existing stale-position, topology, normal, mask, color, material, UV and
convention mutations still reject without publication. This proof is never
retained across dabs or callers.

The frozen normal-browser cohort uses the independently authored 999,002-vertex,
1,998,000-triangle SGL fixture SHA-256
`dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`
and the 60-second/400-requested-moves-per-second protocol SHA-256
`234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`.
No alpha, mirror, clay, accumulate or fold guard is used. The C199 served WASM
SHA-256 is `f7b5145ca71ac206dd8e198189fe2cde478dc2623caa02eaddfb371119cb76ec`.

| Normal-browser observation | C197 run | C199 run |
| --- | ---: | ---: |
| Delivered moves / applied dabs | 1,770 / 805 | 1,882 / 863 |
| Mean complete queued dab | 24.53 ms | 19.55 ms |
| Mean deformation envelope | 17.18 ms | 12.20 ms |
| Mean retained preparation match | 7.22 ms | 7.22 ms |
| Independent RAF P95 / maximum | 77.5 / 599.7 ms | 72.4 / 609.3 ms |
| App paint CPU P95 | 76.9 ms | 71.9 ms |
| Release acknowledgement | 288.2 ms | 263.8 ms |

The 4.98 ms lower mean deformation time is consistent with removing one
retained comparison from that envelope. These are separate browser runs with
different delivered moves and valid edited shapes. C198 also changed keyboard
camera routing between their bundles. They do not establish a controlled
end-to-end speed ratio. All four responsiveness gates still fail: RAF P95
≤33.34 ms, maximum ≤250 ms, app CPU P95 ≤16.667 ms, and release ≤100 ms.
The normal studio still uses WASM CPU sculpting and WebGL2 drawing.

The raw C199 browser result at
`.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-f29195d3-62d9-42be-89cf-a7fa59754125/result.json`
records zero application/console errors and owned Chrome exit. Its result
SHA-256 is `c2541bbc156aa62af67ff3a56096b3829ef3031e3b671ded44d65ab31d575e49`.
The independent full-project audit at `.artifacts/c199-audit.json` (SHA-256
`435502b1e852434cb77dee60bfd44311229b9caf77a4bc60866b86cb50cdeb4c`)
checks every public vertex, triangle and non-position channel, exact Save,
stable Save and Undo/Redo files, and all affected face orientations and areas.
It finds 40,806 changed vertices, 82,433 affected faces, no new degenerates or
orientation reversals, and maximum changed-edge stretch 2.1865. Its reusable
oracle is `scripts/audit-source-sustained.py`.

Sustained dense-mesh response remains far behind the pinned SculptGL browser.
The remaining measured work includes the still-full retained preparation
match, retained state advance, main-thread rendering and release validation.
This is a measurable core improvement, not SculptGL or ZBrush parity.

Final validation: `cargo fmt --all -- --check`, 2,508 passing workspace
tests with zero failures and one ignored across 83 groups, strict native and
`wasm32-unknown-unknown` Clippy, `scripts/build-web.sh`, and
`scripts/package-macos.sh` passed. The macOS package signature verified.
