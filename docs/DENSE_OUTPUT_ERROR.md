# Dense-output (interior) error of adaptive RODAS5P

Audit F-002, 2026-09-29.

The adaptive RODAS5P step is accepted on its endpoint embedded estimate. That
estimate does not control the continuous extension between the endpoints.
RODAS5P dense output is order 4 on nonstiff problems and about order 3 on
stiff ones. **Under `DenseErrorControl::Off`, which is what
`integrate_adaptive_dense_observed_with_config` uses, interior values are not
tolerance-controlled.** Errors measured on a dense output grid are
interpolant errors, not endpoint errors.

`integrate_adaptive_dense_observed_with_dense_error_control` takes a mode:

| Mode | Effect |
|---|---|
| `Off` | the legacy path, bit for bit |
| `Report` | for every accepted step that contains an interior output time, records a filtered-defect estimate at theta = 1/2; the run is unchanged and the estimate's work is charged to the report only |
| `Enforce` | as `Report`, and rejects the step when the estimate exceeds one tolerance unit; the estimate's work is charged to the run |

The estimate: with d = M u'(t_m) - f(t_m, u(1/2)) the interpolant defect at
the half step, solve (M - (h/2) J) e = (h/2) d and take the WRMS of e. That
is one implicit-Euler step of M e' = J e + d over half a step: it stays
bounded on stiff components (e -> -J^{-1} d) and is (h/2) M^{-1} d on
nonstiff ones. Cost per sampled step: one right-hand side, one Jacobian and
one LU.

Measured on Prothero-Robinson, lambda = -1e5, t in [0, 2], 201 output times
(`tests/dense_error_control_contracts.rs`):

| rtol | true interior max (tolerance units) | `Report` estimate | `Enforce`: true interior max |
|---|---:|---:|---:|
| 1e-4 | 3.2 | 2.7 | 0.92 |
| 1e-6 | 312 | 263 | 1.05 |
| 1e-8 | 209 | 162 | 1.38 |

Not done: the fair-ab two-arm campaign records do not carry an
"interpolant error" label, because adding a field would change the sealed
v2 record checksums. The dense arm of that campaign is documented here
instead.
