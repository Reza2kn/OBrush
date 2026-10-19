# Source Worker cross-realm diagnostic

This is a transport and checked-publication seam, **not** an ordinary browser
brush route or a speedup claim. The normal held-pointer Source path still runs
in the studio realm. The release browser UI does not expose the cold capture
methods. A module Worker can own a display-free Rust/WASM `Document` and Source
claim, accept ordered inputs, send sparse position/normal rows, and return a
warm retained-state packet. The studio can publish that result only through a
one-use cold capture that verifies epoch, revision, complete scene, target,
packet checksums, retained-state binding, and history budget. Publication
changes public geometry and retained state as one history step; a no-op does
not consume redo.

The independent cross-realm fixture is a single unprotected triangle with
vertices (0,0,0), (4,0,0), (0,4,0), an orthographic view, one Inflate input at
screen (320,192), radius 220 px, pressure 0.5, intensity 0.3, and no alpha or
symmetry. `node scripts/check-source-worker-realm.mjs web/dist` launched a real
Chrome module Worker and passed with 3 changed rows in both the intermediate
and final 172-byte deltas, a 520-byte retained packet, and one accepted dab.
Fixture SHA-256: `a6033ae61f3df2ecc5083ea96b1fa0b1df0c319b52d67db7c2cec438245c9f97`.
Options SHA-256: `1fc6abbfc41d4980f78313674284593273e209b9a8f3ed71c2982c7f715cd71a`.
Input SHA-256: `8d7996e8836b22f711728ef11927864664a82fca2ac88f7ef6425200d55bf51f`.
WASM SHA-256: `33425c5c674d68744e4309a6a6cd34a5b38dc517d0bfeeafa36fa17d0251b82e`.
The ignored local receipt is `.artifacts/source-worker-realm/2e3fd0ea-17a3-47fd-9309-202625d696fd/result.json`.

Focused cold-capture tests passed 5/5, including an actual
`SourceWorkerTransaction` result, exact public JSON, Undo/Redo, subsequent warm
dab equality, rejected stale scene/revision, and preserved redo on no-op.
Native strict Clippy, WASM strict Clippy and `scripts/build-web.sh` passed.
The first WASM Clippy attempt failed because the worker used a private
fallible mesh snapshot API and a cancellation closure lacked `mut`; those
integration errors were fixed before the successful checks.

This fixture has only three vertices. It does not test the ordinary input
queue, pointer release, GPU publication, five public Save states, 100k or 1m
meshes, or sustained 400 Hz pressure. The current per-input delta scans and
validates whole meshes and copies the current mesh, so its cost may dominate
at scale. Ordinary activation requires a bounded live-preview protocol and
release ordering that preserve the studio's single Document/history owner,
followed by the frozen SP-WORKER-01 browser protocol at both mesh sizes.
