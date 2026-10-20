# MRP07 sparse press-normal feasibility result

**Decision: scoped feasibility is positive; product admission remains open.**
The read-only sparse calculation reproduced the full picked report and alpha
track words on the retained fixture checks and the 32-contact public-project
oracle. Five native and five normal owned-Chrome WASM pairs exceeded the frozen
50-ms press-time threshold with no additional WASM linear-memory pages. The
full first-dab seed remains necessary and was slower following the sparse
press in this browser cohort. Malformed face-normal overflow and every
allocation/tree rejection class have not been independently paired. The
MRP07 preregistration therefore does **not** admit a product change or a
million-vertex responsiveness/parity claim. MRP05 and MRP06 remain failed.

The frozen protocol is
[`SCULPTGL_MILLION_SPARSE_PRESS_MRP07_PREREG.md`](SCULPTGL_MILLION_SPARSE_PRESS_MRP07_PREREG.md)
(SHA-256 `d32df9a9bd789498bb4eff503d584c3dc1ae8b416c57b9c399a9288536431501`).
Both runtimes loaded the exact 351-MB public `root-before.obrush` with
999,002 vertices and 1,998,000 triangles (SHA-256
`367d4327bd14f2d3a67db5129a901226c37091926184e9f6c603c7126f838c36`).
The diagnostic is feature gated and leaves the ordinary press path unchanged.
Its read-only source remains in isolated commit `5c9417d`; the primary
checkout has not merged the candidate or switched its press path.

The 32 distinct face rows are retained verbatim in
`.artifacts/mrp04-stage/mrp05-native/mrp07-oracle-distinct.log` (SHA-256
`1d7ed44aa4f1a5efee7ef21acdfb5a39d2b640dafa72cba221207e1fa2c8405e`).
Complete picked-disposition serialization, each alpha-track bit and the
mirror pair matched the full state. Separate triangle, quad, 16-face
extraordinary shared-corner, exact-corner, masked, mirror, absent-contact,
invalid-contact, NaN position, mask-convention and same-size position/topology
tests passed. The initial targeted run **failed** on an invalid picked face:
full returned `RejectedPickedNormal(MissingOriginalFace)` while the sparse
diagnostic returned `MissingOriginalFace`. The diagnostic was corrected to
follow the picked-normal API's rejection wrapper and the three targeted
tests then passed. This failed check is retained here rather than discarded.

| Paired fresh process | Native full press (ms) | Native sparse press (ms) | Owned Chrome full press (ms) | Owned Chrome sparse press (ms) |
| --- | ---: | ---: | ---: | ---: |
| 1 | 220.42 | 35.77 | 334.7 | 72.5 |
| 2 | 208.89 | 36.61 | 371.3 | 74.3 |
| 3 | 209.80 | 35.87 | 374.1 | 105.6 |
| 4 | 208.55 | 35.71 | 457.0 | 93.6 |
| 5 | 209.00 | 35.73 | 420.7 | 77.8 |

Paired median press savings: **173.27 ms native**, **297.0 ms WASM**.
Native full data:
`.artifacts/mrp04-stage/mrp05-native/mrp07-native/runs.json` SHA-256
`ae6381d01bb3dcf76552ba701003dddcaf9e5c9ffa5701026d3689c5379203ef`;
driver SHA-256 `6af33f9ed7aeb2dcef31ef8916cf808dcf0bb5171751541b3c52e142992ba2cb`.
WASM full data:
`.artifacts/mrp04-stage/mrp07-wasm/runs.json` SHA-256
`1b067608278351b5597569bc99e1388d4cd05f8dd528a1f72322dd4ac8a9c81f`;
owned-Chrome driver SHA-256
`5febe89991231ab8bf933bf2e8d59258f2c273a807ff13144d63f981b59bd6c9`.
The independently rerun audit checked each imported topology, exact bound
artifact, pair order, alpha-track words, sampled memory, absent errors and
owned Chrome exit/closure: `.artifacts/mrp04-stage/mrp07-wasm/independent-audit.json`
SHA-256 `f9e12a9a8c51e31b992abdc1ed1f61d9f081ef71e9c67f699b143141574d991b`;
auditor SHA-256 `b3f2ac845af68fb5096040d7486f97ff4ffa96eba195dc883a5beba7e51db76d`.

All ten WASM measurements used **1,502,150,656 bytes** of linear memory
before press, after press and after first-dab seed. Sampled Chrome-tree RSS
sparse-minus-full varied from −190.4 MB to +3.3 MB across pairs; this sampled
process metric is not a deterministic peak-memory proof. The WASM build SHA-256
is `2b0a4ecd4d293419628691030f6132da6d154154c41772e0e5d6d02eb78538be`,
JS glue `c057f8cb666f114fcafe8a0e629473321061f403f301a6019cd661cafdd82eec`,
and index `daeee0a7099f1cbefbefcdd69abbbcd5aaf7c1b79a036d5bd769042f7ce2a830`.
The unchanged full first-dab seed took 219–323 ms in full-press runs and
345–476 ms in sparse-press runs. Combined press plus first-seed paired savings
had median 197 ms, so a whole-frame 60-second responsiveness gate is still
required and the prior four-threshold failure is not overwritten.

Validation: `cargo fmt --all -- --check`, core feature tests (including the
three targeted fixtures), strict WASM app Clippy and the official
`scripts/build-web.sh` succeeded. The first official build invocation failed
because this isolated worktree did not have a local 0.2.129 wasm-bindgen CLI;
the retry used the lockfile-matched 0.2.129 CLI from the primary checkout,
then a custom feature package was built for the read-only browser probe.
No product brush, document, render or history code was changed.
After the final diagnostic adjustment, full `cargo test --workspace` and
`cargo clippy --workspace --all-targets -- -D warnings` also exited 0.
Their retained logs are respectively SHA-256
`f54740171370d501177236b53eb12434b0d1e44c75f0577a227bb7050e77805f`
and `b8d9ad601a291ad5225798083601802851f2d8756d8baeae69829a3ed56df9bd`
under `.artifacts/mrp04-stage/mrp07-wasm/`. The test log includes macOS
incremental-cache cleanup warnings about already absent object files; no test
failed. Native Clippy reported only the existing future-compatibility note
for the `block` dependency.
