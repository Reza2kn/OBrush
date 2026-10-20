# M03 packaged native Move consumer gate

**Frozen before UI contact.** Use the M02-built, ad-hoc-signed
`.artifacts/OBrush.app` executable with SHA-256
`8dc4e14d400afbad61da81a796869f4feae4deaa692c0614b0c210a2ee6c2d18`.
Start with its fresh default sphere. Through visible controls select Move,
set radius `0.037` and strength `79%`, leave X symmetry and backface culling
on, and record a screenshot of those values. Save a public `.obrush` project
before contact. Drag from a visibly front-facing region of the sphere in
at least eight separately timed held pointer moves over at least 250 ms,
then release. Preserve the input-coordinate/time record, a post-contact
screenshot, and a second public project. Undo once and save; Redo once and
save. Do not use internal geometry mutation APIs.

Pass requires the actual visible settings, a distinct before/after public
project with at least eight changed original vertices on both X sides,
finite valid geometry and no new degenerate triangles or inverted radial
faces, exact before mesh/project after one Undo and exact after mesh/project
after one Redo. Record face-quality and radial cosine minima before/after,
plus any visible pointed/triangular artifact in screenshots. If the gesture
does not edit, preserve it as an inconclusive or failed consumer observation
with frame/pick uncertainty; do not reinterpret it as a geometry pass.

This is one native UI fixture, not parity with ZBrush or closure of the
user's long-brushing tearing report. A clear changed project and visible
quality comparison are both necessary before saying the specific native
consumer behavior is improved.
