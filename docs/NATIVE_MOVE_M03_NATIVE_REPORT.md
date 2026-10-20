# M03 packaged native Move consumer check — inconclusive

The [frozen M03 gate](NATIVE_MOVE_M03_NATIVE_PREREG.md) did **not** reach a
Move contact. This is a failed setup/consumer observation, not evidence that
the M02 input fix resolves the user's pointed or triangular Move artifacts.

The ad-hoc-signed packaged executable was
`8dc4e14d400afbad61da81a796869f4feae4deaa692c0614b0c210a2ee6c2d18`
(SHA-256), from M02 commit `a0c2b1947e5bcba3b7b03f2a7777cad3842db1d9`.
The native app displayed a fresh default sphere with 40,962 vertices and
81,920 faces. Its visible Move tile was selected, X symmetry was checked,
and the radius slider displayed `0.037`. At strength-slider coordinate
`[288,1329]` the UI displayed `78%`; a further coordinate click intended
to set `79%` returned `Computer Use server error -10005:
noWindowsAvailable`. The actual settings screenshot is retained at
`.artifacts/native-move-m03/settings-78percent.png` (SHA-256
`1bbe9179fdc7189ce57f3380cb70acd26816a5a7dc81bfc4e7bfa516c5794d45`).

The pointer error persisted for a slider click, a header click, and a
surface drag after rebinding the exact packaged `.app`, raising the AX
window, and resetting the computer-use session. `getAXState`, screenshots,
and keyboard shortcuts still worked. The macOS Save dialog initially showed
a disabled Save button in its collapsed state; expanding it and navigating
to the isolated receipt folder through its normal UI enabled Save. The
pre-contact public project was saved as
`.artifacts/native-move-m03/m03-before.obrush` (SHA-256
`2b88d59fc06ab6eed9ef25b51de267ff3791b00438e84a7d3247e1f651371fbb`,
version 2, 5.1 MiB). This preserves the untouched native starting mesh.

No subsequent surface event could be delivered through the available
native UI control. Therefore there is no held-event timing or pick witness,
no after project, no Undo/Redo pair, and no before/after face-quality
measurement. All M03 product and visual gates remain **open**. The M02
core/host tests and browser burst receipt remain separate evidence and do
not substitute for this native consumer gate. A rerun needs working native
coordinate drag control with observable held-event timing; retain this
failed receipt when that occurs.
