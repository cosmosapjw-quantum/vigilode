# Final whole-branch review — VigilODE production wave 1

Range: `9b5d083..15eb5f6` on `production/wave1-integration` (27 commits, 47 files, +7064/−76).
Reviewer seat: final whole-branch review (read-only; no suite rerun; no subagents).
Passes made: (1) core zero-scale rule and every driver call site; (2) `rodas5p_fast.rs` band port and
visibility changes; (3) `dense_output_v2.rs` (counter reconciliation + W-filtered modes) and
`sequential.rs` capture path; (4) `rodas5p_lagged.rs` in full; (5) `vigilode` facade (`solve.rs`,
`solution.rs`, `fd.rs`, `options.rs`, `error.rs`, `lib.rs`); (6) `vigilode-coupled` diffs;
(7) t-verify node report, ledger row L-0137 and RESULTS.json keys; (8) ledger rulings and deferred
minors; (9) integration test log failures. Test files were scanned for coverage, not read line by line.

## Strengths

- The zero-scale rule is implemented once (`rodas5p-core/src/norms.rs:37-44`) with a unit test that pins
  bit identity for positive scales (`to_bits` of `e / s`), both zero signs and NaN. Every call site in
  the dense, small, batch, matrix-free unstaged and coupled drivers keeps `e / s` bit-for-bit when the
  scale is positive, so default runs are unchanged.
- The band-solve port keeps default kernel dispatch identical (`kernel != Indexed` covers exactly the old
  `Slices` branch) and the factorization counter is incremented in the same order as before
  (`rodas5p_fast.rs:509-528`). Checked size arithmetic and NaN/Inf rejection in
  `rodas5p_fast_banded_solve` are correct.
- `reconcile_outer_rejection` is now called in both transactional drivers and in the dense control loop
  after the W-filtered decision (`dense_output_v2.rs:1757`), so a W-filtered interior rejection is
  reconciled through the existing path rather than a new one.
- W-filtered modes are opt-in variants; `Off/ReportDefect/Report/Enforce` take the old code path; SABR is
  refused up front; the estimate is documented as "an estimate, not a bound".
- The lagged driver reuses the fast workspace, factor, solve and coefficient tables, charges refreshes
  before factoring (failed refreshes stay counted), keeps abandoned iterations, and the forced-refresh arm
  reproduces the fast driver. Kill gates for A2 and B1 were scored as preregistered and not loosened.
- The facade refuses (typed `Unsupported`) rather than substitutes: vector atol, mass, dense output, band
  on JVP-only. `Status::Success` is explicitly not a certificate. `Tier::Performance` records requested
  vs executed tier. The guard never performs arithmetic on callback values.
- t-verify is clean: preregistration committed before the run, byte-reproducible RESULTS.json, and
  corrections to the plan recorded in the node report rather than edited into the plan.

## Issues

### Critical

None. No default-path bit-identity break found; every behavioural change on default entry points is on
inputs that previously errored, NaN-looped or were the named counter defect.

### Important (fix before the wave is called done)

1. **Lagged driver bypasses the zero-scale rule in its error norm** —
   `crates/rodas5p-integrators/src/rodas5p_lagged.rs:400-409`.
   The end-of-step norm is a copy of the *pre-A1* fast-driver loop (`e / (atol + rtol max(|y|,|y_new|))`).
   b1-lagged branched from base in parallel with a1-defects, so the merge left it on the old arithmetic.
   With `atol = 0` and a component that is zero at both ends, `0/0 = NaN -> INFINITY`, every attempt is a
   `LocalError` rejection and the run fails, while `integrate_rodas5p_fast_observed` now succeeds on the
   same problem. This breaks (a) the Task 1 rule "call it from every driver" and (b) the B1 module claim
   that a forced refresh "performs the fast driver's arithmetic". Fix: call
   `rodas5p_core::scaled_error_component` (one line) and add the lagged driver to
   `tests/a1_zero_tolerance_scale.rs` (default options and `refresh_every_attempt`).

