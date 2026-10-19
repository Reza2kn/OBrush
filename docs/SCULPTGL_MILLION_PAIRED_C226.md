# C226: million-vertex OBrush versus pinned SculptGL browser cohort

The same frozen 999,002-vertex / 1,998,000-triangle SGL sphere (SHA-256
`dfb5316f62e392e633f03cb662dcb12215ed165a71a9ae72a3fd639528731afe`)
and 60-second, 400-requested-moves/s protocol (SHA-256
`234870ed03a240931a22f421968917efaeed8becc1df834c998bbe3e9c101a33`)
were run in separate owned Chrome processes against OBrush C224 and pinned
SculptGL source commit `8e45dafc0f7906e5238dfffc1a2df742bd25f29d`.
Both runs completed Save, Undo and Redo, closed their owned browser cleanly,
and had no application/console or recorder errors. Different delivered
contacts and dabs prevent a matched-stroke speed or sculpt-response claim.

| Observed active recording | OBrush | SculptGL |
| --- | ---: | ---: |
| RAF interval P95 | 83.3 ms | 16.8 ms |
| RAF interval maximum | 600.0 ms | 33.4 ms |
| App RAF callback CPU P95 | 71.2 ms | 0.2 ms |
| Mouse-release acknowledgment | 161.2 ms | 8.0 ms |
| Applied OBrush dabs / recorded SculptGL brush callbacks | 886 | 1,139 |
| Preregistered active-recorder gates passed | 0/4 | 4/4 |

The callback CPU numbers are recorder observations of each application and
do not have identical internal work scopes. The RAF and release observations
are the directly relevant user-facing responsiveness evidence. The C225
root-only report separately gives the OBrush queued geometry, renderer
admission and upload stage timings. SculptGL's recorded brush callback mean
was 1.638 ms; that callback is not the same boundary as OBrush's 19.797 ms
queued Source-geometry timer. No cross-application per-dab ratio is inferred.

Independent public-file audits pass. The OBrush `.obrush` before/after files
have exact Save/Undo/Redo and 40,804 changed vertices / 82,431 affected
triangles. A separate bounded version-3 SGL raw-word parser checked the
actual reference before/after files (67,932,232 bytes each): original
header, render/pose, colors, materials, faces and UV words are unchanged;
Save/Undo/Redo are byte-exact. SculptGL changed 37,292 vertices / 75,367
affected triangles. Neither output introduced a degenerate or reversed
affected triangle. OBrush's maximum/RMS displacement among changed vertices
was 0.224600 / 0.112670; SculptGL's was 0.216663 / 0.109901. Because the
applications applied different dabs at different times, these aggregate
values do **not** establish numerical brush parity or a causal quality ratio.

The [OBRush browser result](../.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-root-b16cbce9-74a8-4078-aed6-e482806a2d52/result.json)
has SHA-256 `45bdb6bc641b884040c9356ef603ebc3edae4d4f38a4c6ea76deb61baab19406`.
The [SculptGL browser result](../.artifacts/sculptgl-sparse-guard-c69-stage/bench/actual/SP-LONG00-1m-60s-source-723d26f2-1d89-4496-834e-5da8d9d02672/result.json)
has SHA-256 `ac4af25aa79da1af0a64a8336e72da636bda3ce2c6a789803512aa1577e00330`.
The paired timing summary `.artifacts/c226-paired-summary.json` has SHA-256
`4e835b834d689246239ea4f40dbb4ec7fd140a38efe6a46802068790cdf88c33`.
The independent root audit `.artifacts/c226-root-audit.json` has SHA-256
`64b671be454faaed50c3c08b1733ed2c8125cd2f07997b37331d58b1b54ecd26`;
the source SGL audit `.artifacts/c226-source-sgl-audit.json` has SHA-256
`1d51937081af2aee124f52f887a1ba843901e18263b4d1e9fceae6290f911375`.
An attempted run of the root JSON auditor on binary source SGL failed with
a UTF-32 decode error; `.artifacts/c226-source-audit.json` preserves that
failed check. The source-specific raw-word audit above replaced it.

This cohort makes the remaining performance gap concrete. It covers one
frozen million-vertex Brush workload, not all sculpt tools, devices, scenes,
save formats, native GUI behavior or the complete 317-ID SculptGL inventory.
