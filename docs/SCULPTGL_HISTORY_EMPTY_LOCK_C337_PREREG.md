# C337: Empty locked stroke versus an available Redo branch

This is a static discrepancy and a frozen normal-browser gate, not a paired
result or a product change. The pinned reference is SculptGL source commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`; the inspected OBrush base
is `1ecc42def96bd8dbe90b340d4aa4b8233ac134a1`.

The source path suggests that an empty locked Brush contact **discards an
existing Redo branch**. `SculptBase.start()` pushes a geometry state after a
hit. `StateManager.pushState()` clears `_redos` immediately. With Lock on,
`SculptBase.startSculpt()` skips the initial sculpt and a stationary contact
has no update. `SculptGL.onDeviceUp()` calls `cleanNoop()`, which removes the
empty Undo state but also clears `_redos`. OBrush's `Document::end_stroke()`
returns when its before/after states match and deliberately preserves Redo.
Both behaviors leave the visible mesh unchanged; a normal geometry comparison
alone would miss the history difference.

## Frozen same-workflow browser gate

In each app, use the normal browser UI to reset to an empty scene, add a
sphere, add a cube, set history length to 10, and Undo the cube through the
visible History control. Save the app's public scene file and confirm one
sphere with a useful Redo that restores the cube. Undo the cube again if the
availability probe used that Redo.

Select Brush and turn on its **Lock position** control. Pick an interior
sphere point away from the cube's former outline. Issue exactly one trusted
pointerdown and pointerup at the same CSS pixel with no pointermove, wait for
normal completion, then Save again. The scene bytes, visible geometry and
camera must be unchanged from the pre-contact sphere state. Capture public
History/UI state and press the normal Redo control once, then Save a third
time. The decisive comparison is whether the cube returns: the static
prediction is *no* in pinned SculptGL and *yes* in OBrush. Record exact
pre-contact, post-contact, and post-Redo file hashes, object counts/order,
history-depth observations, source/OBRush build hashes, browser version and
all input coordinates. Read-only source internals may explain the result but
must not substitute for the visible Redo action and public file.

Repeat the sequence with the same normal pointer path but Lock off and one
nonzero brush displacement as a positive control. That edit should create a
real state, invalidate Redo in both apps, and itself Undo/Redo exactly within
each app. If the primary stationary contact fails to hit the mesh, opens a
modal, moves a vertex, or changes scene bytes, retain the failed receipt and
correct only setup before inferring history behavior. Do not change
`Document::end_stroke()` before the paired run; matching a source path that
may be an undesirable user-visible quirk requires actual normal-UI evidence
and a separate policy/compatibility decision.

Pinned source SHA-256 values: `GuiStates.js`
`3ca3d9c97eceae1a641b069bcc068f0e0cf1be38a7f7c78c0ed68ab18c29fac5`,
`StateManager.js`
`54930095d934df833bbaf246c69af8bf0bf23aa640e0ac40c1b26c1f61441e74`,
`SculptBase.js`
`d7d19e167189041b2bcc91d4e073e6ab9b41d9a57747ab018d763bc8870c3634`,
`SculptGL.js`
`0941577e0b8b0c5cc93e6ca63fa69ea630a730661e183e38e4ba964b1bff1462`,
and `GuiSculptingTools.js`
`1440a49252a6a95ac912ea5ad6b6ce0235d82e09813c94b16690794c7a96e0bf`.
OBRush `document.rs`, `app.rs`, and `shortcut_profile.rs` SHA-256 values are
`5a80b738abe259e6427c258c7a579ccbd54448918f58116a46584d14514bc71a`,
`eead6aa7221ddc52bae11cb8e3fc92f5aee14acb2869023b06e7a26891273d40`,
and `2071bf78afa79d3e7de28450c8f1313fbe7f6dcf2f2c13043423c5ed8d49132b`.

The existing C175–C177 reports already pair 3/10/50 history limits and
mixed scene/sculpt/topology traversal on smaller workflows. This new gate
does not repeat or replace those receipts. Memory-pressure eviction, long
sessions, native physical UI, every editing command and full
`SGL-HISTORY-*` parity remain open. No Cargo or browser execution was used
to prepare this static gate.
