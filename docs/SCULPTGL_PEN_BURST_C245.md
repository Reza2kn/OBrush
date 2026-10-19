# C245: same-frame pen pressure burst in the ordinary browser brush

The ordinary browser path receives `PointerEvent.pressure` through the canvas
capture listener before eframe sees the pointer event. The app previously kept
one latest pressure and read one latest egui pointer position per frame. C231
and C232 exercised separate-frame synthetic pen samples, and C238 checked
admitted reference dabs; those results did not cover two held samples arriving
before the same animation frame.

The independent browser diagnostic runs with
`node scripts/check-brush-inputs.mjs WEB_DIST c245-pen-burst --burst-only`.
It opens an owned isolated Chrome instance, uses the visible factor controls
and public Save route on a fresh 40,962-vertex sphere, and compares two Draw
strokes. Both begin with a trusted pen down at pressure 0.25, then receive two
trusted held pen moves at `(600,343)` and `(650,343)` before the same observed
RAF count. The first case sends pressures 0.25 then 0.75; the control sends
0.70 then 0.75. Both use intensity factor 1 and radius factor 0. The source
comparison is limited to browser input batching; it does not assert that the
same stroke matches SculptGL brush geometry. The pinned reference pressure
factor behavior and separately admitted multi-dab comparison are in
[C238](SCULPTGL_PRESSURE_MULTIDAB_ADMISSION_C238.md), on pinned SculptGL
source `8e45dafc0f7906e5238dfffc1a2df742bd25f29d` and hosted bundle
SHA-256 `79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.

The **pre-change diagnostic** was authored from worktree base
`98813b6554c6e9c33144dccb45f540ad5302c347` and served the integrated
root browser WASM SHA-256
`aa4aff8a64c2ee36f88dd83f856657fc96efb82865f528a2123c5eaedac95d36`.
The served file had the same hash. The complete preserved result is
`.artifacts/brush-inputs-0.1.6/c243-pen-burst-trusted-8edfa178-cd0d-4992-b35b-83ad0ea22472/result.json`.
The pre-change control used 0.75 then 0.75. Both moves were trusted DOM pen
events with `buttons=1`; each pair shared RAF
count 286 or 285 respectively. Both strokes processed ten dabs after the
initial dab, and the final public mesh hash was identically
`2dc8cb2792e7898525cdf7770f4a65a58eaaa993af9f81ba50630d29cae0db0c`.
The first-dab GPU hash and initial public mesh hash matched across cases.
Each final GPU buffer matched its public save, and the owned browser exited.
This shows the earlier pressure sample was lost along with its position in
this same-frame ordinary Draw route; it does not by itself prove how an actual
tablet coalesces events. The diagnostic predeclared its meaningful equality
gate through exact mesh hashes and trusted same-frame event records; no
geometric tolerance was needed.

An earlier attempted diagnostic dispatched JavaScript-created `PointerEvent`s
in one task. Both reached the canvas with the requested pressure, but were
untrusted and produced zero further sculpt dabs (`1 -> 1`). Its failed run is
preserved at
`.artifacts/brush-inputs-0.1.6/c243-pen-burst-429c0199-8843-429a-9fbc-fe07e470d329/result.json`.
The trusted CDP result above, not this failed synthetic dispatch, is the
regression baseline.

The first post-change run with a 0.75/0.75 control was inconclusive: Chrome
delivered only the final control move, while both 0.25/0.75 moves were
delivered and the modified mesh had 1,201 changed original vertices with
exact one-stroke Undo/Redo. Its failed result is preserved in
`.artifacts/brush-inputs-0.1.6/c245-pen-burst-169eb084-6b9f-43d6-ad85-49daf587e986/result.json`.
The revised 0.70/0.75 control requires two delivered trusted events in the
same browser frame before comparing outputs.

Two further fixture attempts are retained because neither satisfied that
delivery requirement. Concurrent CDP dispatch coalesced the first event in
the low-to-high case at
`.artifacts/brush-inputs-0.1.6/c245-pen-burst-revised-0df099fd-fb13-46c4-ad79-c3b6c1c06f6c/result.json`.
A sequential dispatch with a 2 ms pause delivered both events but crossed an
animation frame at
`.artifacts/brush-inputs-0.1.6/c245-pen-burst-spaced-972853ac-f573-4d28-b460-ab05a4c08078/result.json`.
The final fixture holds its own app RAF callbacks during the two trusted CDP
events, verifies both DOM records at the same RAF count, then releases the
callbacks. This controls the precise batching condition under test; it does
not emulate an actual stylus driver.

The first **post-change pass** used served browser WASM SHA-256
`3062a3c0296d77a8c1edc654b3fc4b024d68ab515beed48bb86a19d5c93d36db`
and is preserved at
`.artifacts/brush-inputs-0.1.6/c245-pen-burst-held-raf-4ef5f2d4-ff89-4f65-81e3-27439b807184/result.json`.
It passed the same-frame geometry/history gate; its low-to-high case processed
`1 -> 6` dabs and its near-high case `1 -> 9`. A subsequent scope guard
excluded locked and local-transform strokes from replay, requiring a rebuilt
WASM artifact and a final check.

The **final post-change run** passed from isolated source base
`98813b6554c6e9c33144dccb45f540ad5302c347`. Its served browser WASM
SHA-256 was exactly
`fc465dceddd225efe173f1b442416591615eca3090c379fe0760ef43865c1ba7`
(23,797,885 bytes), and its diagnostic script SHA-256 was
`46a4e8ad6031d8b1896f0e0e4cfc0b676e7f664a362df52dd8c3211271d3a6c2`.
The preserved result is
`.artifacts/brush-inputs-0.1.6/c245-pen-burst-final-9f1f868f-70b6-465e-bc20-399203bf0f8e/result.json`.
Both pairs reached the DOM as trusted held pen moves at the same RAF count
(264 and 262 respectively). The canvas measured `1280 x 720` CSS pixels at
origin `(0,0)` with matching client dimensions; the egui press position
decoded as `(550,343)`, the DOM position. Both cases shared the exact initial
public mesh and first-dab GPU hash. The low-to-high case processed `1 -> 9`
dabs, changed 1,217 original vertices, and produced final mesh SHA-256
`9c392b3f257d8cdcb71ea4a90aa41fb9a29f44a5d42b04653dcc5086d1c93877`.
The near-high case also processed `1 -> 9` dabs, changed 1,211 original vertices,
and produced final mesh SHA-256
`51e0a18da1f555212db3b397664dee9123951699686917984900b0b96eed81f4`.
Both queued sample counts returned to zero with no drops. Both GPU readbacks
matched their public Save meshes, and one-stroke Undo/Redo restored the exact
initial/final saved mesh in the low-to-high case. The difference proves that
the first same-frame pressure and position now reach the ordinary Draw path;
it does not establish a SculptGL shape match or physical tablet behavior.

The browser change retains at most 256 ordered held pen positions and their
own pressure until the next app frame. On overflow it drops the oldest sample
and splits the stroke path rather than interpolating across missing events.
Replay is limited to an already active, queued ordinary brush at 100% UI zoom
and without mid-frame modifiers. Source geometry/Paint, continuous and locked
modes, Drag, different zoom levels, an initial press and moves batched before
the first frame, overflowing a real browser frame, transformed canvas layout,
and physical hardware remain separate gates. Move follows the ordinary queue
but has no dedicated paired fixture. The bounded queue may process up to 256
samples in one app frame, so this change is **not** a sustained frame-time
improvement claim. The shared brush geometry and native path are unchanged.
Native pressure remains conditional on `winit::event::Touch.force` reaching
egui; the current native route has no verified physical stylus fixture.

`cargo fmt --all`, `cargo test --workspace --locked`, strict native
`cargo clippy --workspace --all-targets --locked -- -D warnings`, strict WASM
`cargo clippy -p obrush-app --lib --target wasm32-unknown-unknown --locked -- -D warnings`,
`scripts/build-web.sh`, `node --check scripts/check-brush-inputs.mjs`, and the
isolated normal-browser run passed. The app tests cover queue bounds, FIFO,
cancel/clear, and ordinary/continuous/Drag admission. These are correctness
checks, not pressure hardware or full reference parity checks.
