# OBrush 0.1.15 candidate 2: targeted browser checks

Eight bounded local cases pass against the immutable candidate 2 web build. The
same run retains a failed ninth gate: the attempted normal Egui keyboard UI
scale shortcut did not change the viewport. This is partial browser acceptance,
not SculptGL or ZBrush parity. The [candidate 1 failures](SCULPTGL_HOST_CANDIDATE1.md)
remain unchanged; the [baseline inventory](SCULPTGL_PARITY.md) is not promoted.

The [machine-readable receipt](evidence/sculptgl-host-runtime-candidate2.json)
binds 33 actual browser downloads to 33 complete GPU/project comparisons,
independently repacked as f32 vertex attributes and u32 triangle indices. Every
actual served HTML/JS/WASM response matches the frozen web file hash. The topology
worker is frozen but was not requested. Trusted pointer, keyboard and file input
events reached the production routes; no application, tool, camera or mesh state
was assigned through JavaScript. The original download bridges remained active.

## Frozen build and executed tooling

- Candidate directory: `.artifacts/sculptgl-release-0.1.15-candidate2`.
- Source manifest SHA256:
  `8b9b74041ec0dcaebf27f094afa978a11021e015973e48f15390aabeb673669d`.
- Build record SHA256:
  `7f7f1a9ba7aa24185a02d001670c3d8c1b7102f3046ba744f1dc026714ea1fc1`.
- Frozen helper SHA256:
  `7258c4c508c2407bbb9352f5e07d87e8b35f5eeaf9ea672ecf6ce7b0f4c0c199`.
- Executed helper SHA256:
  `48a5d7b360941084ac4ee03e62e461e1b4e7c53fbaa7c8e77a694d6eeec41436`.

The helper correction after the build freeze affects only its independent,
read-only primitive contact oracle. Offline preparation retained a projection
identity failure of `2.6094477561855456e-5` pixels from combining the separately
serialized eye and matrix, and a fixed sampling grid that missed an offset torus
in the prior cumulative scene. Coherent inverse-matrix near/far endpoints and
projected face candidates resolve those helper defects without widening the
original numerical gates. The independent projection errors on the four retained
primitive meshes are at most `7.207745511550456e-11` pixels; barycentric point
reconstruction errors are at most `1.0547118733938987e-15`. Original helper copies,
failure explanations and validation records remain in
`.artifacts/sculptgl-host-runtime/`. Candidate binaries were not changed.

## Navigation and the original wheel event

The middle pan and secondary orbit traces each dispatch an ordered press, move
and release as one synchronous CDP message batch. Passive pointer observations
group all three events within one rendered scene frame. The pan moves the target
by `0.16416528552146076` world units and the eye by
`0.16416520105473575`; their translation vectors differ by at most
`9.499490261077881e-8`. Orbit changes the normalized eye-to-target direction by
`0.2500457239253123`. Both preserve camera distance, the exact saved project and
the complete visible GPU buffers. Release probes show no further camera or mesh
changes and no pending dabs. Internal Egui update IDs are not observed.

The original raw wheel event remains `deltaY = -240`. On the independently
authored approximately unit sphere, camera distance changes from
`3.4184410572052` to `2.6622846126556396` and stays outside the fixture. It leaves
the saved project exact and permits a second useful Draw stroke with the normal
fixed 75-pixel radius option. Both strokes preserve fully masked original points;
the second stroke has exact whole-project undo/redo and a quiet release. This
does not establish a global camera collision policy: deliberate repeated zooming
can still approach or cross a surface.

## Fresh primitive Draw checks

The normal Cube replacement and three Add routes use a fresh application session,
Draw, radius `0.180` and strength `35%`, with production default mesh detail.
Actual ray/triangle contacts are computed from saved geometry and the read-only
camera matrix, then driven by trusted mouse input. One positive Draw trace is
checked per shape.

| Route | Initial vertices | Initial triangles | Original vertices changed |
| --- | ---: | ---: | ---: |
| Replace Cube | 24,578 | 49,152 | 292 |
| Add Cube | 24,578 | 49,152 | 171 |
| Add Cylinder | 66,562 | 133,120 | 1,596 |
| Add Torus | 65,536 | 131,072 | 5,827 |

Each trace makes a useful position edit, preserves every existing other part,
matches the actual complete GPU buffers to the downloaded project, restores exact
whole-project undo/redo and remains quiet after release. The cube traces also
refine locally. These samples do not establish all-brush or long-overlap surface
quality, a complete primitive reference comparison, or native GUI acceptance.

## Retained unsupported UI-scale gate

The normal `Cmd+=` event reaches `obrush_canvas` as trusted keyboard input, but
the viewport remains unchanged. Browser page dimensions and scale, saved project
and GPU buffers remain exact. The scaled Egui navigation tests therefore were
not reached.

The pinned `eframe 0.33.3` browser runner explains the missing route: in
`src/web/app_runner.rs`, lines 60–65, it explicitly sets
`zoom_with_keyboard = false` and `zoom_factor = 1.0`, leaving zoom shortcuts to
the browser. An earlier inference from Egui's default keyboard zoom behavior
alone was incomplete. Candidate 2 exposes no normal internal UI-scale control or
read-only scale metric, so no exact scale or scaled navigation claim is made.
No shortcut variations were rerun to erase this failure.

Only an owned, isolated headless Chrome 155.0.8059.40 profile and localhost server
were used, at 1600×1000 and DPR1. Nonlocal requests were blocked. The owned Chrome
process exited with code 0, its PID was confirmed gone, and its server closed.
Release probes include asynchronous GPU readback and are functional checks, not
performance or input-to-photon measurements. No native apps, accessibility
features, user tabs or user profiles were controlled.
