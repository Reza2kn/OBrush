# C282 interior-face Brush Culling comparison: scoped geometry match, frozen gate failed

The one preregistered C282 normal-UI four-document run used Source commit
`8e45dafc0f7906e5238dfffc1a2df742bd25f29d`, hosted JS SHA-256
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`,
fixture SHA-256
`80b6430b152aad76fface01e9bf4890fdd525b2d9f11c60cd74e6c1b021d876b`,
harness SHA-256 `38c9f3a391d45b31e27092c5469794a07e4c0bf18f87346295cd83f0e893fa71`
and audit SHA-256 `d88dd0928a3a768f9e09f05ea6fc78daabce950ab783063fd23b3d4ee15ef8ab`.
OBRush WASM/JS bytes remained exactly the C280/C281 baseline:
`913ed5f6ac0fd3f3e0e55788d40d7421d844820bf92f664a4c8a6b7858bd8f99`
and `03d74bea813d209f42f0aecbabcf7a4f85d39d509ed8b65f742def4ec3d4b440`.
No product code changed.

Raw records:

- Source on: `.artifacts/sculptgl-brush-culling-c282-ui/source-on-b496a32d-a6ff-4ef6-897d-80ab9c4b26a7/result.json`
- OBrush on: `.artifacts/sculptgl-brush-culling-c282-ui/root-on-8694d730-ed37-4730-ba61-b4954d2b8a5e/result.json`
- Source off: `.artifacts/sculptgl-brush-culling-c282-ui/source-off-de39b45a-cdb0-4a34-a8ca-4e7d331fa7f7/result.json`
- OBrush off: `.artifacts/sculptgl-brush-culling-c282-ui/root-off-450babbf-7082-4386-bed5-8dd0a3dfaf80/result.json`
- Frozen audit: `.artifacts/sculptgl-brush-culling-c282-ui/source-on-b496a32d-a6ff-4ef6-897d-80ab9c4b26a7/paired-audit-root-on-8694d730-ed37-4730-ba61-b4954d2b8a5e-with-off.json`

All four published projection-derived path points met the frozen interior
face bound; the minimum barycentric coordinate was `0.0696498` and minimum
XY edge distance `0.001231246`, versus required `0.05` and `0.001`.
The maximum on/off paired world-ray X difference was `1.24472e-7`, Y
`1.74323e-8`, and radius difference `1.42254e-7`. Source directly reported
RGB picked face `[221,222,243]` at press and both held positions. OBrush
reported that same face at the second held position, its sole admitted dab.

The scoped geometry comparison was close: maximum paired public XYZ
component difference `1.1175870895385742e-8` with culling on and the
same maximum with culling off. The per-state changed RGB-ID sets matched.
Culling on left the back sheet exact; culling off moved front and back.
The frozen public history, active GPU position and display-normal, object
identity, topology, attribute, post-release quiet-frame and numeric gates
also passed, as recorded in the audit.

**C282 overall failed its frozen observation gate.** Its preregistration
required both Source and OBrush to publish the picked face at press and both
held stages. OBrush's normal UI admitted no geometry dab at press or first
held (`dab_observations.total_calls=0`), so its `controlled` pick report is
`null` at those stages. The audit retained four failures (root on/off ×
press/first held). This is missing observation at no-dab stages, not a
wrong-face observation. The successful numerical subchecks do not turn
C282 into a passed paired fixture. C279 setup, C280 numerical and C281
hypothesis failures remain intact; full Brush Culling parity remains open.
