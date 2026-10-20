# M02 browser consumer smoke

**Frozen before the browser run.** Serve this worktree's official
`scripts/build-web.sh` output in a private loopback server and launch owned
headless Chrome with an isolated profile. Record the portable and served WASM
SHA-256, browser version, actual visible Move selection and brush metrics,
viewport/canvas bounds, screenshots, trusted DOM pointer events with RAF
indices, and downloaded public `.obrush` projects. Use the visible `7` brush
shortcut and normal mouse contact on a fresh sphere. Save before and after
through the ordinary app Save command, then Undo and Redo through ordinary
shortcuts. Do not call a geometry mutation API.

The held-frame control uses a press near the center/front of the sphere,
eight movement positions in 5-pixel X / 2-pixel upward increments separated
by at least 20 ms, and release at the last point. Pass requires actual Move
selection, finite in-viewport trusted pointer events, at least eight changed
original vertices in the saved project, one-step exact Undo to the before
project, exact Redo to the after project, and no runtime error. Record the
accepted radius/strength rather than assuming a UI slider value.

A separate burst sends press, eight movement positions, and release as fast
as the owned CDP session permits on a fresh sphere. It may be called a
**browser same-frame witness** only if all contact DOM events have one RAF
index and the before/after saved project passes the same geometry/history
checks. If the browser divides the contact across RAFs, retain that timing
and label it only a fast-contact smoke. A CDP burst is not physical input
and cannot prove the earlier native CUA event path or visual tear closure.

Any failure keeps its screenshots, projects, trace, and build hashes.
