# Initial Continuous consumer attempt in 0.1.19 candidate1

The frozen candidate1 produced useful ordinary and Continuous Draw results through normal browser controls. The whole run remains **failed** because its second-Undo contract incorrectly ignored the existing fixture-import history entry. Only Draw reached actual consumer execution; the other eight eligible tools, roster, interruptions and no-op cases remain unexecuted in this attempt.

The [receipt](evidence/sculptgl-continuous-initial-0.1.19.json) binds the exact immutable source/build/WASM/helper and preserves the raw result. This is a local workflow record. [FEATURE_PARITY.md](FEATURE_PARITY.md) and [the SculptGL inventory](SCULPTGL_PARITY.md) define the broader requirements.

| Observed Draw gate | Ordinary | Continuous |
| --- | --- | --- |
| Timer permissions consumed | 0 | 6 |
| Useful geometry response; selected channels and fully protected vertices | Passed | Passed |
| Original face/UV/material metadata unchanged | Passed | Passed |
| Released timer inactive, no queued timer tail or outstanding permit | Passed | Passed |
| Complete downloaded project matched actual visible GPU | Passed | Passed |
| Separate CPU completion, normal Save, GPU completion, then retained 0/200/800 ms quiet probes | Passed | Passed |
| Whole-stroke Undo and Redo | Exact | Exact |
| Original helper's second Undo restores fixture again | Failed | Failed |

Continuous output differs usefully from the ordinary stationary single press. The configured period is 16600 microseconds and pause-gap policy is 100000 microseconds; these are observed configuration values, not measured physical input latency or a paired source timing result. Permissions count attempted dabs. Normal Save/download scheduling is explicitly included in the completion contract.

The run retained 15 actual project downloads and 13 runtime project/GPU comparisons. Both normal Undo1 operations restored the fixture; Undo2 restored the original fresh default document. A new separately frozen helper captures that actual default document with normal Save before each import, then requires Undo1 fixture, Undo2 preimport, Redo1 fixture and Redo2 exact stroke. It preserves the original tile/settings/pointer path and every timer/channel/normal/material/protection/GPU/quiet requirement. The source and 16 oracle preparation checks are ready, but no passing runtime claim is made before the next immutable build.

The source3 raw failure, earlier preparation TDZ error and original source copies remain preserved. Its owned Chrome exited 0, its own localhost server closed, and no user tab or accessibility API was operated. ContinuousLocked, source brush response, physical pressure/focus/device loss, native GUI, restart preference persistence and performance remain separate gates.

After the release diagnostic showed an unrelated probe Save coalescing with bracket input, a separately frozen Continuous helper adds the same independent processed-key acknowledgment to its radius-command interruption only. It observes OpenBracket pressed after-shortcuts with the paired before row and no Save event, within 120 read-only RAF observations, without waiting on the expected radius or changing input. All nine ordinary/on trial and original history/channel/GPU/quiet requirements remain unchanged. This source-only derivative has 16 preparation-oracle checks; actual nine-tool execution remains pending its own quiet browser slot.

That subsequent slot completed the [separate candidate3 consumer cohort](SCULPTGL_CONTINUOUS_CONSUMER_0_1_19.md): all 16 cases, nine ordinary/Continuous pairs, roster, interruptions and no-ops passed, with 214 actual project downloads and 189 saved/GPU comparisons. A separately preregistered second Python reconstruction then passed all 189 pairs. These additive results leave the initial attempt failed and preserve its original numerical/history gates; no paired source or hardware-latency claim follows.
