# C285 prospective mode-first Smooth Tangent pair after C283 setup failure

C283's Source-on run completed, but its OBrush-on setup stopped before
sculpting because the frozen harness requested `source_geometry.pressure.radius.value`
while Smooth was still in `Legacy` mode. Current OBrush code renders the
numeric Source geometry fields only after selecting `SourceOrdinary`.
The raw failed C283 run and frozen harness remain unchanged.

C285 repeats the same independent fixture SHA-256
`5cc5947599f58cbfc738035d64504e7763368631f70a93cbfe55c6263da5c777`,
pinned SculptGL commit/bundle, four fresh normal-UI documents, visible
Smooth Relax-only on/off, radius64px/intensity50%, no pressure, Alpha None,
culling/symmetry/Continuous/Lock off, 1024×808 front-orthographic fit,
integer center `(+3,-20)`→`(+16,-20)` mouse gesture, public
Save/Undo/Redo and active GPU observations, and **all** frozen C283
numerical, geometry, protection, history, topology and display-normal
gates. The *only* harness setup change is to click the visible Smooth
`SourceOrdinary` mode control **before** editing the numeric Source fields.
This changes UI setup order, not the effective settings, fixture, pointer
path or acceptance tolerance. A new Source run is required so both sides
share the same new harness hash. Preserve any failed setup or numerical
check. No product edit before the paired baseline.
