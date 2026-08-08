# Already-converged refined-root certificate root cause

`refine_target_root` initialized `last_correction_wrms` to positive infinity.  The initial
sequential stage vector can already satisfy the target residual at iteration zero, in which
case no correction exists.  The convergence branch correctly accepted iteration zero, but
the final finite-value audit then rejected the untouched infinity.

Fix: represent the internal last correction as `Option<f64>`.  Iteration-zero convergence
requires no correction; the serialized/report value is 0.0 when no correction was needed.
This does not weaken the residual gate and preserves strict correction checks after any
actual refinement iteration.
