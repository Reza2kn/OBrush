# Q04 Move close-up: failed frozen hover setup

Q04 ran the [frozen protocol](NATIVE_MOVE_CLOSEUP_Q04_PREREG.md) in an
owned Chrome 155.0.8059.85 at 1600 × 1000 CSS pixels, DPR 1. The local
and served official WASM SHA-256 matched at
`f3f718d0fac0972ed90bf63cbf06e11b4a489ec0b1fee7a00a00fe9147dfc58a`.
The retained result is
`.artifacts/native-move-q04/15ffb486-3d09-411f-96a0-8dc06f9dc0d5/result.json`
(SHA-256 `051c6a46595919f20173e0263152611e8b0fa10c416d88e3b66274d6c010b37f`).
The run exited 1, and owned Chrome exited 0.

The Q02 initial project loaded through the public file input and saved with
exact input SHA-256
`2b88d59fc06ab6eed9ef25b51de267ff3791b00438e84a7d3247e1f651371fbb`.
The mesh remained unchanged and camera finite. But all three wheel events
left the camera distance at `3.27466154`, outside the frozen `[1.4,1.7]`
gate, so the harness stopped before loading the final state. The screenshot
SHA-256 is
`25114d80b2663b8d52620a7b31cada84d56575609e383acc2d9d57ba4a05ddf4`.
Q03's wheel changed distance only after pointer movement and a middle drag;
Q04 omitted both. This supports a viewport-hover setup issue, but does not
prove which input event the host requires. Q04 provides no paired pixel or
tearing verdict and remains a failed run. Q05 will test the narrower ordinary
pointer-move setup under a separately frozen protocol.
