# Sparkdancer Draw recovery — 2026-10-21

The imported Sparkdancer model now passes repeated Draw buildup and two leg
checks in the ordinary browser app. Browser frame pacing remains below the
30 fps target. This is a scoped improvement, not a SculptGL or ZBrush parity
claim or a promise of unlimited buildup on every mesh.

## What changed

- Refine the affected patch and preserve curvature rather than subdividing the
  entire imported model. A conservative local window runs the same geometry
  kernel as the full mesh, with a full-path fallback when coverage is uncertain.
- Keep fragile existing triangles coherent during Draw. When uniform step
  scaling stalls the picked surface, a bounded local solve can recover a useful
  step. Original face-quality, fold, mask, support and movement limits still
  decide whether the rounded result is accepted.
- Calculate canonical ray intersections in f64. A recorded f32 edge rejection
  had selected the back of the model despite a valid front surface. Independent
  front/rear fixtures now cover that case.
- Admit contact anchors at triangle edges and handle a one-ULP movement-cap
  rounding case without increasing the allowed movement.

The importer retained 369,407 vertices and 738,900 triangles. It skipped four
facets with exactly repeated welded positions. Existing detail and sharp edges
in the STL are preserved; this work does not repair arbitrary old mesh damage.

## Normal browser results

Frozen geometry build: `71aa64020dc4477cedb91a21eb7044bd0e00c7c8`, version
0.1.21, clean source tree. Portable WASM SHA256:
`87b52cb9341307d22e7143314a95bd8523e2ab039312d3e33d8debe9f27e27dc`.
SIMD package SHA256:
`81a62ff1c6e9f096459738a73d7208d291ca11d6d211295c121c4aa8d5d26171`.
These tests used the normal file picker and mouse/keyboard input in disposable
Chrome sessions, and inspected public saved projects rather than mutating app
state through test setters.

| Case | Settings | Result |
| --- | --- | --- |
| Same-point buildup | Radius 0.180, strength 35%, X symmetry, 256 contacts | 256 document edits; picked normal growth 0.580665; final eight ray advances total 0.00387652, above the frozen 0.0018 minimum |
| Upper leg | Radius 1.0, strength 35%, symmetry off, 12 contacts | 12 useful contacts; picked normal growth 0.0446836 |
| Lower leg | Radius 1.0, strength 35%, symmetry off, 12 contacts | 12 useful contacts; picked normal growth 0.0296506 |
| Lower leg close-up | Radius 0.180, strength 35%, X symmetry, eight additional zoom steps, 12 contacts | 12 useful contacts; picked normal growth 0.0530781; projected brush radius about 23 CSS pixels |

Every case passed exact serialized Undo/Redo and reopen/save comparisons,
release-idle checks, and the app's GPU-error check. The 256-contact case grew to
375,639 vertices, adding 6,232 locally. Contact count is a finite endurance
witness; it does not establish indefinite growth.

Retained browser receipt IDs: `draw-b135dcaa-5a3e-4bd8-846b-6b167a14f091`,
`draw-f223be1b-6014-4daa-937b-7ba1fe45def7`,
`draw-4ccabb90-4c62-4ed1-b318-3f72f361d70b`, and
`draw-a9f67119-58bc-4c53-8312-babdb98ea69e`. Private screenshots and saved
models remain local and are not distributed with the repository.

## Exactness and regression checks

Separate full-mesh versus local-window traces compared every position, normal,
triangle, mask, color, material and BrushResult after each contact. Both
256-contact symmetry settings passed, including 12 near-edge picks per lane.
Eight-contact runs with nondefault masks, colors and materials passed too.
The geometry build passed 2,719 workspace tests with zero failures and 16
ignored private/experimental tests, strict native Clippy and WASM checks.

The private source STL is not included. Its SHA256 is
`5502a1c2ce42adbf70729cb1b00a8110eb8c5ace5d1f6f332c2ad2e1e7b9ef9c`.
Owners of the same file can run the ignored core traces with
`OBRUSH_SPARK_STL` pointing to their local copy:

```sh
cargo test --locked --release -p obrush-core --lib private_spark_ -- --ignored --test-threads=1
cargo test --locked --release -p obrush-core --test spark_import_draw -- --ignored --test-threads=1
```

Full/window trace log SHA256:
`278e96ff19d4e68acbf4a4888518abbd69ef7867c63bae40a7ad6985d82fac90`.
Attribute trace log SHA256:
`5511213e7b52497d9e31ed97836abedeeb64e9c6d0bff73deb049c24ad375f9f`.
Workspace log SHA256:
`1cb48a5b3da6d0b736bcf6604e9e6c37d6e60bd4f770eb9e60c27bf80d4cfe5e`.

## Remaining failures

The frozen ten-second held-stroke browser gate still fails its 33.6 ms P95
frame-time limit: the geometry build measured 50.1 ms, with a 52.14 ms release
acknowledgment, no pending dabs and no edits during the following 500 ms.
The real close-up held-stroke run also measured 50.0 ms. An earlier zoom setup
crossed the front surface and made no edits; it remains a failed setup, not a
performance pass.

A separate fresh-sphere 512-contact endurance test still stalls. Its saved
contact-302 witness shows a guard-safe step exists, so that failure remains an
optimizer convergence issue. Neither the Sparkdancer passes nor a successful
build closes that broader Draw issue.
