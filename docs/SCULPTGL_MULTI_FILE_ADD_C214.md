# C214: multi-file Source Add

The SculptGL-profile File Add picker now accepts several selected SGL, OBJ,
PLY and STL files in one invocation on browser and native hosts. Each file
receives a new revision-bound request, passes the existing bounded shared-Rust
import validation, and appends as its own Undo step. OBJ files may themselves
contain several `o` records; those parts remain one transaction for that file.
An invalid member is reported and later selected files are still attempted.
Earlier valid members remain in the scene, as in the pinned reference's
independent `FileReader` loop. The existing 128 MiB per-file input cap and
document/scene allocation caps continue to apply.

The ordinary browser used one real multiple-selection event with independent
OBJ (four vertices/four faces), PLY (three/one), and STL (three/one) fixtures.
The scene grew from six to nine parts. Three visible keyboard Undo actions
restored the exact pre-batch saved `.obrush` SHA-256
`73da411033eb00ebd2d78ff0fc378c88218b0d2c32f2e4c7b55f2b24271be63c`;
three Redos restored the exact nine-part SHA-256
`304a0f787882ff453692b183f81252866634df40622db16205d1aecabc5032e0`.
A second selection of valid PLY, malformed two-object OBJ and valid STL added
exactly two parts, displayed a file-2 rejection and required exactly two Undo
steps to restore the prior project. The pinned hosted SculptGL browser also
appended all three valid files from one selection, with the expected
four/three/three vertex counts and four/one/one face counts. Its asynchronous
per-file completion order is not guaranteed; OBrush currently processes the
picker order sequentially. Matched multi-file appearance, transformed geometry,
native physical picker interaction, large batches and long-session latency
remain open. This is a File Add workflow improvement, not complete SculptGL
or ZBrush parity.

The gated browser receipt is
`.artifacts/source-file-add-browser/eae7f450-0b51-49b3-bb4b-721b7c07f717/result.json`
(SHA-256 `e469485d6f1bb540585bcf311ff871a76120d7d701962a193146e72a4b5ba55d`).
Its runner is `scripts/check-sculptgl-source-file-add.mjs` (SHA-256
`b7d3f553894ea2fd233bd3924ce43a843c8cba1d3dd3123be76e903a8d4430b4`),
the final portable WASM SHA-256 is
`2b72ff69d6e5605c2f6c6d019ef905d073b91ea8a270c7bbdcc17af97217c73f`,
and the pinned hosted reference bundle SHA-256 is
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The first C214 browser run had a narrower reference count check and preceded
the malformed-middle-file continuation change. Its receipt remains at
`1225f60a-e028-4b2b-a708-f989d5f1245d`; the final gated receipt above is
the production-source result.

Final-source verification passes `cargo fmt --all -- --check`,
`git diff --check`, strict native and WASM Clippy, `scripts/build-web.sh`,
`scripts/package-macos.sh` and macOS `codesign --verify`. The complete
workspace run passed 2,519 tests with zero failures and one intentionally
ignored test across 83 result groups; `.artifacts/c214-tests-final.log` has
SHA-256 `c10e5d8855abacfd65500767af47764b5d4edb6585724449412d2e909697634e`.
The packaged native executable SHA-256 is
`20e78fc9e9ffa6a707b270ec46fa0659cdc6cc322fb4e86abf0608a21d1944c4`.
