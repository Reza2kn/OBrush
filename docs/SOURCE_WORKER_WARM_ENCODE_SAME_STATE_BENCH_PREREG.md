# Same-state admission benchmark for a fused Source warm encoder

**Design only; no fused encoder, Cargo run, browser run or product change.**
This adds a decision protocol to `SOURCE_WORKER_WARM_ENCODE_FUSION_PREREG.md`.
The C284 read-only profile of the frozen 100k Worker stroke (raw result
SHA-256 `53c4e1e2065633375843cce9966e5c83dc1e9cef561b10d275fcbc49bbb9cbc4`)
measured a 99.0 ms semantic state fingerprint, 55.9 ms of it in the historical
tree, against 21.6 ms of tree serialization, 32.4 ms mesh digest and 8.7 ms
packet checksum. The fingerprint's 40 ms preprofile threshold is met. Its
hash work is still required, so 99.0 ms cannot be counted as a prospective
speedup. No candidate is admitted without the same-state test below.

## Freeze one exact warmed state before trial code

Use the independently imported 99,602-vertex SP-WORKER-01 before-project
as the large public mesh source: the retained C284 `root-before.obrush` is
35,016,898 bytes, SHA-256
`1100e81f00726ea03ad699f16356b6bddea8861b795262f51b0b56b8af3c7e46`.
In a baseline-only native fixture maker,
load and validate that project, apply a fixed, explicitly recorded Source
contact sequence using shared core, then save both the resulting complete
public project and its v1 warm packet. The original browser result stores
packet lengths, not the packet bytes, so it cannot be reused as an exact
warm-state fixture. Freeze the generator source hash, core commit, input
project SHA-256, explicit tool/radius/strength/contact/pressure/sequence,
post-dab project SHA-256, packet SHA-256, counts and packet length **before**
writing the trial encoder. Keep the generated project/packet under the
256 MiB cap and retain them even if later tests fail. A native and a browser
WASM reader must decode this same packet against this same saved mesh and
obtain the same state fingerprint. Fixture preparation and decoding are
outside timed sections.

Use smaller, independently authored triangle-only and mixed original-face
fixtures for exactness and adversarial tests, including a branched tree,
free slots, signed-zero finite words and at least two future dabs. These do
not replace the 100k performance state. Do not substitute a fresh seed for a
warm producer: it loses the historical tree and selected-refresh words.

## Isolated A/B harness, not a product path

Keep a test-only copy of the pinned v1 encoder and a test-only dual-sink
prototype in the same native binary and the same browser Worker WASM module.
Neither function is callable from the ordinary studio or its held-input
protocol. Both receive the **same immutable decoded state, mesh and subtool
ID** on every pair; no setup, clone, decode, import, JS transfer or hash of
the result is inside either timed encode call. The prototype must still
validate mesh and state binding, compute the original semantic state digest
and exact v1 packet checksum, and use the same bounded fallible writer. Its
only proposed saving is a single state/tree source walk that performs
existing semantic hash updates and packet writes together. The reference
and prototype should be compiled with identical optimization/SIMD features;
record rustc, target, Cargo.lock, WASM/JS and both function-source hashes.

The static word map is a proof obligation for that dual sink:

| State component | Semantic fingerprint stream | v1 warm payload |
| --- | --- | --- |
| Generation, stored-face tag, center | u64, u8, three f32-bit u32 words | Same words and order |
| Canonical face rows and first-triangle rows | u64 counts plus face u32x4 and row u32 words | Omitted; hash before positions |
| Position, face-normal, vertex-normal rows | u64 counts and exact u32 bit words | u32 counts and the same bit words |
| Selected render chunks | Omitted | Eight u64 words |
| Tree generation/rebuild, cells | u64 counters and u64 cell count | u64 counters and u32 cell count |
| Live-cell tag, serial, depth, split/loose | u8 tag, u64 serial, u8 depth, twelve f64-bit u64 words | Same words and relative order; parent encoding lies between serial and depth |
| Tree parent/children/free handles | Presence tags plus u64 `usize` words | u32 parent sentinel, child tag plus u32 handles, u32 free count/handles |
| Tree bounds, faces, leaves, positions, rings | Exact f64/f32/u32 bit words with u64 counts | Same bit words with u32 counts |

The semantic stream starts with its existing domain separator; the wire
starts with the unchanged magic/version/target and independent mesh/state
digests. The final packet checksum covers the header **after** payload length
and state digest are patched, plus all payload bytes. Each optional parent,
child and free-list value retains its original representation in each stream.
Hashing the wire bytes as a substitute for the semantic stream would fail
this mapping. A test-only trace should compare the semantic digest before
and after fusion and compare the complete packet byte for byte; it must not
use the candidate encoder itself to generate its expected hash.

Before timing, compare every packet byte, header length, mesh digest, state
digest and checksum. Decode both through the unchanged v1 parser. Compare
private ordered tree/cell/free/ring/face-box words, selected chunks, exact
position/normal words, and two subsequent controlled dab outputs bitwise.
Compare public Save/Undo/Redo and history depth after a checked one-use host
commit. For malformed packets, reuse the full independent stale target,
scene/revision, replay, truncated/oversize, resealed digest, nonfinite,
tree-link/free-list/face/ring and late-failure corpus in the parent prereg;
every rejection must leave mesh, state and history unchanged. Byte identity
and adversarial results are hard admission gates, not just diagnostics.

For timing, run three untimed warmups per implementation, then nine paired
encodes in ABBA/BAAB counterbalanced order per realm. Time the full encoder
call with a monotonic timer, consume the returned length/bytes outside the
timed interval, and report every sample, pair delta, median and worst pair.
Run the native binary on the local native host and the WASM pair in one
owned browser Worker with the exact saved state, not separate page sessions.
Record visibility, browser version, memory pages/high-water and native
allocation/high-water diagnostics. Avoid GC/fixture reparse between paired
samples; if the browser becomes hidden or memory bounds are breached,
invalidate the pair and preserve the failure rather than replacing it.
The browser and native results are reported separately; native speed is not
used as a proxy for WASM.

## Admission and stop rule

The isolated prototype may advance to an ordinary-product candidate only
if packet/history/malformed tests all pass; peak allocation and caps are no
worse; and the **same-state WASM** median baseline-minus-prototype full
encode time is at least 40.0 ms, with at least eight of nine paired deltas
positive and the lower paired quartile at least 40.0 ms. Native results must
be correct and directionally consistent, but their absolute latency is not
the browser gate. A failed gate stops the fusion trial. Do not tune fixtures,
drop outliers, widen acceptance, replace failed packets or invoke the 1m
cohort. If admitted, a separately reviewed product candidate must still
pass the unchanged frozen 100k ordinary browser shape/history/GPU/queue/RAF
gates; the known saved-normal GPU oracle failure remains visible until an
independent producer-to-VBO display-normal oracle is qualified. Even a
40 ms isolated encoder win would not by itself fix the measured ~695 ms
release-to-commit or ~483 ms RAF maximum, and establishes no SculptGL or
ZBrush parity.

The C285 build/browser lane is owned elsewhere. Request exclusive Cargo and
browser ownership before fixture generation, native benchmark builds or
any WASM/browser A/B run. Preserve the exact test-only patch hash and all
failed receipts; remove all benchmark exports before any product promotion.
