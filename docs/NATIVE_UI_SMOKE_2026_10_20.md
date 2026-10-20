# Native macOS UI smoke, 2026-10-20

This is a local consumer check of the packaged OBrush app, not SculptGL or
ZBrush parity. The app was launched through its macOS window, and its ordinary
visible controls and keyboard shortcuts were used. No diagnostic application
API changed the document.

The package was built from the integrated source tree at `0bbfa4a` (later
commits through `9d00d18` changed scripts, documentation, or tests only).
The ad-hoc-signed `.artifacts/OBrush.app/Contents/MacOS/OBrush` executable has
SHA-256 `61421c5c1102d7102e1fc25f065fd4189de45d454cee90753b0d21d1995bbcc2`;
`codesign --verify --deep --strict` passed. Host macOS is 26.5.1 (25F80).

Starting with the app's new 40,962-vertex sphere, Draw was selected. A native
window drag from `(1430,950)` to `(1500,950)` with fresh-app default settings
made one undoable edit. Saving through the visible **Save project** button to
`.artifacts/native-smoke-after.obrush` produced SHA-256
`63887d7d0d19ecfc132f23fbf48773a10fa11693146121abefd3da98d6c6b57c`.
The normal Command-Z action displayed “Edit undone”; its saved project,
`.artifacts/native-smoke-before.obrush`, has SHA-256
`2b88d59fc06ab6eed9ef25b51de267ff3791b00438e84a7d3247e1f651371fbb`.
Between these public files, 834 of 40,962 position rows differ, with maximum
Euclidean displacement `0.0082846564563052` world units. Normals changed;
triangles, masks and colors did not. An independent double-precision check of
all 81,920 saved triangles found no zero-area faces or original-face normal
reversals; minimum `|cross| / longest_edge²` quality remained `0.68822982516944`
before and after this small stroke.

Command-Shift-Z displayed “Edit restored”. The normally saved Redo file was
byte-identical to the first After project. Opening the Before file through
the visible **Open** dialog displayed its loaded status, and saving it again
was byte-identical to the original Before project. All four files are retained
locally under `.artifacts/`, which is ignored by Git.

This one macOS gesture establishes a useful Draw edit, exact public Undo/Redo
project recovery, and exact native project reopen/save for this fixture. It
does not measure physical input latency, other brushes, pen pressure, long
sessions, GPU words, or reference response. The native UI's controls are drawn
in the canvas and did not expose per-control accessibility elements in this
session; the actions used screenshot-derived coordinates and visible dialogs.
