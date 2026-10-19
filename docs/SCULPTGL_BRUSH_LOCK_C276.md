# C276 integer-coordinate Source Brush Lock: scoped paired pass

The preregistered C276 normal-UI pair passed for one Source Draw/Brush Lock
gesture with an independently authored 10,114-vertex RGB-ID sphere and 16×16
asymmetric hook alpha. Both SculptGL and OBrush used a 1024×808 logical sculpt
viewport, orthographic front fit, radius 64 screen points, intensity 25%,
zero pressure factors, and the same integer press/+80 X/-80 Y/back +80 X/release
path. Clay, Accumulate, Negative, Culling, X symmetry, Continuous and Prevent
folds were off. All controls, imports, strokes, Save, Undo and Redo used normal
browser UI. The pinned SculptGL source commit was
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d` and hosted JS SHA-256 was
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.

The new Source result is local ignored
`.artifacts/sculptgl-brush-lock-ui/source-18f028e4-e1e8-4b0e-9546-e238ecaa1192/result.json`
(SHA-256 `ad77afb74dbf7269bd0fb3e8a632def236be2bee418e628485e341140ce96747`).
The OBrush result is
`.artifacts/sculptgl-brush-lock-ui/root-b4dea9cd-f62a-48cf-ba9d-2df6e862ae08/result.json`
(SHA-256 `4597bd2639cbe6826a21f24a7be088d95bcd21a575c366fe61a7a2813aa94406`).
The complete paired audit is
`.artifacts/sculptgl-brush-lock-ui/source-18f028e4-e1e8-4b0e-9546-e238ecaa1192/paired-audit-root-b4dea9cd-f62a-48cf-ba9d-2df6e862ae08.json`
(SHA-256 `d2f15ce9ff2f5d541913d4363da07d69d633c716971c71a7d6c051465405c726`).
Both browser captures used the same harness SHA-256
`80873618d94c59551042b78afceca2880d699a92b55c679dba0f263d5df176fe`.
The fresh OBrush portable WASM/JS SHA-256 values were
`df38aa2544ce3cda77b56869af6ee51ceeee3fd2f6540a2dc21d52b31c2568c8`
and `03d74bea813d209f42f0aecbabcf7a4f85d39d509ed8b65f742def4ec3d4b440`.
The paired audit script SHA-256 was
`1193902530f8160c8ac43e1a62dac6929b194bbfef4c5237acf32036b97eb357`.

Both trusted DOM input traces delivered the exact requested integer points.
The measured X/Y projection scales were each exactly 1 and the world brush
radii were both `0.232804338527813`. Source and OBrush changed the same 26
original RGB vertex IDs. Maximum paired final XYZ component error was
`5.186961971048731e-13`, RMS `5.762112396395352e-15`, under the frozen
`1e-6` limit. Final active GPU display-normal words matched bit-for-bit;
the perpendicular held preview differed by at most `1.1641532182693481e-10`
in display-normal components. Each active GPU position stream matched its
public Save, with unique active-object identity. Public Before=Undo and
After=Redo matched in each app. There was no press dab; held directions gave
distinct alpha-oriented previews; OBrush reported held radii `[80,80,80]`;
the returned endpoint, release, and quiet frames were exact within each app.
Both owned browser processes and servers closed with exit 0 and no cleanup
errors.

The shared `obrush-core` route retains the press alpha origin and uses Source
Brush's live vertex/normal response while Lock is active, even when the visible
Accumulate setting is off. A held update replaces the previous preview against
the press baseline in one transaction; the host routes Source Draw Lock through
that core path rather than the unrelated legacy locked stamp. Focused tests
cover repeatability, zero motion, rollback with an existing Redo, no commit
after a rejected update, and a stale alpha lease. The workspace test suite,
strict native and WASM Clippy and fresh browser build passed for this candidate.

**Scope:** This passes only the integer-coordinate static single-object
Draw/Brush Lock fixture. C274 remains a failed normal-UI admission for
fractional coordinates: trusted DOM events differed by `3.0517578125e-5`
screen pixels in the horizontal/vertical held radii, above the same `1e-6`
gate. C275 found that this loss occurred before OBrush received the DOM
coordinate. Clay and other locked tools, alternate cameras/viewport sizes,
pressure, symmetry, continuous, dynamic topology, off-mesh gestures, and
high-poly held responsiveness remain unpaired or unmeasured. The wider
SculptGL Brush Lock parity row remains open, as does ZBrush parity.

The first C276 root run stopped before fixture import because a CDP window
bounds request contained a fractional width. Its raw result is retained at
`.artifacts/sculptgl-brush-lock-ui/root-eaa7382b-fc99-45cc-bcf8-82d7f202138f/result.json`
(SHA-256 `9eae35e77953b6d04f9c4c66aea0f4a7d055b81b8a2af8a811b344cb9b69643b`).
The setup helper was corrected to use the already verified 1024×768 normal
alignment and then add 40 integer window-height pixels. Both applications
were rerun with that same corrected harness SHA; fixture, controls, pointer
trace and acceptance tolerances were unchanged.
