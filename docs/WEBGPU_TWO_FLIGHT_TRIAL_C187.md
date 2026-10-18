# C187 two-dab WebGPU queue trial (rejected)

The opt-in resident Brush preview waits for GPU pick, deform and presentation
completion after each dab while browser contacts coalesce. C187 tested whether
one additional submitted dab could improve browser responsiveness without
changing the checked Rust document commit. The trial used the independently
authored 999,002-vertex SGL sphere and the existing ten-second pointer stream.
The browser driver copied `RUN_C181_MILLION_LONG_SIMD.mjs` with only the
`webgpu-two-flight=1` query option added (trial driver SHA-256
`21cf36991f2d6925907c369fa895be47eb3b6f7166e4ea18a7365941a588e9e4`).

The first browser two-stroke check matched all six saved viewport PNG hashes,
including Undo and Redo. The first long run then failed after two dabs:
calling the existing async `wait_submitted(&self)` and mutating the same
WASM-bound probe before its promise resolved triggered wasm-bindgen's
`recursive use of an object detected which would lead to unsafe aliasing in
rust` error. Cancellation was interrupted by that aliasing error. This is a
real trial failure, preserved in
`.artifacts/gpu-pointer-c146/c187-two-flight-million-long-r0/result.json`
(SHA-256 `4b5524661a52f9516fe19c83b0a0ecbcd7124f70f4ac44bdba6a4d361ca1bcf0`).

A synchronous WASM export returning a promise backed by an owned queue handle
removed the outstanding Rust borrow. With that correction, the long run
passed browser GPU validation and exact *internal* before/Undo and after/Redo
PNG recovery, with no exception. Its result is
`.artifacts/gpu-pointer-c146/c187-two-flight-million-long-owned-fence-r1/result.json`
(SHA-256 `5e6c9b6e75f165900459187b1efdd8ae283158a5572f2d6488fd33a26e38d2c4`).
The corrected trial sent 221 pointer moves, received 223 contacts, coalesced
only one and applied 222 dabs. C184's one-flight run received the same 223
contacts but coalesced 111 and applied 112 dabs. Thus the output shapes are
not matched, even though each run's own history is exact. The two-flight run
still measured 50.1 ms RAF P95, about 400 ms maximum RAF interval and
413.7 ms release, versus 50.1 ms, 400 ms and 414.7 ms in C184. The bounded
queue changed how many contacts became sculpt operations without improving
the observed frame or release gates.

The queue option and owned-promise API were removed. `scripts/build-web.sh`
passed after restoration; the final GPU WASM SHA-256 is
`7400c7230649d4c5c56b134d05ecad57ca4090017da88b1f1ae8cfeeb47c7a81`,
the C184/C186 build. The restored ordinary one-flight two-stroke browser run
passed and matched all six C186 before/preview/edit/second-edit/Undo/Redo PNG
hashes, with no browser exception (`c187-restored-one-flight-two-stroke-r0/result.json`,
SHA-256 `fd19d925f75e9c3011eebcdd601d9171805666eccffcfc6f065467a5cb6a101e`).
Formatting, six GPU library tests and strict native/WASM workspace Clippy
also passed; the complete workspace suite had passed against the same restored
source in C186. The normal browser studio still uses WASM CPU geometry and
WebGL2 drawing; this trial makes no SculptGL responsiveness or ZBrush parity
claim. A subsequent implementation must control dab cadence and
input-to-visible latency as well as GPU throughput.