2. **Lagged Richardson WRMS has a third zero-weight behaviour** — `rodas5p_lagged.rs:225-232, 263-267`.
   Weights `atol + rtol |y_n|` are start-state weights, the same kind as the staged-MF Krylov weights that
   the ledger ruling keeps as a typed refusal. Here a zero weight gives `x / 0` (NaN or Inf) and the
   attempt is returned as `CoreError::NonFinite`, counted as a `nonfinite_step_failures` rejection (a
   misclassification: nothing was non-finite), with `valid = false` forcing a refresh next attempt.
   The ruling's own logic applies; pick one: typed refusal at entry (as staged MF) or treat a zero-weight
   component by `scaled_error_component` and map an infinite norm to `Refresh::Contraction` instead of
   an error. Either way record it next to the staged-MF exception in the docs.

3. **W-filtered modes silently degrade to `Off` on JVP-only problems** —
   `dense_output_v2.rs:1561-1571` (only SABR refused) and `:1747` (`Err(_) => unavailable += 1`).
   With a matrix-free Jacobian the capture has neither `factor` nor `matrix`
   (`wfiltered_estimate_from_step` returns `InvalidInput`), every sample is "unavailable", and
   `WFilteredEnforce` runs exactly like `Off` with no error. The doc says "Sequential method with an
   explicit Jacobian only" but nothing enforces it. A user who asked for interior enforcement gets none.
   Refuse up front (same place as SABR) when the problem has no explicit Jacobian, and add a test.

