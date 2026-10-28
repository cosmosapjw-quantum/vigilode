# Node b1-lagged: lagged-factorization stage-root RODAS5P driver

Production wave-1, Task 3. Plan: `docs/plan/PRODUCTION_PLAN_20261020.md` §5 T1, T2, T9; §6 B1.
Branch `production/b1-lagged` from `9b5d083`. Toolchain: `RUSTUP_TOOLCHAIN=stable`
(rustc 1.97.0, cargo 1.97.0); `rust-toolchain.toml` (1.94.1) not used, not edited.

**Verdict: kill gate FAIL on both problems** (HIRES: factorizations per accepted step and endpoint error;
Brusselator 1D n = 100: factorizations per accepted step). The preregistered constants
(θ = 0.1, q_max = 0.5, 6 iterations, the a_i) were not changed after measurement.

## What was done

* `crates/rodas5p-integrators/src/rodas5p_lagged.rs` (new): opt-in
  `integrate_rodas5p_lagged_observed(problem, span, y0, adaptive, output, Rodas5pLaggedOptions)`, result
  `Rodas5pLaggedResult { fast: Rodas5pFastResult, work: LaggedWork }`, driver id
  `rodas5p-lagged-richardson-v1`. Same U form (`W = I/(hγ) − J₀`), same stage right-hand sides, update,
  embedded-error norm, controller (`rodas_next_step_after_attempt`), clock (`adaptive_end_step`,
  `below_min_step`) and output collector as the dense fast driver with default options; it reuses the fast
  driver's `Workspace` (`factor`, `solve`, `choose_lu`) unchanged.
  * J₀ = J(tₙ, yₙ) is built at every new state (T1 exact target); only the LU is lagged.
  * Stage solve: x = P⁻¹c, then x ← x + P⁻¹(c − Wx); W x through the problem's JVP when present
    (Robertson), else the current dense Jacobian's matvec (HIRES, Brusselator). No finite differences.
  * Stop when WRMS(P⁻¹r) ≤ ρᵢ = θ/(8·max(aᵢ,1)), θ = 0.1, a = (0.58, 0.24, 6.25, 5.97, 3.73, 4.57, 1.01, 1.00)
    (constants `RODAS5P_LAGGED_THETA`, `RODAS5P_NONSTIFF_STAGE_SENSITIVITIES`, doc comment cites T1).
  * Refresh (refactor at the current (h, J₀), restart the stage directly) before the first step (and after
    a failed attempt), when qₖ = WRMS(P⁻¹r_{k+1})/WRMS(P⁻¹r_k) > 0.5, or when a 7th iteration would be needed.
    Every refresh increments `direct_factorizations` and `LaggedWork::preconditioner_refreshes` (split by
    reason); abandoned iterations stay in `richardson_iterations` and are also in `abandoned_iterations`.
  * When P was factored at the attempt's own (h, J₀) the stage is the direct solve with no iteration.
* `rodas5p_fast.rs`: visibility only (`Workspace`, 17 of its fields, `new`, `choose_lu`, `factor`, `solve`,
  `failure_kind` → `pub(crate)`); no arithmetic or banded/error-norm code touched.
* `lib.rs`: module and re-exports.
* `tests/b1_lagged_contracts.rs` (new): HIRES and Brusselator constructors copied from
  `rodas5p-cli/src/stiff_benchmark.rs` (no new dependency).

## Unit conversion (documented in the module doc)

The fast system is the plan's (I − hγJ₀)Uᵢ = … divided by hγ, so the raw residual r = c − Wx is in
right-hand-side units and the plan's state-unit residual is r_T1 = hγ·r. The tested quantity is P⁻¹r with
P = I/(h_Pγ) − J_P, which carries the same 1/time scaling as W, so P⁻¹r is a U-form state increment with
**factor 1**. In the plan's normalisation P_T1 = h_Pγ·P, P⁻¹r = (h_P/h)·P_T1⁻¹ r_T1; the h_P/h factor only
appears if the preconditioner is renormalised, which the iteration does not do. P⁻¹rₖ = −(I − T)eₖ (T2), so
the test estimates the stage error within 1 ± q.

## Commands (all with `RUSTUP_TOOLCHAIN=stable CARGO_TARGET_DIR=/home/claude/targets/shared CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0`, `--locked -j 1`, plus `--config profile.dev.package.rodas5p-integrators.codegen-units=255`, see Deviations)

| command | exit | result |
|---|---|---|
| `cargo test -p rodas5p-integrators --test b1_lagged_contracts` | 0 | 3 passed, 1 ignored |
| `cargo test -p rodas5p-integrators --test b1_lagged_contracts -- --ignored --nocapture` | 0 | 1 passed (measurement below) |
| `cargo test -p rodas5p-integrators --test rodas5p_fast_contracts` | 0 | 4 passed |
| `cargo test -p rodas5p-integrators --test rodas5p_fast_allocations` | 0 | 2 passed |
| `cargo test -p rodas5p-integrators --test int03_banded_fast` | 0 | 0 passed, 1 ignored |
| `cargo test -p rodas5p-integrators --test int03_banded_fast_contracts` | 0 | 4 passed |
| `cargo test -p rodas5p-integrators --lib rodas5p_fast` | 0 | 4 passed (incl. `rodas5p_fast::tests::in_place_lu_solves_pivoted_and_banded_systems`) |
| `cargo clippy -p rodas5p-integrators --lib --test b1_lagged_contracts` | 0 | no warnings in touched files |
| `cargo fmt -p rodas5p-integrators -- --check` | — | touched files clean (pre-existing diff in `production_audit_packet_curve.rs` only) |

