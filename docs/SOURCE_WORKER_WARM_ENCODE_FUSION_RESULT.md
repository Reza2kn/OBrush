# Warm encoder same-state A/B: rejected

The feature-gated dual-sink encoder produced a byte-identical v1 packet and
bit-identical geometry/state after two further dabs. The frozen admission
performance gates failed: the owned Chrome Worker WASM median saving was
**1.85 ms**, the lower quartile was **0.20 ms**, and only **7/9** pairs were
positive. The preregistered thresholds were at least 40 ms median and lower
quartile, with at least 8/9 positive pairs. No ordinary encoder or packet
change is promoted.

The fixture was made before trial source from the frozen 99,602-vertex,
199,200-triangle SP-WORKER-01 project. Its input SHA-256 was
`1100e81f00726ea03ad699f16356b6bddea8861b795262f51b0b56b8af3c7e46`.
The baseline-only generator as built had SHA-256
`f19befa70c50043452e04e2a657cd112927e6f38e31c8b7df636bdee5094e17d`
at core commit `7fc94aec2f0656ad1adf970528dfb6d0ed2c30a0`.
It loaded and validated the project, applied Inflate intensity 0.3 at face
98,999, center `[0.005235772579908412,0.004188680090010207,0.9999325474103292]`,
eye `[0,0,-1]`, radius 0.18, pressure 0.8, target ID 7. This changed 517
vertices and yielded generation 1. `fixture/post.obrush` is 21,470,127 bytes,
SHA-256 `424361411af2adb0858e4d2d5b0893b698508d2bf7e8ea866ab78578b8d5ae18`.
`fixture/warm-v1.bin` is 16,227,304 bytes, SHA-256
`530526c88b9ff5d3b5138651d807457bf39bd4f2a0f8e29b97853fa3c0ae9f53`.

Both paths received the same decoded immutable mesh/state. Full packet bytes,
header digests and checksum matched the baseline fixture; the unchanged v1
decoder accepted both. Two future Inflate dabs matched public mesh and
re-encoded retained state. Focused retained-warm tests passed 2/2; strict
feature-gated core Clippy passed. The trial did not exhaust the independent
malformed host/history corpus, so it would not qualify for promotion even if
the speed gate had passed.

The WASM test ran both functions inside one owned module Worker in visible
headless Chrome 155.0.0.0 on macOS. `browser-result.json` SHA-256 is
`fb357327174f4442ba15be030523b0f5941920764e31cd9facee6154563f2dd2`.
The `web-release` WASM SHA-256 is
`486f33576e9a37b49107b0324dce5501c374fe9d9c859d5fd3a927cf65aa548b`.
WASM linear memory grew from 15,335,424 to 366,215,168 bytes across fixture
decode, future-dab checks and all benchmark pairs; this is a final footprint,
not a measured per-pair peak allocation. Every pair is retained in raw JSON.
The nine WASM savings in milliseconds were
`[2.45,0.20,-0.25,2.80,-1.60,0.45,3.75,1.90,1.85]`.

The native `release` repeat in `native-result.txt` (SHA-256
`6263e0bdb8ec0cc351b543b98be75bab36e56662f74efaa8f8bc7ca3fbb7b34a`)
also saved only about 1 ms per pair. Its purpose was directional evidence;
it is not a substitute for the WASM result.

Trial source and the browser runner are retained under this ignored artifact.
`trial-sources.tar.gz` has ten source entries and SHA-256
`dcffa472e5d702a11312253b2f95597b99a126288e67d047d6d693a1b9915840`;
the tracked-only patch has SHA-256
`51d605ce837635b89a9257cc5d767e84e1eec69a9188fd03b869c4db26b0780c`.
The ordinary product source is restored after this report.
