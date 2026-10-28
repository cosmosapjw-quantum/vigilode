# Task 3 report — b1-lagged

Status: DONE_WITH_CONCERNS. The code is complete and tested. The preregistered kill gate gives **FAIL on both problems**, which is a valid scientific outcome.

Branch `production/b1-lagged` (from 9b5d083), pushed. Commits:
- `c1dfb9e` feat(integrators): B1 lagged-factorization stage-root RODAS5P driver
- `16f6631` docs(plan): b1-lagged node report — kill gate FAIL on HIRES and Brusselator

Node doc: `/home/claude/wt/b1-lagged/docs/plan/nodes/b1-lagged.md` (full table, commands, exit codes, deviations).

## Files
- `/home/claude/wt/b1-lagged/crates/rodas5p-integrators/src/rodas5p_lagged.rs` (new, 569 lines). Contains `integrate_rodas5p_lagged_observed(problem, span, y0, adaptive, output, Rodas5pLaggedOptions)`, `Rodas5pLaggedResult { fast, work: LaggedWork }`, the constants θ = 0.1, q_max = 0.5, 6 iterations and the a_i (doc comment cites T1), `rodas5p_lagged_stage_targets()`, and a module doc that derives the unit factor.
- `/home/claude/wt/b1-lagged/crates/rodas5p-integrators/src/rodas5p_fast.rs`: visibility only. `Workspace`, 17 stage fields, `new`, `choose_lu`, `factor`, `solve` and `failure_kind` become pub(crate). No arithmetic, banded-solve or error-norm lines changed.
- `/home/claude/wt/b1-lagged/crates/rodas5p-integrators/src/lib.rs`: module and re-exports.
- `/home/claude/wt/b1-lagged/crates/rodas5p-integrators/tests/b1_lagged_contracts.rs` (new). HIRES and Brusselator constructors copied from stiff_benchmark.rs.

## Unit factor
r = c − W x is in RHS units, and the plan's state-unit residual is hγ·r. The tested quantity is P⁻¹r with P = I/(h_Pγ) − J_P. P has the same 1/time scaling as W, so P⁻¹r is a U-form state increment and the factor is exactly 1. In the plan's normalisation, P⁻¹r = (h_P/h)·P_T1⁻¹ r_T1. Since P⁻¹r_k = −(I − T)e_k, the test estimates the stage error within 1 ± q.

## Tests (stable 1.97.0, --locked -j 1, all exit 0)
- `b1_lagged_contracts`: 3 passed, 1 ignored.
  - Forced refresh reproduces the fast driver on Robertson and HIRES at rtol 1e-4, 1e-6 and 1e-8. The test asserts 1e-12 relative; a one-off `assert_eq!` on all states also passed, so the outputs are bit-identical. Step, factorization and Jacobian counts are equal.
  - Accounting identities, JVP path on Robertson, matvec path on HIRES.
  - Stage targets.
- `rodas5p_fast_contracts` 4 passed. `rodas5p_fast_allocations` 2 passed. `int03_banded_fast_contracts` 4 passed. `int03_banded_fast` 0 passed, 1 ignored. `--lib rodas5p_fast` 4 passed.
- clippy (`--lib --test b1_lagged_contracts`): no warnings in touched files. fmt: touched files clean.

## Measurement (rtol 1e-6; reference = fast at rtol 1e-11; endpoint max error in tol units)

| problem | arm | accepted | rejected | Jac | fact | fact/acc | Rich. it (abandoned) | W·x | err abs | err tol-units |
|---|---|---|---|---|---|---|---|---|---|---|
| HIRES | fast | 204 | 6 | 204 | 210 | 1.029 | 0 | 0 | 1.640e-9 | 0.259 |
| HIRES | lagged | 205 | 8 | 205 | 134 | 0.654 | 4434 (770) | 4434 | 2.602e-9 | 0.410 |
| Bruss n=100 | fast | 82 | 10 | 82 | 92 | 1.122 | 0 | 0 | 7.903e-6 | 1.686 |
| Bruss n=100 | lagged | 82 | 10 | 82 | 70 | 0.854 | 1673 (386) | 1673 | 7.891e-6 | 1.683 |

Wall time (debug build, diagnostic only): HIRES 8.9 ms fast vs 33.7 ms lagged; Brusselator 51 ms vs 348 ms.

Kill gate:
- HIRES **FAIL**: fact/acc 0.654 > 0.4 and error ratio 1.587 > 1.5; steps +0.5% passes.
- Brusselator **FAIL**: fact/acc 0.854 > 0.4; steps +0% and error ratio 0.998 pass.

Diagnosis, from a temporary uncommitted trace (interpretation): budget refreshes dominate (126/134 on HIRES, 62/70 on Brusselator).
- The first preconditioned residual is 1e3–1e4 tolerance units against ρ = 0.0125 (0.002 for stages 3–6).
- q ≈ 0.10–0.20 at the usual h/h_P ≈ 1.1–1.2, which matches T ≈ 1 − h_P/h on nonstiff components (Corollary 2a).
- Six iterations fall just short of the ~1e-6 reduction needed. During step growth (h/h_P of 3–5), q ≈ 0.8, which triggers contraction refreshes.

Untested follow-ups (not tried, to avoid tuning): a better start than P⁻¹c, GMRES instead of Richardson, a step-normalised P.

## Deviations
- WRMS weights are atol + rtol|y_n|, because y_new is unknown during the stages. This is never looser than the step scale.
- Convergence is checked before the q rule, and the last correction is applied on convergence.
- "Brusselator n = 100" means `brusselator_problem(50)`.

## Concerns
1. **Shared target dir corruption hazard.** Cargo hashes workspace path packages by relative path, so the five worktrees overwrite each other's `rodas5p-integrators` artifacts. Twice, my test target was linked against another node's lib (E0432, missing symbols). To isolate, I added `--config profile.dev.package.rodas5p-integrators.codegen-units=255`, which gives the crate its own hash; the env vars are exactly the controller's. Other nodes' test results in the shared dir may be affected in the same way, including false passes against a stale lib.
2. The B1-lagged gate is FAIL; B1 as specified (Richardson, 6 iterations) should not be promoted.

Memory note for main session: VigilODE B1-lagged (Richardson on a lagged LU, T1/T2/T9) failed the preregistered kill gate on 2026-10-21. Factorizations per accepted step were 0.65 on HIRES and 0.85 on Brusselator n = 100, limited by the 6-iteration budget at q ≈ 0.1–0.2.
