Upstream: https://github.com/shuoli84/slsqp-rssl
Pinned commit: 4091ed1215556e6d7ccc334c17a3f6333f52c547
License: MIT (see LICENSE)
Local changes: optional whole-Jacobian and objective-gradient API; legacy finite differences remain default.

Upstream describes this Rust implementation as a port of SciPy's Fortran SLSQP.
`SCIPY-BSD-LICENSE.txt` and `SLSQP-BSD-NOTICE.txt` preserve the SciPy and
Dieter Kraft/ACM notices. Their notice source is SciPy v1.15.3; this does not
assert that it was the exact Fortran revision used by the Rust author.

OBrush uses the all-inequality analytic entry point with bounded variable and
constraint counts. Final rounded candidates still pass OBrush's independent
geometry, motion and picked-ray checks before publication.
