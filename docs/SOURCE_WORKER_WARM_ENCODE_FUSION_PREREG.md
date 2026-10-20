# Source Worker warm-encode traversal attribution and conditional fusion

**Status: static design, no implementation or performance claim.** The
100k frozen finish-phase probe used product base `de7c6f6`, temporary probe
SHA-256 `ac554e2ddcd46fe77112b1f008c204e144f2e0e38cb60c04d0e71e48ecec5295`,
and raw result SHA-256
`a9e3558f6dfbf427e1ae590c9f749f37eecc8e7ce0fc40472ab2f36064bb460c`.
Its 1,289-dab Worker finish produced a 16,222,744-byte retained packet.
`encode_source_worker_retained_warm` took 164.2 ms; the whole finish reply
interval was 194.3 ms. This does **not** measure packet size as a transport
cost or isolate the serializer, digests and validation within that 164.2 ms.
The frozen verifier still failed RAF max, queue-by-100ms and the existing
saved-normal GPU oracle. No codec candidate is admitted from this aggregate.

## Static work inventory

The v1 encoder in `source_worker_retained_warm.rs` first computes
`seed_mesh_digest(mesh)`, which scans public positions/triangles/original
faces, then `state.seed_fingerprint()`, which walks exact face rows, first
triangle rows, position words, normal words and **every historical ordered-tree
word**. `state.encode_warm(mesh, &mut out)` then validates the public mesh,
compares its full topology and exact position words against the state, checks
the state allocation budget and emits exact warm payload words. The tree is
walked again in `OrderedTree::encode_warm`. `Writer::raw` checks the byte cap
and possible reserve on each primitive. Finally the header payload length is
patched and BLAKE3 checksums the complete output buffer. No full state clone
occurs in this path, but the tree is visited by both state fingerprint and
serialization, and its serialized bytes are visited by the packet checksum.

The mesh validation/binding, semantic state fingerprint and packet checksum
serve different checks. The first rejects invalid/stale public geometry; the
second confirms that a decoder rebuilt the same behavior-bearing producer;
the third authenticates the exact private packet bytes. A change may combine
work but must preserve all three results and the exact v1 wire bytes. The
fingerprint and wire encodings differ for parent/child presence and `usize`
width, so hashing serialized bytes as the state witness is not equivalent.

## Read-only preprofile gate

Before changing the encoder, instrument only this function and its private
helpers on the same frozen 100k Worker route to time: mesh digest; semantic
state fingerprint (including a separate ordered-tree digest timer); mesh
validation; state-to-mesh binding; budget check; scalar array serialization;
ordered-tree serialization; writer growth count/total moved capacity; and
final packet checksum. Record output lengths, probe patch and served build
hashes. Each timer is additive and must not alter packet bytes or publication.
Keep the 100k input, fixture, verifier and other browser gates unchanged.
The existing 164.2 ms aggregate is a baseline observation, not a target
subphase value.

Admit **only one** traversal-fusion trial if the semantic state fingerprint
takes at least 40 ms on the frozen warmed producer, the ordered-tree digest is
a substantial part of it, and an independent same-state native/WASM
microbenchmark predicts at least 40 ms net encoder reduction after adding
simultaneous hash updates to serialization. If these are not demonstrated,
stop at attribution; do not implement fusion or try compression on the
assumption that a 16 MB packet is inherently slow. A writer fast path or a
streamed packet checksum would require a separate preregistration and profile
because they address different work and allocation risks.

## Conditional, byte-identical candidate

Keep the public API, v1 header, payload, checksum and decoder unchanged.
Within the private encoder, initialize the semantic BLAKE3 domain and emit
the same generation/tag/center words. Hash the immutable face and first-triangle
rows, which are omitted from the warm packet, in their current semantic
order. Walk position and normal rows once, feeding both the unchanged wire
writer and semantic hash. Emit selected-chunk words to the wire only. Walk
the historical ordered tree once using a private dual sink: write every
existing little-endian wire word and separately update the semantic hash in
its existing order and width (including explicit option tags and u64 `usize`
words). Patch the state digest header slot only after all semantic words were
visited; patch payload length, then compute the unchanged packet BLAKE3 over
the complete final header and payload. Keep `seed_fingerprint()` as the
independent decoder oracle. No unsafe cast, cached pointer identity, tree
rebuild, approximate bounds or packet version change is allowed.

All existing fallible cap checks and `try_reserve` behavior stay before a
packet can be returned. If a dual-sink write or hash operation fails, the
partially constructed private buffer is discarded. The state remains behind
an immutable borrow throughout serialization, so a digest cannot be reused
for a different mesh/state lifetime. No state words, faces, selected chunks,
tree topology, free slots or history ownership may be dropped or normalized.

## Exactness, adversarial and performance gates

Freeze independently authored warm fixtures *before* code changes: a
triangle-only surface and an original-face mixed surface; a branched tree
with historical free slots; multiple dabs and one resumed cross-realm dab;
and signed-zero words in otherwise finite coordinates/normals. Store old
encoder packet bytes/SHA-256 and fixture/build hashes. Candidate packets must
match old packets **byte for byte**, including mesh/state digests, payload
length and checksum, in native and WASM. Decode both against the same exact
mesh and compare retained word fingerprint, internal ordered-cell/free/ring/
face-box words, next two dabs, public positions and normals bitwise, Save/Undo/Redo
bytes and history depth. No added peak allocation or cap regression is
acceptable.

Run independent malformed-packet tests through the unchanged decoder and
one-use host publication: wrong target, stale mesh/scene/revision, old epoch,
replay, truncated/oversize header or count, bad checksum, resealed mismatched
state digest, nonfinite normal/bounds, duplicate or missing faces, bad
parent/child cycle, free-list omission/duplicate, serial collision, wrong
rings/face boxes and a late rejection after substantial tree decode. For each
case assert unchanged document, current mesh and retained state, undo/redo
depth, and that a later valid dab or valid commit still behaves exactly.
Preserve every failing packet and test receipt.

If exactness and malformed-packet gates pass, measure the **same retained
state** with the old and candidate encoder in native and WASM, including
validation, buffer construction, digest and checksum. Require at least
40 ms net reduction at 100k without increased peak bytes. Only then run the
unchanged frozen 100k browser cohort with independent whole-file shape,
history, endpoint, GPU producer-to-VBO, queue and RAF gates. The frozen
saved-normal GPU oracle's known failure remains separately visible; it is
not silently waived. Any failed gate rejects the candidate and stops before
1m. An encoder win alone is not SculptGL or ZBrush parity.