4. **Homotopy counter drift left open; challenge the ruling** — `integrate.rs:705-707` and
   `dense_output_v2.rs:2404-2406`. Both homotopy drivers have exactly the transactional pattern
   (kernel verdict, then `error <= 1.0` outer gate) without `reconcile_outer_rejection`. The ruling
   defers them as "not named in the brief", but spec §6 names them explicitly ("the same pattern in the
   dense variant, legacy SABR and homotopy drivers"), and the fix is the same one line per site that
   a1-defects already applied twice. Fix now with a counter-sum test, or rewrite the ruling to cite the
   spec and open the wave-2 card; "not in the brief" is not a reason when the spec lists it.

5. **The zero-scale rule changed `wrms` for iteration-convergence callers too; challenge the ruling's
   scope** — `rodas5p-core/src/norms.rs:93-97`.
   Changing `error_scale`/`wrms` (as spec §6 asked: "one rule, in `error_scale`") also changes every
   non-error-norm user: Newton convergence (`nonlinear.rs:137`, Radau/BDF comparators), inner forcing
   (`sequential.rs:490-492`), SABR residual estimates (`sabr.rs:159,198`), certification
   (`certification.rs:203-205,335`). These used to return a typed error on a zero scale; now a nonzero
   residual on a zero-scale component gives `+inf` (Newton never converges -> step loop; inner forcing ->
   `InvalidInput` from `rodas5p_inner_forcing_target` with a different message). The staged-MF ruling
   argues correctly that the rule is wrong for iteration tests, but it was applied to one site only.
   Either audit these callers and pin each with a test, or keep `wrms` strict and expose the rule as a
   separate error-norm function used only by end-of-step norms. At minimum record the list.

6. **Untested paths on the cross-node seams (deferred minors to promote)**:
   - B1 "JVP-path test passes with zero iterations": the JVP branch of `apply_w`
     (`rodas5p_lagged.rs:204-209`) is never exercised by an iterating stage. Add a case with a JVP and
     a lagged P (h change after rejection) that takes >= 1 Richardson iteration.
   - A2 I-1: no test drives a `WFilteredEnforce` interior rejection and checks
     `accepted + rejected == attempts` after `reconcile_outer_rejection`. This is the only new rejection
     source in the wave; it should have the counter-sum test the a1 transactional fix got.

### Minor (defer with a card unless noted)

7. **Driver ids as literals** — `rodas5p_fast.rs:1644-1647`. `"rodas5p-fast-banded-fixed2-v1"` and
   `"…-exact-cache-v1"` are record keys (they land in `Stats::route.driver_id`) but are not `pub const`
   like `RODAS5P_FAST_BANDED_SLICES_DRIVER_ID`. Cheap; fix in this wave.
8. **Public enums grew without `#[non_exhaustive]`** — `rodas5p_fast.rs:175` (`BandedKernel`),
   `dense_output_v2.rs:1057` (`DenseErrorControl`). A §6 A1 acceptance item is `cargo semver-checks`
   clean; adding variants to exhaustive public enums is a major change. Acceptable pre-1.0 inside the
   workspace, but the facade is where stability is promised; decide before release.
9. **Facade leaks internal types into its public `Stats`** — `vigilode/src/solution.rs:5-6,233-273`
   (`AdaptiveStepConfig`, `WorkCounters`, `LinearSolverConfig`, `BandedWork`, `Rodas5pFastLu`,
   `RoutingRecord`). Any change in `rodas5p-*` becomes a facade semver change. Wave-2 card.
10. **`FailureReason::StepSizeTooSmall` is the catch-all** — `vigilode/src/solve.rs:210-212`. A run that
    stopped on repeated linear-solve or non-finite failures is reported as a step-size failure; the
    driver counters to distinguish them are in `Stats`. Defer with the partial-solution card.
11. **B1 constants not bound to t-verify output** — `rodas5p_lagged.rs:83-84`. a_7 = 1.01 while
    RESULTS.json gives 1.01099 (the only constant rounded down; effect 0.1% on rho_7). Add a test that
    reads the constants against RESULTS.json (this is the deferred "no cross-check" minor).
12. **`Rodas5pLaggedResult` undocumented and not `Serialize`** — `rodas5p_lagged.rs:134-140`, while
    `LaggedWork` is. Matters only when the facade attaches it.
13. **W-filtered capture clones the LU on every trial step** — `sequential.rs:452-454`. Opt-in only;
    conflicts with the P0 zero-allocation-per-step promise if W-filtered ever becomes default.
14. **Coupled validation message** — `vigilode-coupled/src/solver.rs:474-476` says "positive" for a
    check that now allows `atol = 0`.
15. **Roundoff fragility of "exactly zero" (plan-level note)**. All a1 zero-scale regression tests use a
    decoupled zero component. When other rows depend on the zero component and partial pivoting swaps
    rows across it, the stage solve can return a roundoff-level nonzero in that component, which the rule
    turns into `+inf`. This is inherent in `atol = 0` (the scale then becomes `rtol * roundoff`), not a
    defect of the rule, but the docs should say that `atol = 0` is supported only for components that
    stay exactly zero in floating point; one coupled regression test would show the actual behaviour.

## Cross-node questions asked in the brief

- **Zero-scale rule vs W-filtered estimator**: consistent. The estimator uses `error_scale` + `wrms`, so it
  follows the rule; an identically zero component gives `d_i = 0`, so the estimate is finite unless the
  back-solve introduces roundoff (item 15). Under `WFilteredEnforce` an infinite estimate rejects via the
  controller, as specified.
- **Zero-scale rule vs lagged WRMS**: inconsistent (items 1, 2).
- **Should the facade route anything new?** No. A2 and B1 both failed their kill gates, so neither may be
  attached to `Tier::Performance`; the FixedWidth2 kernel stays opt-in because the brief kept default
  kernel selection. Two follow-ups: (a) Task 5's brief text "wave 2 attaches the lagged driver" is now
  contradicted by the B1 FAIL; add a ruling mirroring the A2 one (no attachment without a new B1 card);
  (b) the facade already accepts `atol = 0`, so it now inherits the rule — add one facade contract test
  with `atol = 0` on the dense and JVP routes.
- **Counters and statuses**: refresh, abandoned-iteration and W-filtered work are charged to the run
  where the plan says so; `Report` keeps work in the side report, `Enforce`/`WFilteredEnforce` charge the
  run. Estimates are labelled estimates everywhere (`DenseErrorSample.estimate_wrms`, `q_k`,
  `Status::Success`). The open counter defects are items 2 (misclassified failure kind) and 4 (homotopy).
- **Default bit-identity**: not broken. Visibility changes in `rodas5p_fast.rs` are arithmetic-free;
  `scaled_error_component` returns `e / s` for `s > 0`; band dispatch is equivalent; the coupled eager
  path removes only the stage-0 JVP that was never read (the crate's eager/residual-first bit tests pass).

## Triage of deferred minors in the ledger

| Origin | Item | Decision |
|---|---|---|
| T1 | residual-first stage-0 JVPs | defer |
| T1 | FixedWidth2 id literals | **fix now** (item 7) |
| T1 | MF zero-scale test only unstaged | resolved (staged refusal test at `a1_zero_tolerance_scale.rs:240`) |
| T1 | two scheduling-dependent failures | defer (ruling accepted; see below) |
| T1 | `fallback.rs:110` receives `atol = 0` | defer, but fold into item 5's audit list |
| T2 I-1 | no interior-rejection counter test | **fix now** (item 6) |
| T2 I-2 | Lemma-3 bound cannot tell hγ from h/2 | defer |
| T2 I-3 | LU clone per trial step | defer (item 13) |
| T2 I-4 | two extra public helpers | defer; decide with item 8 |
| T3 | WRMS weights `|y_n|` only | defer (documented, conservative) |
| T3 | 6th correction not credited | defer; document the convention |
| T3 | JVP-path test with zero iterations | **fix now** (item 6) |
| T3 | contraction ratio skips x0 | defer; document |
| T3 | `Rodas5pLaggedResult` undocumented / not Serialize | defer (item 12) |
| T3 | codegen-units workaround in recorded runs | defer (diagnostic numbers only) |
| T4 | dead vars, claim-2 min at grid end, Cor 2a grid-only, cosmetic ref | defer |
| T4 | no cross-check vs stage-transfer constants | defer as a test (item 11) |
| T5 | StepSizeTooSmall catch-all | defer (item 10) |
| T5 | Stats not Deserialize | defer; "reproducible from its record" needs it before release |
| T5 | BandJacobianFn zeroed-buffer doc | defer |
| T5 | no Fatal/Recoverable tests for ft/J/JVP callbacks | defer |
| T5 | clippy pre-existing count unexplained | defer |

## Rulings challenged

- **Homotopy counter drift deferred as "not named in the brief"** — wrong basis; spec §6 names it (item 4).
- **Staged-MF zero-weight exception** — the reasoning is sound, but it was applied to one site; the
  lagged driver (item 2) and the iteration-convergence users of `wrms` (item 5) are the same case and got
  different behaviour. Widen the ruling or narrow the core change.
- **Three allocation-counter failures are pre-existing** — accepted with a caveat: only two were shown
  failing on base; `collective_admission_contracts::test_vg1_zero_allocation_in_rhs_and_jvp_hot_loops`
  was not. Wave 1 does not touch `collective_model.rs`, and all three use a process-global allocation
  counter, so the ruling is very likely right; one base run of that test under default threading would
  close it.
- **check-research-node exit 1 (L-0124)** — accepted; it is identical on base. Note that the ledger row
  L-0137 says PASS while the checker job stays red on this branch until the tombstone card lands.
- Other rulings (toolchain, per-node push, plan text cleanup, shared/per-node target dirs, 1e-12 vs bit
  identity for B1, T2 gate against Off, stiff-example comparator, partial solution at t0 only,
  mass/dense refusal) — accepted.

## Declined to judge

- Performance numbers in node reports (HIRES/Brusselator/PR step ratios): diagnostic on a shared 2-core
  host per the global constraints; I checked only that gates are reported as preregistered.
- Line-by-line correctness of the ported FixedWidth2 kernel against PR #73: ported tests
  (`banded_exact_tests.rs`, `banded_recovery_contract.rs`) cover it; I checked dispatch and counters only.
- verify_theory.py mathematics: outside a code review of the Rust branch; I checked only consistency of
  its outputs with the constants B1 and A2 use.
- MSRV 1.94.1 compatibility: cannot be checked with the available toolchain (ruling).
- Whether atol = 0 should be supported at all in the protected tier: a product decision in the plan.

## Assessment

**Ready to merge: With fixes.**

The default path is intact and the five nodes integrate cleanly; the defects are on the seams the
per-task reviews could not see: the lagged driver missed the zero-scale rule it was developed alongside,
W-filtered enforcement can silently turn itself off, and two counter-drift sites named in the spec remain.
Items 1–4 and 6 are small (each a few lines plus a test) and should land before the wave is called done;
item 5 needs at least a recorded audit list.
