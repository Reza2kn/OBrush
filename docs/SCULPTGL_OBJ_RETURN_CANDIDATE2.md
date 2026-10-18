# OBrush 0.1.16 candidate 2: retained actual OBJ UV return

The normal browser OBJ consumer now passes both retained triangle and quad
UV-return fixtures on the frozen candidate 2 build. Positions, RGB, canonical
geometric normals, triangle indices, original faces and corner UV IDs are exact.
Distinct equal-valued seam IDs remain distinct. The imported version 6 project
keeps original surface metadata, pauses adaptive detail and matches its complete
visible GPU buffers. Whole normal-import undo/redo is exact.

The [original candidate 1 UV-loss failure](SCULPTGL_ATTRIBUTES_CANDIDATE1.md)
remains unchanged. This rerun uses the same actual prior browser-produced files,
so its inputs and expectations were not replaced to obtain a pass. The
[machine-readable receipt](evidence/sculptgl-obj-return-candidate2.json) binds
the files, source projects, build, actual input/download events and all checks.
Independent Python repacking confirms **nine actual downloads and nine complete
geometry/material GPU pairs**.

- Source manifest SHA256:
  `6dfb7a028430577a3a7b0d62e513a50178e1c36c98530e208b132285fbb62f8e`.
- Build record SHA256:
  `34b14d67ddfc854d1c4a7040f27534d84d8d87e2748c48518b1654b3072bee90`.
- Frozen/executed helper SHA256:
  `2969aa2a1f219a0010a0a8079ac9d3bda9a4406b48d95aa319b6855d7227f557`.
- Actual triangle OBJ SHA256:
  `3c138d9e31814e5eae0853962b36f7956c5fc34b90bd06b58543b8f8f63c0ec3`.
- Actual quad OBJ SHA256:
  `eaa1c47d67a0205938e4c1bbc0e85cf03d0c66489ab18d1bff6704168173b9b3`.

The triangle fixture has 2,522 vertices, 5,040 original faces and 15,120 UV
coordinates; the quad fixture has 1,538 vertices, 1,536 original faces and 1,734
UV coordinates. The original producers explicitly omitted material rows and
standard OBJ masks. These imports correctly retain implicit material defaults
and zero masks; this is not a material-file exchange test. Supplied OBJ normal
indices are validated and normals recomputed from geometry under the declared
consumer policy. Exact normal equality here applies to these canonical geometric
source normals, not arbitrary authored normals.

Only startup and the two normal import/save/history workflows run in this small
check. Prior shader, export-policy, Paint and static-subdivision groups are not
repeated. All actual served HTML/JS/WASM hashes match the immutable web files;
the topology worker is frozen but not requested. Trusted file input and original
download bridges are used without assigning app, mesh, tool or camera state.

Owned isolated headless Chrome 155.0.8059.40 and its localhost server are closed;
Chrome exited with code 0 and its owned PID is confirmed gone. User tabs/profiles,
native GUI and accessibility were not controlled. Object/group/MTL semantics,
partial-UV policy, complete malformed/allocation input, native execution and
matched SculptGL/ZBrush comparisons remain separate gates. No full OBJ parity,
visual, performance or latency conclusion is drawn.
