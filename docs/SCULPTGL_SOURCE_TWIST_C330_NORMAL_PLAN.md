# C330 Source Twist selected-normal refresh plan (frozen before product edits)

This follows the failed C323 normal-UI gate. Keep the C323 Twist radial kernel
candidate and the face/fold guard unchanged. Pinned Source is
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, its Twist JS SHA-256 is
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
and the independent C306 SGL fixture SHA-256 is
`a2d6b8a12528bc170581705f7fb270dd82ead0e2f974983c207e99b7509cae4e`.
Do not edit the frozen C318 capture/scorer (`5ea518ad24d4fe0f4906fd18f498dd032bc1bfa5ba76629d6deb26b`,
`e2a991c0186ef1b8f4ad82e243af4dda03c64e6a383e0ce2cf2925798f861a4f`).

The failed C323 receipt is SHA-256
`a56274e56b1c5f95d4580efd290a79a18a265c63c5b46b8feb8d3e87b56737c8`.
First-held changed XYZ agrees within `1.4901161193847656e-8` over the same
37 IDs, but active GPU normal max diff is `3.113818820565939e-5`.
Source refreshes 37 moved IDs plus protected stationary ID543; OBrush also
refreshes 28 stationary neighbors. ID611 is the largest error: Source keeps
the Before normal while OBrush changes it. `Twist.stroke` passes picked IDs
to `mesh.updateGeometry`, whose vertex-normal update writes those IDs only.

Candidate scope: use the existing retained Source selected-normal producer for
the imported Source Twist host, seed it before the ordinary stroke snapshot,
refresh only Source-picked IDs after an accepted Twist movement, and render
that producer during the held stroke and idle. Preserve public Mesh normals,
other brush routes, and the deformation guard. If retaining and publishing
normal state cannot be made atomic under the existing memory/history budget,
reject the candidate rather than silently falling back to a different GPU
normal recipe.

Gate: normal-UI paired Source/OBRush C318 first-held positions <= `1e-6`,
active GPU normals <= `1e-6`, exact touched-ID sets including protected ID543,
stationary ID611 unchanged, then held2/After/Undo/Redo and saved positions.
The known held2 folded Source target is expected to remain a position failure
while the guard is kept. Core tests must cover selected versus adjacent
normal rows, mask-protected picked row, no-op/Undo/Redo retention, bounded
failure atomicity, and unchanged Local Scale/ordinary brush behavior.
Required checks after code changes: fmt, workspace tests, strict Clippy,
WASM target, browser build and frozen normal-UI pair. No checks run until the
shared browser/Cargo lane is released.
