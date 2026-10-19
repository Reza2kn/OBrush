# C231: normal-browser pressure-factor response

The C230 factor UI was exercised in an isolated Chrome browser using the normal
`web/dist` build, real egui clicks and trusted synthetic CDP pointer events.
The tested WASM SHA-256 was
`9742032c6abf9c56068093a08f7dd158e24292f2acbec33d8abe8ad9ea7e486e`;
the served file matched it. The complete four-case result, saved `.obrush`
files, screenshots, GPU readback evidence, and owned-browser cleanup record are
in [the preserved C231 factor artifact](../.artifacts/brush-inputs-0.1.6/c231-factors-final-7c92860c-db4a-432c-9d08-b901f145ba5c/result.json).

Each case starts from the same fresh 40,962-vertex sphere. The visible factor
controls were set and read back before contact. At pen pressure 0.25, factors
0/0 changed 427 original vertices. Radius factor 1 with intensity 0 changed
129 original vertices, all within that 427-vertex set. Intensity factor 1 with
radius 0 changed the same 427 vertices; their displacement was half the base
case with maximum absolute position-component error `7.0e-8` (gate `3e-7`).
Mouse input with both factors 1 produced exactly the same mesh hash as the
pen-pressure-0.25 case with both factors 0. Every case saved through the public
project route and matched the actual GPU buffer readback. The separate
[default-policy regression run](../.artifacts/brush-inputs-0.1.6/c231-pressure-regression-969aa2a0-2a53-496e-99df-1242ba1e79e9/result.json)
also passed its zero-pressure, mouse/full-pen, and quarter-strength gates.

The older nine-case alpha/pressure browser harness now starts each raster
import through the visible **Import alpha** button, then supplies the file to
that live request. Its [final C231 run](../.artifacts/brush-inputs-0.1.6/c231-full-inputs-final-d4d20b1a-4a76-4ef2-b2d9-f02f8a6e3084/result.json)
passes nine semantic comparisons across zero, quarter and full pen pressure;
neutral mouse; black, white and asymmetric PNGs; rotation; radius pressure;
and mirrored output. It also checks public project Save, exact mesh Undo/Redo
for asymmetric alpha, rejection of a 1025-pixel-wide alpha while retaining
the current sculpt, actual GPU buffer readback, and no runtime errors. The
quarter-pressure displacement has median ratio `0.2500009` and maximum
absolute component error `7.5e-8` against one quarter of the full-pressure
displacement. Earlier failed runs are retained under
`.artifacts/brush-inputs-0.1.6/`.

Final verification also passed `node --check scripts/check-brush-inputs.mjs`,
`cargo fmt --all -- --check`, `cargo test --workspace --locked`, strict native
workspace Clippy and strict WASM app Clippy. The browser artifacts above use
the same installed build throughout; this change only updates the verification
harness and documentation, so the application binaries were not rebuilt.

This confirms the visible browser controls affect one ordinary Draw dab as
intended. It does not verify a pressure ramp within a stroke, a physical pen,
the native GUI, other tools or meshes, a paired SculptGL UI response, or the
full SculptGL brush family. Both SculptGL pressure inventory IDs remain open.
