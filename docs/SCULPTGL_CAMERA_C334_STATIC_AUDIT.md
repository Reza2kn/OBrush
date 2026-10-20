# C334 pinned camera-control audit and picked-pivot candidate

Current-state audit at OBrush base `1ecc42d`, against SculptGL commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`. The pinned source hashes
and a proposed normal-browser gate are in
[C334's preregistration](SCULPTGL_CAMERA_PICKED_PIVOT_C334_PREREG.md). This
audit changes no inventory row status.

| Family | Pinned reference route | OBrush current route | Remaining evidence |
| --- | --- | --- | --- |
| Projection/FOV/modes/speed | `GuiCamera.js` registers projection, FOV 10–90, Orbit/Spherical/Plane and speed. | `app.rs::camera_panel` exposes these; `renderer.rs::Camera` has matching state names and Source projection math. | UI/runtime matrix and nondefault FOV/mode pairs remain open. |
| Front/back, left/right, top/bottom, reset | `GuiCamera.js` uses paired toggle buttons; `Camera.js` has each opposite orientation. | `app.rs::camera_panel` has Front/Left/Top, `renderer.rs::axis_view` toggles opposite sides; Source shortcut resolver binds F/T/L/Space. | Picked focus, shortcut delivery and paired animation endpoints remain open. |
| Orbit, pan, zoom | `Sculptgl.js` routes right, middle and wheel; `Camera.js` owns motion. | `navigation.rs` captures right/middle displacement; `app.rs::navigate_camera` and `renderer.rs` apply motion. | Mouse gains, modifier chords, gesture timing and native/browser pairs remain open. |
| Picked orbit pivot | `Camera.start` ray-picks on right down when `_usePivot`; `setPivot` shifts center/offset and perspective zoom without a visible jump. `getOptionsURL.js` defaults pivot to true. | Before C334, `camera_panel` disabled Pivot in Source framing and `navigate_camera` explicitly rejected Source pivot; the Source pose always used zero plane/depth offset. | C334 candidate adds a camera-local offset and ordered press capture; its browser gate has **not run**. |
| Touch focus/fit, pan, pinch | `Sculptgl.js` registers Hammer one/two/three-finger pan, pinch and double taps. | `web/index.html` has touch-action none and pressure delivery but no corresponding camera recognizer. | Separate touch workflow and mobile-browser gate; C334 does not cover it. |

The candidate modifies app/renderer/Source camera host/navigation and one
display-only transition channel. It keeps the saved mesh, import/export and
history code untouched. The Source pivot defaults on when Source framing is
first enabled, and a visible checkbox can turn it off. On right-button down,
the candidate picks the world surface point; it stores the inverse local
offset and source zoom that keep the current view fixed until motion. Fit and
Reset clear that offset. The browser and full Cargo gates are pending, so
this is a reviewable implementation candidate rather than accepted parity.

Known constraints to examine in the browser: the prior Source orbit angular
gain may differ from SculptGL; panning/zooming after a picked orbit does not
yet implement Source's independent `_trans` channel; nondefault FOV, large
affine scenes, empty-background pivot presses and Spherical/Plane motion need
their own comparisons. None is inferred from this static audit.
