## Task 2: B1b-gmres — Krylov-accelerated lagged factorization (node id `b1b-gmres`)

New hypothesis card (the wave-1 B1 Richardson card FAILED its kill gate: refreshes forced by the 6-iteration
cap because q ≈ 0.1–0.2 per Richardson step against a ~1e-6 reduction target). New mechanism: replace
Richardson by GMRES right-preconditioned with the same stored LU P (T2: GMRES residual ≤ κ(P) q^k, and
GMRES is never worse than Richardson's polynomial), so the same refresh rule fires far less often.

- In `crates/rodas5p-integrators/src/rodas5p_lagged.rs` add an option `StageSolver::{Richardson (default,
  wave-1 behaviour bit-identical), Gmres}` using `rodas5p-krylov` `gmres_into` with right preconditioning
  (`Preconditioner` side Right), restart = 10, stopping on the same T9 targets ρ_i measured on the
  preconditioned residual P⁻¹r (compute it explicitly at each GMRES exit check if gmres_into stops on the
  true residual; document which residual is tested).
- Refresh rule unchanged in spirit: refresh when the GMRES iteration count for a stage exceeds 6 or the
  observed convergence factor (‖r_k‖/‖r_0‖)^(1/k) > 0.5.
- Preregister before measuring (`docs/plan/nodes/b1b-gmres.md` first section, committed and pushed before
  the measurement run): same problems, tolerances, reference and kill gate as wave-1 Task 3 —
  factorizations per accepted step ≤ 0.4, accepted steps ≤ +5% over the fast driver, endpoint error ≤ 1.5x
  the fast driver's — on HIRES and Brusselator 1D n = 100 at rtol 1e-6. Report PASS/FAIL per problem with
  all counters; never tune the constants.
- Tests: forced refresh every step with `Gmres` still reproduces the fast driver within 1e-12 relative on
  Robertson and HIRES; JVP path exercised with iterations > 0.

