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
| `Off` | the default; the legacy path, bit for bit |
| `ReportDefect` | records the unfiltered defect (h/2) M^{-1} d of every accepted step that contains an interior output time: one right-hand side, no Jacobian, no LU (a mass solve when M != I); the run is unchanged |
| `Report` | for every accepted step that contains an interior output time, records a filtered-defect estimate at theta = 1/2; the run is unchanged and the estimate's work is charged to the report only |
| `Enforce` | as `Report`, and rejects the step when the estimate exceeds one tolerance unit; the estimate's work is charged to the run |

The estimate: with d = M u'(t_m) - f(t_m, u(1/2)) the interpolant defect at
the half step, solve (M - (h/2) J) e = (h/2) d and take the WRMS of e. That
is one implicit-Euler step of M e' = J e + d over half a step: it stays
bounded on stiff components (e -> -J^{-1} d) and is (h/2) M^{-1} d on
nonstiff ones. Cost per sampled step: one right-hand side, one Jacobian and
one LU.

The filtered modes cost a Jacobian and an LU per sampled step, so they are
audit modes for matrix-free runs, not defaults (external re-audit RA-07).
`ReportDefect` is the cheap alternative. It is unfiltered: on a stiff
component the defect carries the factor |h lambda|, so it overstates the
error there. It is a pointwise sample at theta = 1/2, not a bound on the
interval, and it does not separate the local defect from the error
propagated from earlier steps.

| Case | true interior max | `ReportDefect` estimate |
|---|---:|---:|
| PR lambda = -1e5, rtol 1e-6 | 312 | 1.3e7 |
| PR lambda = -1, rtol 1e-6 | 0.140 | 0.196 |
| PR lambda = -1, rtol 1e-8 | 0.168 | 0.261 |

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
