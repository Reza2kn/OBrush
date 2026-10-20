# Q03 Move close-up: failed frozen camera gate

Q03 ran the committed [frozen protocol](NATIVE_MOVE_CLOSEUP_Q03_PREREG.md)
and [harness](../scripts/check-native-move-q03-closeup.mjs) in owned Chrome
155.0.8059.85 at 1600 × 1000 CSS pixels and DPR 1. The official local and
served WASM both hashed
`f3f718d0fac0972ed90bf63cbf06e11b4a489ec0b1fee7a00a00fe9147dfc58a`.
The run exited 1, and its exact result is retained at
`.artifacts/native-move-q03/b2a229b6-db5c-41ee-a40f-cf81c2e162cf/result.json`
(SHA-256 `15a329d4062fab575b8864287258b42d40f3dd9f9a91df25657ad295d4dac67a`).

The public Open and Save of the frozen Q02 initial project succeeded: saved
project SHA-256 matched the input
`2b88d59fc06ab6eed9ef25b51de267ff3791b00438e84a7d3247e1f651371fbb`,
and the mesh stayed byte-identical after navigation. The initial project
actually carries a camera distance of `3.27466154` rather than the assumed
default `4`. The specified middle drag changed the camera target from
`[0,0,0]` to `[-0.69212872,0,0.25264674]`. Five wheel steps then gave
distances `2.55030894`, `1.98618257`, `1.54684055`, `1.20468068`, and
`0.93820626`; the last is outside the preregistered `[1.0,1.6]` gate.
The harness stopped before loading the final state. The before screenshot
SHA-256 is
`957ea99e20c0bb038e6b11af6b804e70367067dc04a589db7f3e63c1c2d33821`.
At original pixels it shows a blank smooth part of the sphere because the
navigation missed the Q02 edited area. No paired pixel or tearing verdict
is possible from Q03. This remains a failed run; Q02's narrower numerical
result and the user's visual complaint remain open.