Equivalence: with `refresh_every_attempt: true` (no iteration runs, so no W·x is formed), Robertson and HIRES at rtol 1e-4,
1e-6, 1e-8 give the fast driver's outputs, step counts, factorization and Jacobian counts; the test
asserts 1e-12 relative, and a one-off check with `assert_eq!` on all output states also passed
(bit-identical; not kept in the committed test). The default-option contract test checks the
accounting identities on Robertson and HIRES at rtol 1e-6; on Robertson (which declares a JVP) every W·x
went through the JVP, and on HIRES through the matvec.

## Measurement (preregistered; rtol 1e-6, atol = rtol × the benchmark's atol_scale; reference = fast driver at rtol 1e-11; debug build)

Error = endpoint, max over components; "tol units" = |y − ref| / (atol + rtol |ref|). Wall time is
**diagnostic only** (shared 2-core host, unoptimised build). Counts are reproducible (two runs identical).

**HIRES** (n = 8, t ∈ [0, 321.8122], atol 1e-10)

| arm | accepted | rejected | Jacobians | factorizations | fact/accepted | Richardson it. | abandoned it. | W·x (matvec) | endpoint max abs err | endpoint err (tol units) | wall ms (diagnostic) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| fast | 204 | 6 | 204 | 210 | 1.029 | 0 | 0 | 0 | 1.640e-9 | 0.259 | 8.85 |
| lagged | 205 | 8 | 205 | 134 | 0.654 | 4434 | 770 | 4434 | 2.602e-9 | 0.410 | 33.69 |

Refreshes: initial 1, contraction 7, budget 126. Stages: exact 744, lagged-converged 827, refreshed 133.

**Brusselator 1D, 50 cells (n = 100)** (t ∈ [0, 10], atol 1e-6)

| arm | accepted | rejected | Jacobians | factorizations | fact/accepted | Richardson it. | abandoned it. | W·x (matvec) | endpoint max abs err | endpoint err (tol units) | wall ms (diagnostic) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| fast | 82 | 10 | 82 | 92 | 1.122 | 0 | 0 | 0 | 7.903e-6 | 1.686 | 51.35 |
| lagged | 82 | 10 | 82 | 70 | 0.854 | 1673 | 386 | 1673 | 7.891e-6 | 1.683 | 347.62 |

Refreshes: initial 1, contraction 7, budget 62. Stages: exact 392, lagged-converged 275, refreshed 69.

## Kill gate (§6 B1) per problem

| problem | fact/accepted (fail > 0.4) | accepted vs fast (fail > 1.05) | error vs fast (fail > 1.5) | verdict |
|---|---|---|---|---|
| HIRES | 0.654 — fail | 1.0049 — pass | 1.587 — fail | **FAIL** |
| Brusselator n = 100 | 0.854 — fail | 1.0000 — pass | 0.998 — pass | **FAIL** |

Both errors are under one tolerance unit on HIRES (0.26 vs 0.41). The 1.5x criterion is applied as
preregistered.

## Why it fails (observation from a temporary, uncommitted iteration trace; status: interpretation)

Most refreshes are budget refreshes (126 of 134 on HIRES, 62 of 70 on Brusselator). The trace on HIRES
shows the cause. The stage values are about 1e5 tolerance units, the first preconditioned residual
WRMS(P⁻¹r₁) is 1e3–1e4 tolerance units, and the target is ρ = 0.0125 (or 0.002 for stages 3–6), so a
reduction of about 1e-6 is needed. The observed contraction is q ≈ 0.10–0.20 at the usual step growth
h/h_P ≈ 1.08–1.18; this matches Corollary 2a, where the nonstiff components of `I/(h_Pγ) − J` against
`I/(hγ) − J` give T ≈ 1 − h_P/h. Six iterations at q ≈ 0.13 give about 5e-6, just short, so the stage hits the
6-iteration budget and refreshes. During step-growth phases (h/h_P ≈ 3.6–5) q ≈ 0.8 and the contraction
rule refreshes after two iterations. The method's order is not the issue: accepted steps match the fast
driver to within 0.5%.

Untested hypotheses for later cards (not tried here, so as not to tune this gate): a starting iterate
better than P⁻¹c (e.g. extrapolated from the previous step's stages, which shrinks R₀ rather than q);
preconditioned GMRES on each stage (the plan's own preference in T1/T9) instead of Richardson; a
step-size-normalised preconditioner.

## Deviations from the brief

* **WRMS weights** are atol + rtol|yₙ|. The step's scale atol + rtol·max(|yₙ|,|y_new|) is unknown during
  the stages, and these weights are componentwise no larger, so the test is never looser.
* **Order of tests in an iteration:** convergence is checked before the contraction rule. On convergence
  the final correction (already computed) is applied.
* "Brusselator 1D (n = 100)" = `brusselator_problem(50)` (n = 2·cells = 100), the benchmark's
  `brusselator-1d-50`.
* The fast reference arm and the lagged arm use dense storage. On these problems the fast driver picks the
  in-place zero-skipping LU, so both arms share it.
* **Build isolation:** the shared `CARGO_TARGET_DIR` mixes the five worktrees' `rodas5p-integrators`
  artifacts (cargo hashes workspace path packages by relative path). Twice, my test target was linked
  against another node's library, which lacked the new symbols (E0432). All recorded runs therefore
  add `--config profile.dev.package.rodas5p-integrators.codegen-units=255`. This gives this crate its own
  metadata hash, leaves floating-point semantics unchanged, and shares the dependency artifacts. The env
  variables are exactly the controller's.
