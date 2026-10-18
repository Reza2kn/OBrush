# C183: versioned BLAKE3 resident upload and lightweight commit capture

The browser's opt-in WebGPU Brush preview previously spent 3.6 seconds
starting on the `999,002`-vertex fixture. The GPU handoff encoded a bounded
mesh upload with a whole-buffer SHA-256 checksum, then separately built a
CPU reference normal/topology snapshot even though the browser bridge needed
only a checked identity for its later sparse commit. The upload now writes
version 2 with a BLAKE3 checksum, and the browser bridge stores a lightweight
`ResidentDocumentCapture` containing the exact mesh fingerprint, active
SubTool ID, pose and mask convention. It retains the same stale-mesh, pose,
selection, redo, fold and undo checks. Full `ResidentDocumentInput` remains
available for native/scalar reference work. The GPU decoder still accepts
version 1 SHA-256 uploads and rejects a corrupted version 1 or 2 checksum.
The transient upload is not a public project format; `.obrush` and OBJ are
unchanged.

The comparison used the independently generated SGL sphere (SHA-256
`dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`)
and two consecutive three-dab Draw mouse strokes. Captured settings were
radius `0.1800000072`, strength `0.3499999940`, X symmetry and accumulation
on, hardness zero, world-space radius, and viewport `[225,57,1365,882]`.
The first and second strokes changed 5,171 and 5,654 vertices respectively.
Before, first-after, second-after, visible Undo and visible Redo PNG hashes
were identical across the original SHA-256/full-capture build, the final
SIMD build and the forced-portable WASM fallback.

| Local browser run | First preview start | Second preview start | First capture / GPU upload | Second capture / GPU upload |
|---|---:|---:|---:|---:|
| Original SHA-256 + full CPU capture | 3,595 ms | 858 ms | 1,784.9 / 1,802.6 ms | 456.3 / 401.3 ms |
| Lightweight capture, SHA-256 upload | 3,525.9 ms | 799 ms | 1,709.2 / 1,807.8 ms | 400.1 / 398.2 ms |
| Lightweight capture, BLAKE3 upload, SIMD app | 590 ms | 441.8 ms | 219.2 / 363.2 ms | 189.8 / 251.4 ms |
| Lightweight capture, BLAKE3 upload, portable app | 714.7 ms | 565.4 ms | 336.8 / 370.5 ms | 309.3 / 255.4 ms |

These are individual local wall-time observations, not controlled paired
distributions. The first-row and final-row complete viewport hashes are
byte-identical. The full-capture and lightweight-capture core test also
commits the same sparse geometry and recovers the same saved document through
Undo/Redo; a changed color channel rejects before mutation as stale. The
normal studio still uses WASM geometry and WebGL2; this is a separate WebGPU
preview. The 10-second million-vertex RAF and release gates remain failed in
the preceding C182 run, so this startup improvement does not establish
SculptGL responsiveness or parity.

Browser evidence: original
`.artifacts/gpu-pointer-c146/c183-two-start-stages-r0/result.json`
(SHA-256 `d33e667ecbb5c5ad8a197baf03268db261a2d7149c2f01cce8be88445f363d67`),
lightweight-only
`.artifacts/gpu-pointer-c146/c183-two-start-light-capture-r0/result.json`
(`6842c36c27cb1502e52ab11c4ace489da833b34e352e0a3153c4f0d1ff7a1d6a`),
final SIMD `.artifacts/gpu-pointer-c146/c183-two-start-blake3-r0/result.json`
(`65a19133c9c9fe282a5a5bfa7903061e5549fedc4b496a9436b409a0c0b7de92`),
and final portable
`.artifacts/gpu-pointer-c146/c183-two-start-blake3-scalar-r0/result.json`
(`d9ed477491afdcd4ad85a1af22601fac0dc32771241d515d45311d3ab7745de0`).
The final portable app, SIMD app and GPU WASM SHA-256 hashes are respectively
`453455d1274b1184f4eaf00d48a8ffb0e73d4788861763477703718dfdbf6158`,
`d3fb0ea3915b68986bfcb26833b1acc0ec4b5ebf182d0b55a86b66a23e5542d6`
and `49332db376be57d2cb807b55dd2bf4cfbd1f95905131a71624721d6ff8eb081e`.

The final 10-second SIMD browser run started the preview in 599.9 ms, then
sent 223 pointer moves, received 225 contacts, coalesced 112 and applied
113 GPU dabs. It changed 25,502 vertices; event timing differs from C182,
so this is not a paired geometry comparison. RAF P95 was 50.1 ms, maximum
416.6 ms, and release 437.3 ms, including a 344.7 ms checked commit. Its
visible Undo/Redo hashes match the run's before/after states. The same three
responsiveness gates still **fail**. The result is
`.artifacts/gpu-pointer-c146/c183-simd-million-long-r0/result.json`
(SHA-256 `7dd5ae512b2847254eb310683fb7ac00f3796034bfbb4f12fb8bdea7cbc4095f`).

Final checks passed: `cargo fmt --all -- --check`, 2,496 workspace tests
across 103 groups (zero failed, one ignored), strict native and WASM-target
workspace Clippy for portable and SIMD configurations,
`scripts/build-web.sh`, `scripts/package-macos.sh` with a valid local app
signature, `node --check web/resident-pointer.js`, and `git diff --check`.
The browser checks used the rebuilt packages and closed their owned browser
processes cleanly. These checks do not establish SculptGL or ZBrush parity.
