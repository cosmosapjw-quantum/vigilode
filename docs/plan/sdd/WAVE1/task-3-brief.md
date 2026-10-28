## Task 3: B1-lagged — lagged-factorization stage-root driver (node id `b1-lagged`)

Plan §5 T1, T2, T9; §6 B1. New file `crates/rodas5p-integrators/src/rodas5p_lagged.rs` (plus `lib.rs`
export), an opt-in driver `integrate_rodas5p_lagged_observed(problem, span, y0, adaptive, output, options)`.

- Same method and coordinates as the dense fast driver (`rodas5p_fast.rs`, U form with
  W = I/(hγ) − J, `u_i = Σ Γ_ij k_j`), same controller, clock and output rules (reuse its functions; make
  them `pub(crate)` if needed — no arithmetic change in `rodas5p_fast.rs`).
- Each stage system W u = c is solved by preconditioned Richardson iteration
  x ← x + P⁻¹(c − W x), starting from x = P⁻¹c, where P is a stored LU of I/(h_P γ) − J_P from an earlier
  step (h_P, J_P its step size and Jacobian). W x uses the current step's J₀: through the problem's JVP if
  present, else through the explicit current Jacobian's matvec. The Jacobian is evaluated each step (the
  exact RODAS5P target, T1); only the factorization is lagged.
- Stopping rule (T9): stage i stops when WRMS(P⁻¹ r) ≤ ρ_i, ρ_i = θ / (8 · max(a_i, 1)), θ = 0.1, with the
  nonstiff-limit sensitivities a = (0.58, 0.24, 6.25, 5.97, 3.73, 4.57, 1.01, 1.00) as constants with a
  doc comment citing T1. WRMS uses the step's error scale. Note: in the scaled form W = I/(hγ) − J the
  residual must be converted to the state-increment units of the U form before the WRMS test; derive and
  document the factor.
- Refresh rule (T2): estimate q_k = WRMS(P⁻¹r_{k+1}) / WRMS(P⁻¹r_k). Refactor P at the current (h, J₀) and
  restart the stage when q_k > 0.5, or when the iteration count exceeds 6, or before the first step. A refresh
  is a counted event (`direct_factorizations` plus a new counter in the driver's result); the abandoned
  iterations stay counted.
- When P was refreshed in the current step (P = W exactly), the first iterate is the direct solve; the
  driver then reproduces `integrate_rodas5p_fast_observed` up to rounding. Test: with an option that forces
  a refresh every step, outputs equal the fast driver's within 1e-12 relative on Robertson and HIRES.
- Measurement test (`#[ignore]`d, recorded in the node report, labelled diagnostic): HIRES and Brusselator
  1D (n = 100, the definitions used by `rodas5p-cli/src/stiff_benchmark.rs`), rtol 1e-6, atol per those
  definitions: report accepted/rejected steps, Jacobian evaluations, factorizations, Richardson iterations,
  JVP/matvec count and endpoint error against a tight reference (fast driver at rtol 1e-11) for fast vs
  lagged. Kill gate (§6 B1): factorizations per accepted step > 0.4, or accepted steps more than +5% over
  the fast driver, or endpoint error more than 1.5x the fast driver's. Report PASS/FAIL per problem; never
  tune θ or the thresholds to pass.

