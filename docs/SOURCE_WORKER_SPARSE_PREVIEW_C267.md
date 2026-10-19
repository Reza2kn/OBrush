# C267 preregistration: checked sparse display preview packet

This change is a transport primitive, not an activated ordinary browser brush
or a SculptGL acceptance result. It is intended to remove the per-dab public
project serialization, complete mesh clone, and complete mesh digest from the
main-thread preview consumer. It does not make the Worker producer's affected
row selection certified or remove any other full-scene work by itself.

The independently authored fixture has three vertices with baseline positions
`(0,0,0)`, `(1,0,0)`, `(0,1,0)` and normals `(0,0,1)`. Epoch 9, active part 7,
and sequences 1 then 2 change rows 0 and 1, then return row 0 to baseline.
The expected overlay counts are 2 then 1. Exact `f32::to_bits` equality is the
oracle, including rejection of a `-0.0` predecessor against `+0.0` baseline.
No reference ZBrush/SculptGL fixture is involved because the packet is private
implementation machinery.

The packet is little-endian V1: eight-byte `OBRSPRV1` magic, version, epoch,
sequence, active ID, vertex count, row count, ascending unique rows containing
the index and six old plus six new `f32` words, then a BLAKE3 checksum. The
packet is capped at 64 MiB and the count at `MAX_VERTICES`; the overlay stores
only changed rows and explicitly caps persistent entries at `MAX_VERTICES`
(one million), with fallible allocation. The worst-case logical payload is
28 MiB for the row keys and words, plus implementation-dependent hash-table
buckets; a host should discard the overlay after the stroke. A mismatch
rejects the whole packet and cancels the overlay.
The returned rows are suitable for a bounded renderer patch; no document or
history is modified. The final publication remains the existing complete V1
checked delta and retained-state path with its single Undo transaction.

Required follow-up gates before this can be called a performance improvement:
focused and workspace tests, strict native/WASM Clippy, browser build, a normal
browser Worker and GPU readback on the million-vertex fixed fixture, exact
public Save/Undo/Redo, affected-face shape checks and four responsiveness
thresholds. A Worker producer must supply an independently certified complete
affected-row set; this codec deliberately does not infer one by scanning the
whole mesh.

Isolated core verification on 2026-10-19: source at SHA-256
`a1eacb8508df51ce5e32e081e49cd7cce534977f64aa7185016fa2916a357371`;
`cargo fmt --all -- --check` passed; four focused preview unit tests passed
(0 failed); `cargo clippy -p obrush-core --all-targets -- -D warnings` and
`cargo clippy -p obrush-core --target wasm32-unknown-unknown -- -D warnings`
both passed. No full workspace test, browser build, GPU check or comparative
performance measurement was run on this isolated codec branch. The main
remaining risks are stable-baseline host ownership, certified complete row
enumeration by the producer, and restoring the renderer after a rejected
preview packet.
