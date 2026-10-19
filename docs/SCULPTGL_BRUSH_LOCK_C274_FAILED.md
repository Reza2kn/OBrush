# C274 Source Brush Lock: failed first paired admission

This is the unmodified first normal-UI acceptance result for the prospective
shared-core Source Brush Lock route. It used the frozen gate in
`SCULPTGL_BRUSH_LOCK_CORE_PREREG.md` (commit `f3c83a92e489d6018befe04626efd3298b4b9914`),
the pinned SculptGL commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`,
and the independently authored RGB-ID sphere and 16×16 hook alpha. Do not
replace this result with a later run or count C274 as a pass.

The Source result is `source-e416d486-8a6a-4956-89a7-567098b2e109/result.json`
(SHA-256 `88e238851520c70eaf9643de96c384e841c58e946d4a7431a88ebffe81549f56`),
the OBrush result is `root-df1883c1-e57b-461c-a424-7d4d3724cded/result.json`
(SHA-256 `bc1eeeeab8bc245c8ba784063c6302bdd774e864cb2899c78c90568d173f4aac`),
and the raw paired audit is
`source-e416d486-8a6a-4956-89a7-567098b2e109/paired-audit-root-df1883c1-e57b-461c-a424-7d4d3724cded.json`
(SHA-256 `9864018d5295eb33aebea718122956aecc61bea96e7de185c4380206bc5d042f`),
all beneath local ignored `.artifacts/sculptgl-brush-lock-ui/`. The fresh
Source capture was checked against the earlier frozen C273 Source public/GPU
oracle. Both runs used the same harness SHA-256
`867d2a03978b7b54175f8efc989e0a262d4585eb751468cc28a8e3baa05adc71`.
The fresh OBrush WASM and JS SHA-256 values were
`df38aa2544ce3cda77b56869af6ee51ceeee3fd2f6540a2dc21d52b31c2568c8`
and `03d74bea813d209f42f0aecbabcf7a4f85d39d509ed8b65f742def4ec3d4b440`.

The paired before geometry and faces matched. Source and OBrush changed the
same 26 RGB vertex IDs. Maximum paired after-position component difference was
`5.960464477539063e-8` (RMS `4.914135129433556e-10`), within the frozen
`1e-6` geometry tolerance. Public Before/Undo and After/Redo matched in each
application. Each active GPU position stream matched its own public Save and
the active OBrush object identity was unique. Paired active GPU display-normal
maximum after-component difference was `9.713403414934874e-10` (RMS
`1.7541379757053464e-11`). No press dab, distinct held orientations, exact
repeated endpoint, release/last-preview equality, and quiet frames passed.

**Admission failed:** the first and third OBrush held-preview radii were
`76.6925048828125` screen pixels, while the second was
`76.69247436523438`. Their difference is `3.0517578125e-5`, exceeding the
preregistered equal-radius limit of `1e-6`. The Source held moves used equal
80-pixel distances. The OBrush browser delivered screen coordinates through
`egui::Pos2` (`f32`); this is the observed precision boundary, not proof of
the complete cause. The frozen tolerance is unchanged. A subsequent f64
input-path attempt must have its own preregistration and separate receipts.

Focused core replacement/rollback tests and the host stale-alpha-lease test
passed after correcting the new tests' assumptions. The full workspace test
suite, strict native and WASM Clippy, and `scripts/build-web.sh` passed before
the paired run. Failed setup checks are retained in the local task trace:
shared target cache initially served a stale core artifact until candidate
source mtimes were refreshed; a test initially treated `can_undo()` as history
depth during an open preview; a host test initially inherited the default
Clay option; strict Clippy found two candidate lint errors; the first audit
invocation omitted `OBRUSH_SHARED_ROOT` and could not load the shared fixture
decoder. These setup corrections did not change the frozen pair inputs or
acceptance tolerances. No C274 product code was promoted on this failed gate.
