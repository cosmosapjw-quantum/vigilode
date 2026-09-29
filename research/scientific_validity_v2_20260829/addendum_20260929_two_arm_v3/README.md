# Two-arm admissibility v3 on the n = 96 calibration rows (2026-09-29)

Append-only addendum. No committed v2 record, ledger or frozen rule is changed.

## What was run

`crates/rodas5p-fair-ab/examples/two_arm_v3_calibration96.rs` runs every n = 96
calibration row through `run_two_arm_admissibility_v3_case_synthetic_smoke`
(protocol `TWO_ARM_ADMISSIBILITY_PROTOCOL_ID`, all three parts of the audit
F-007 design, plus the F-033 attribution arm). The clipped and dense arms are
the v2 arms, bit for bit (`two_arm_v3_campaign_contracts.rs` checks their
output checksums against a v2 run of the same row).

- **Authority:** `SyntheticCiSmoke`. This is diagnostic evidence, not a
  canonical campaign row.
- **Code:** branch `claude/jolly-wozniak-7wl15h-wu6c-interpolant-check`, which
  is the audit base `e1c3a5f` plus WU-6a. The solver is therefore the v2
  solver; the WU-3 forcing change and the WU-6b controller change are not in
  it.
- **References:** the families' manufactured exact solutions, so reference
  uncertainty is 0. Robertson, HIRES, Van der Pol and the non-autonomous
  stiff-forcing family have no exact solution, and their 12 rows were
  skipped. They need the Radau references, which are not in the tree.

Files:

| File | Content |
|---|---|
| `two_arm_v3_calibration96.json` | the six v3 records |
| `two_arm_v3_calibration96.log` | one line per row, including the skipped rows |
| `semilinear_fixed_h_and_local_error.txt` | output of `examples/semilinear_fixed_h_order.rs` |

## Results

Case-tolerance units, B = 10, delta limit 2.

| Row (n = 96) | clipped | dense | status | max delta | R_inner | dense steps |
|---|---:|---:|---|---:|---:|---:|
| rotating-nonnormal, rtol 1e-4 | 0.071 | 1.39 | Pass | 0.91 | 0.94 | 48 |
| rotating-nonnormal, rtol 1e-6 | 2.07 | 3.65 | Pass | 0.79 | 1.01 | 141 |
| rotating-nonnormal, rtol 1e-8 | 1.38 | 1.62 | Pass | 0.09 | 1.00 | 421 |
| semilinear, rtol 1e-4 | 7.25 | **17.0** | GlobalErrorExceedsBudget (dense) | 0.15 | 0.80 | 15 |
| semilinear, rtol 1e-6 | 0.10 | **53.2** | GlobalErrorExceedsBudget (dense) | 0.33 | 1.00 | 27 |
| semilinear, rtol 1e-8 | **19.8** | **207.5** | GlobalErrorExceedsBudget (both) | 0.96 | 1.00 | 60 |

- **Part C.** Every recorded delta is at most 0.96, below the limit of 2.
  About 98 interior output times were audited per row.
- **Semilinear rows.** The dense errors 17 / 53 / 207 reproduce the audit's
  E-03 values. Under v3 they are `GlobalErrorExceedsBudget`, not a policy
  verdict.

## F-033 attribution

1. **Not the inexact stage solves.** R_inner = E_dense / E_tight-inner is
   0.80, 1.00 and 1.00: solving every stage to rtol 1e-10 / atol 1e-12 gives
   the same global error and the same step sequence.
2. **Not fixed-h order reduction.** At fixed h = span/N with stages solved to
   1e-13, the endpoint error falls with observed order 5.01, 4.98, 4.99, 4.96
   for N = 25 to 400 (4.21 at N = 800, at the rounding floor).
3. **Local error control holds.** On every step the adaptive dense arm
   accepted, the true local error of that step size, taken from the exact
   state, is at most 0.28 / 0.48 / 1.36 tolerance units (rtol 1e-4 / 1e-6 /
   1e-8). The embedded estimate is conservative: true/estimate is at most
   0.90 / 1.13 / 1.60.
4. **The global error is propagation.** The sums of the true local errors
   over all accepted steps are 1.37 / 2.67 / 6.83 units, 12 to 30 times
   smaller than the global errors of 17 / 53 / 207. Per-step error control
   does not bound global error on this problem: the flow amplifies the
   sub-tolerance local errors. The amplification factor was not measured
   directly, for example by linearized error propagation.

This corrects the audit's reading. The audit said the exceedance "cannot be
produced by accumulating <=1-tolerance-unit local errors". That is true of
accumulation alone, but the local errors here are not only accumulated, they
are also propagated. The accepted steps' local errors are within tolerance,
and the embedded estimator is not blind to them.

## Not done

- The 12 rows without exact solutions.
- The n = 384 and n = 1536 rows.
- The Oregonator holdout.
- Wiring the v3 runner into the canonical campaign freeze or replay. The
  canonical runner `run_two_arm_admissibility_v3_case` exists but was not
  executed.
- Choosing a global-error control for error-amplifying problems. This is a
  method decision for the owner.
