# Node integration (wave 1, final-review fix wave)

Branch `production/wave1-integration`, from `15eb5f6`. Scope: the items of
`.superpowers/sdd/IMPLEMENTATION_WAVE1/final-review.md` assigned to the fixer (1-7, 11, the coupled
validation message, and a facade `atol = 0` test), under the controller rulings at the end of
`progress.md`.

Toolchain: `RUSTUP_TOOLCHAIN=stable` (cargo 1.97.0; `rust-toolchain.toml` pins 1.94.1, not
downloadable, not edited). Env: `CARGO_TARGET_DIR=/home/claude/targets/integration
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0`, `-j 2`, `--locked`.

## Commits

| SHA | Item | Subject |
| --- | --- | --- |
| 58acfeb | 5 | fix(core): keep wrms strict; add step_error_wrms for end-of-step norms |
| 9e535e5 | 1, 2 | fix(lagged): zero-scale rule in the error norm; refuse a zero Richardson weight |
| 7d952ae | facade | test(vigilode): atol = 0 with an identically zero component on dense and JVP routes |
| 84b5105 | 3, 4, 6 | fix(dense): refuse W-filtered control without an explicit Jacobian; reconcile homotopy dense counters |
| 3e450ad | 4 | fix(integrate): reconcile homotopy outer rejections in the observed driver |
| 1e3eb0c | 6 | test(lagged): exercise the JVP branch of apply_w inside iterating stages |
| 293d8c4 | 7 | refactor(fast): FixedWidth2 banded driver ids as pub const |
| e393e3a | 11 | fix(lagged): bind stage sensitivities to t-verify RESULTS.json; a_7 = 1.02 |
| 148b515 | coupled msg | fix(coupled): validation messages say what each tolerance check allows |

## What changed

- **Item 5 (ruling: keep `wrms` strict).** `rodas5p_core::wrms` is back to its pre-wave contract
  (zero scale -> typed `InvalidInput` "WRMS scale must be finite and positive"). New public
  `rodas5p_core::step_error_wrms` carries the zero-scale rule (`scaled_error_component`). Positive
  scales are bit-identical in both (pinned in `core_contracts`). Sites switched to
  `step_error_wrms`: `sequential.rs` `finish_step` and its twin `embedded_error_norm` (the
  protected driver, and through it the coupled fallback, `fallback.rs:110`), and
  `dense_output_v2.rs` `rodas5p_dense_wfiltered_interior_error_estimate` (acceptance norm under
  `WFilteredEnforce`). Inline end-of-step loops keep `scaled_error_component` (fast, small,
  small-batch, banded, MF fast, lagged, coupled solver): they allocate no scale vector.
  Callers on strict `wrms` (audit list): `rodas5p-krylov/src/common.rs:52`; `nonlinear.rs:138`
  (Newton convergence, Radau/BDF comparators); `sequential.rs:491-492` (inner forcing);
  `sabr.rs:159,198,294`; `certification.rs:204,205,335,359`; `homotopy.rs:155,157,1663`
  (output certificate); `transactional_q1_q2.rs:1002,1015`; `radau.rs:546,563`; `bdf.rs:922`;
  `adaptive.rs:909` (step doubling); `adaptive_exponential.rs:288`; `exponential.rs:2204`;
  `dense_output_v2.rs` Report/Enforce interior estimate, defect estimate and interpolant audit;
  `path_controller.rs:506`, `g1_transactional_gate.rs:206`, `homotopy_experiments.rs:180`,
  `unified_screen.rs:498,721`, `g4_s5b0_regime_atlas.rs`, fair-ab examples. These return to
  pre-wave behaviour: `atol = 0` with a zero component is a typed error on those paths.
- **Item 1.** Lagged end-of-step norm uses `scaled_error_component` (as the fast driver).
- **Item 2 (ruling).** A zero Richardson weight `atol + rtol |y_n|` on an iterating lagged stage
  returns `CoreError::InvalidInput` from the run (input refusal; not a `NonFinite` rejection).
  Documented in the module docs as the second start-state-weight exception, next to the staged
  matrix-free one (`rodas5p_matrix_free_fast.rs:2670`). With `refresh_every_attempt` no stage
  iterates and `atol = 0` works, bit-identical to the fast driver.
- **Item 3.** `WFilteredReport`/`WFilteredEnforce` refuse a problem without an explicit Jacobian
  (`!has_explicit_jacobian()`) up front with `InvalidInput`; doc of the variant updated.
- **Item 4 (ruling withdrawn).** Both homotopy drivers call `reconcile_outer_rejection`. Through
  the public API the outer gate `error_norm + output_wrms <= 1` equals the kernel's certificate
  gate `embedded + output_wrms <= 1` up to summation-order roundoff, so an outer-only rejection
  cannot be forced; the new tests (`a1_homotopy_counter_reconciliation.rs`) pin the ledger on
  runs with rejections and passed before and after the fix.
- **Item 6.** `lagged_jvp_branch_runs_inside_iterating_stages` (HIRES + JVP, default options:
  `richardson_iterations > 0`, all products through the JVP);
  `wfiltered_enforce_interior_rejections_keep_the_counter_ledger` (PR lambda = -1e5,
  rtol 1e-8, direct: interior-forced rejections > 0, counters == ledger; passed on arrival,
  since the reconcile call was already present).
- **Item 7.** `RODAS5P_FAST_BANDED_FIXED2_DRIVER_ID`,
  `RODAS5P_FAST_BANDED_FIXED2_EXACT_CACHE_DRIVER_ID` (values unchanged), re-exported.
- **Item 11.** Chose to round up: a_7 = 1.02 (verified 1.0109942639901013); every constant is
  `ceil(100 verified) / 100` and >= the verified value; test reads RESULTS.json.
- **Coupled message.** "atol must be finite and nonnegative" / "rtol and the solver tolerances
  must be finite and positive".
- **Facade.** `zero_atol_with_an_identically_zero_component_succeeds_on_dense_and_jvp_routes`.

## TDD record

RED before GREEN: lagged forced-refresh `atol = 0` (run failed), lagged zero-weight refusal (got
13 rejections, 6 NonFinite), core `step_error_wrms` (did not compile), W-filtered JVP-only
refusal (ran with 1 unavailable sample), a_7 binding (1.01 < 1.01099), coupled messages.
Passed on arrival (coverage pins): facade `atol = 0`, homotopy ledgers, W-filtered interior
counter sum, lagged JVP branch, driver-id constants.

## Test runs

- `cargo test -j 2 --workspace --locked --no-fail-fast`: exit 101; 1484 passed, 4 failed,
  109 ignored (302 result lines). Failures are the known scheduling-dependent targets:
  `rodas5p-core --test leja_setup_work_contract` (1), `rodas5p-integrators --test
  collective_admission_contracts` (1), `--test production_audit_pr101_provider` (2). Rerun with
  `--test-threads=1`: 5/5, 7/7, 9/9 passed.
- `crates/vigilode-coupled`: `cargo test -j 2 --locked --no-fail-fast` exit 0, 37 passed.
- `crates/vigilode-hybrid`: exit 0, 19 passed.
- `cargo clippy --all-targets` on rodas5p-core, rodas5p-integrators, vigilode and
  vigilode-coupled: no warnings in touched lines.
- Note: Task 6 recorded "1530 passed / 3 failed"; this run counts 1484 + 4 from the
  `test result` lines. Not reconciled here (all eight new test names ran).
