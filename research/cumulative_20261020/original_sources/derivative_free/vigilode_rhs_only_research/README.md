# VigilODE derivative-free FVM research, 2026-10-20

Start with `REPORT_KO.md`. This is a source-grounded divergent theory/research
package, not a production solver or a speed benchmark. No GitHub code/defaults
were changed in this node.

## Contents

- `REPORT_KO.md`: own-PR lineage, new equations, assumptions, derived certificates,
  counterexamples to supplied advice, and native implementation contracts.
- `experiments.py`, `RESULTS.json`, `EXECUTION.log`: six bounded numerical/algebraic
  experiment groups including exact low-rank physical-stencil reduction,
  local reaction elimination, Schur control, rational continuous defect,
  frozen-factor order loss and conservative remapping counterexamples.
- `adaptive_scalar_certified.py`, `ADAPTIVE_SCALAR_RESULT.json`: actual adaptive
  scalar curve with inherited radii and binary64 dense-output bounds. The proof
  data are rational; this is not a generic FVM certificate or competitive runtime.
- `verify_certificate.py`, `CERTIFICATE_READBACK.json`: a separate rational
  readback of all 28 windows, 33 trials and 101 reported outputs. No reference
  truth is used for admission. Same author, not independent personnel/formal proof.
- `SYMBOLIC_CHECK.wl`, `SYMBOLIC_RESULT.txt`: actual connected-Wolfram checks.
- `SOURCE_MAP.json`: pinned own repository sources, their scope and new reuse.

## Reproduce in a new directory

Copy or extract this package into a fresh working directory so historical output
files remain unchanged. A Python environment with numpy, scipy and mpmath is
needed for the experiments and high-precision independent checks. Exact executed
versions are in ENVIRONMENT.json and VALIDATION.json. The proof-data readback
requires Python standard library only.

```bash
export OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1
python experiments.py
python adaptive_scalar_certified.py
python verify_certificate.py
```

The scripts write their results next to themselves. Reference DOP853 is used
only on the 32-state low-rank toy. No original VODE/Diffsol timing data, failed
solver record, historical code or original PR authority is overwritten.

`derivative_calls=0` means the new candidate has no nonlinear state Jacobian,
JVP, AD or finite-difference-Jv request. Known physical linear stencil actions,
exact model-specific polynomial operations, and time derivatives of the returned
curve are permitted and disclosed. The toy rank-one closure must not be imposed
on general spatially dependent absorption.

The scalar certificate applies to y'=-2y-y^2, y(0)=1/2, t in[0,1], exact Fraction
proof arithmetic and explicit binary64 output conversions. Bernstein convex hull,
nonnegative invariance and the exact two-point dissipativity identity provide the
continuous argument. 101 high-precision sample checks alone are NOT the proof.
No whole native Rust integration, global arbitrary-client certificate, zero-
allocation result, or performance superiority is claimed.
