# Diffsol 0.16.1 triage comparator

An isolated harness that produces the external rows `ext.diffsol-bdf`, `ext.diffsol-esdirk34` and
`ext.diffsol-tr-bdf2` of the component-triage campaign (`research/component_triage_20261019`). Its rows use the
contract row schema `vigilode-triage-row-v1` (`HARNESS_CONTRACT.md`). It replaces the old Diffsol harness
(`tools/benchmarks/diffsol/main.rs`) for this campaign, and it fixes that harness's defects PA-04 to PA-07 (see
below). The a07 Robertson harness next to it is unchanged.

- **Standalone.** The crate has its own `[workspace]` and its own committed `Cargo.lock`. It depends on no
  crate of this repository: the nine problem models are separate Rust transcriptions, and `parity` checks them
  bit for bit against the Rust registry.
- **Exact pins.** `diffsol =0.16.1` with only the `faer` backend, as in a07. Also `diffsol-la =0.1.0`, the
  linear-algebra crate that diffsol 0.16.1 itself uses, with the same version and unified features. It is
  needed only for the delegating LU counter, because diffsol does not re-export its `LinearSolver` trait. The
  lockfile was seeded from `../a07/Cargo.lock` and pruned by cargo, so every shared dependency has the a07
  version.
- **No timing.** Nothing is timed and no wall time is recorded.

## Build

From this directory:

```sh
cargo fetch --locked                       # once, needs crates.io; afterwards everything is offline
cargo build --release --locked --offline   # -> target/release/diffsol-triage
```

`CARGO_TARGET_DIR` may point elsewhere. The release profile is `opt-level 3`, fat LTO, one codegen unit and
`panic = "abort"`. A panic inside Diffsol therefore aborts the process, and the Python driver records it as a
`process_abort` row.

## Commands

```sh
B=target/release/diffsol-triage
$B list                                    # problems, methods (vigilode-diffsol-triage-list-v1)
$B run --problem robertson --method bdf --rtol 1e-06 --grid uniform101 --campaign ct-20261019-main [--repetitions K]
rodas5p triage-parity --problem robertson > R.json && $B parity --input R.json
```

The exit status is 0 whenever a row or document was printed, failed integrations included. It is 2 for a usage
error: an unknown problem, method or grid, or a bad rtol.

The campaign drivers call the binary themselves (`tools/triage/README.md`):

```sh
N=research/component_triage_20261019
python3 tools/triage/triage_external.py --corpus $N/CORPUS.json --campaign $C --output $N/EXTERNAL.jsonl \
    --diffsol-binary $B [--external-arm ext.diffsol-bdf ...] [--diffsol-timeout 3600] [-j N]
python3 tools/triage/triage_parity.py --binary BIN --corpus $N/CORPUS.json --output $N/PARITY.json \
    --diffsol-binary $B [--diffsol-only]
```

`--diffsol-only` skips the NumPy models, so it runs without SciPy.

## Problems

The nine problems are transcribed from `crates/rodas5p-cli/src/triage.rs` (`PROBLEMS`) and the builders it
calls. They keep the same equations, parameters, y0, t_span and `atol_scale`, and they have no breakpoints or
segments. The right-hand sides keep the operation order of the Rust closures.

| id | n | t_span | atol_scale | J v given to Diffsol |
|---|---|---|---|---|
| robertson | 3 | [0, 40] | 1e-4 | copy of the Rust analytic JVP (`problems.rs::robertson_problem`) |
| hires | 8 | [0, 321.8122] | 1e-4 | sum_k J_ik v_k over `stiff_benchmark.rs::hires_fill` |
| van-der-pol-mu1000 | 2 | [0, 2000] | 1 | copy of the Rust JVP (`stiff_van_der_pol_problem(1e3)`) |
| oregonator | 3 | [0, 360] | 0.010000000000000002 | copy of the Rust JVP (`oregonator_holdout`, JVP only) |
| pollution | 20 | [0, 60] | 0.010000000000000002 | copy of the Rust JVP (`pollution_holdout`, JVP only) |
| prothero-robinson-nl | 1 | [0, 1] | 1e-3 | copy of the Rust JVP (`prothero_robinson_problem(-1e6, 10, 0)`) |
| scalar-linear | 1 | [0, 1e-2] | 1e-3 | copy of the Rust JVP (`scalar_linear_problem(-1e4, 1)`) |
| brusselator-1d-50 | 100 | [0, 10] | 1 | sum_k J_ik v_k over `stiff_benchmark.rs::brusselator_problem` `fill` |
| brusselator-1d-250 | 500 | [0, 10] | 1 | same, 250 cells |

- **Jacobian.** Diffsol takes the Jacobian as a J v closure (`OdeBuilder::rhs_implicit`). With
  `use_coloring = false` and a dense faer matrix, it assembles the dense J from `n` unit-vector calls. Every
  closure above gives each analytic entry exactly on a unit vector. The Jacobian that the solvers factorize is
  therefore the Rust analytic Jacobian. For the JVP-only problems (Oregonator, Pollution) it is the matrix of
  the Rust JVP.
- **Brusselator.** The model is the one the Rust code implements: `-4u`, `+3u`, and Dirichlet values
  `(u, v) = (1, 3)` at both ends. That is the PA-07 note.
- **atol_scale.** The value for Oregonator and Pollution is computed as `(0.01 * 1e-4) / 1e-4`, as in
  `triage.rs::corpus`.

Verification on 2026-10-19 (`triage_parity.py --diffsol-binary`, limit `THRESHOLDS.parity.max_relative_error` =
1e-13): all nine problems pass with maximum relative error 0.0 for f, J and J v at every sample. Sample 0
equals the Diffsol y0 bit for bit, and t_span and atol_scale equal CORPUS exactly.

## Rows (`run`)

- **Arm and fixed keys.** The arm is `ext.diffsol-<method>`. `components` is `[]`, `counters` is null,
  `counter_semantics` is `external-upstream-raw`, `binary.profile` is `external` and `h0_scale` is 1.0. The
  binary computes `binary.sha256` from its own file. `source` comes from `VIGILODE_SOURCE_COMMIT` and
  `VIGILODE_TREE_CLEAN`.
- **Tolerance.** `atol = rtol * atol_scale` is passed as `OdeBuilder::atol([atol])`, one scalar broadcast to
  every component. This is the same scalar atol as the internal arms. There is no component vector (PA-04).
- **Output.** The output times are the contract grid: `[t0, tf]`, or `t0 + (tf - t0) i / 100` with both ends
  exact (`triage.rs::Grid::times`). The states come from `OdeSolverMethod::solve_dense(&times)`, which returns
  one column per requested time from the solver's own interpolant. The last time is the stop time. The t0
  column is Diffsol's interpolant at t0. It can differ from y0 in the last bit, as for brusselator-1d-250 BDF.
- **Solvers.** The solvers are `problem.bdf::<LS>()`, `problem.esdirk34::<LS>()` and
  `problem.tr_bdf2::<LS>()`. `LS` is `FaerLU<f64>` (faer `FullPivLu`) behind a delegating counter. Every solver
  option keeps its Diffsol default; the row records them in `extra_work.solver_options`.
- **First step.** `h0` is null and `extra_work.h0_policy` is `solver_default`. Diffsol computes the first step
  itself (`StateRefMut::set_step_size`, the Hairer-Nørsett-Wanner heuristic); `OdeBuilder::h0` only sets its
  sign.
- **Step limits.** There is no maximum step. The minimum step is Diffsol's absolute default, 1e-13.
- **status.**
  - `ok`: `solve_dense` reached tf with one column per time.
  - `solver_failure`: `TooManyNonlinearSolverFailures`, `TooManyErrorTestFailures` or `StepSizeTooSmall`.
  - `refused`: `OdeBuilder::build` failed.
  - `error`: any other error, including solver construction.
  - Failed rows keep their statistics. Their `states_hex` is `[]`, because `solve_dense` returns no partial
    matrix.
- **`deterministic`.** It compares all K in-process repetitions bitwise: states and statistics.
- **`extra_work`.** It holds `solver_statistics`, the verbatim `OdeSolverStatistics`, and `rhs_op_statistics`,
  the verbatim `OpStatistics` of the rhs operator. It adds the derived fields below. A quantity that is not
  available is null with a `*_reason`, never 0.

  | field | upstream source |
  |---|---|
  | `nfev` | `rhs_op_statistics.number_of_calls` (every f evaluation, including the two of state initialisation) |
  | `njev` | `rhs_op_statistics.number_of_matrix_evals` (CVODE `nje`; each one is assembled from `n` J v calls, counted in `number_of_jac_muls`) |
  | `nlu` | `solver_statistics.number_of_linear_solver_setups` (CVODE `nsetups`, including the initial one; each is one `FullPivLu`) |
  | `accepted_steps` | `number_of_steps`: Diffsol increments it only after an accepted step (`bdf.rs` `step`, `runge_kutta.rs` `step_accepted`) |
  | `rejected_steps` | `number_of_error_test_failures + number_of_nonlinear_solver_fails`: failed attempts, kept separate from accepted steps (PA-05) |
  | `attempts` | `accepted_steps + rejected_steps` |

  `harness_counters` adds independent counts: the rhs and J v closure calls, and the `FaerLU` factorizations
  and solves through the counter. `mapping_check` records that `nfev`, `number_of_jac_muls` and `nlu` equal
  them. All three checks held in every run so far.

Run check on 2026-10-19 (BDF, rtol 1e-6, uniform101; every row `ok`):

| problem | accepted | rejected | nfev | njev | nlu |
|---|---|---|---|---|---|
| robertson | 168 | 0 | 260 | 4 | 25 |
| hires | 363 | 22 | 884 | 9 | 59 |
| van-der-pol-mu1000 | 817 | 125 | 1777 | 19 | 197 |
| oregonator | 1451 | 175 | 3856 | 45 | 274 |
| pollution | 170 | 6 | 241 | 6 | 41 |
| prothero-robinson-nl | 55 | 0 | 59 | 2 | 16 |
| scalar-linear | 179 | 0 | 193 | 2 | 19 |
| brusselator-1d-50 | 203 | 8 | 292 | 4 | 24 |
| brusselator-1d-250 | 201 | 6 | 300 | 4 | 23 |

## What is and is not comparable

- **Comparable: counted work.** The units of `THRESHOLDS.external.counted_work` are compared: `nfev`, `njev`
  and `nlu` against the internal `rhs_evaluations`, `jacobian_builds` and `direct_factorizations`. The cost of
  one unit is not the same on both sides:
  - Diffsol forms each J from `n` J v calls. These are not counted in `nfev`.
  - Diffsol factorizes with full pivoting.
  - The Newton iterations of the implicit methods are visible only in `number_of_nonlinear_solver_iterations`
    and `harness_counters.lu_solves`.
- **Not comparable: tolerance.** The same `rtol` does not mean the same accuracy. Diffsol uses its own error
  norm (WRMS with `atol + rtol |y_i|`), its own controller and its own first step. Comparisons are made at
  matched error: `triage_check.py` admits a group at target ε only when E_up ≤ ε against `REFERENCES.json`. It
  takes the minimum counted work over the admissible rtol groups. It reports the gap per (problem, grid, ε).
  This harness never judges accuracy.
- **Not comparable: time.** No wall time, Ir or allocations are recorded here. Timing a Diffsol solve needs a
  separate protocol: setup apart from the solve, a fixed process order, and a warm protocol as in a07.
- **Upstream defaults stay.** One example: `max_nonlinear_solver_failures = 50` is checked against the count
  over the whole integration. So ESDIRK34 and TR-BDF2 end with `solver_failure` on van-der-pol-mu1000 and
  oregonator at rtol 1e-6. Those rows are kept as typed failures; the options are not tuned.
- **Coverage.** Problems outside the nine get typed `unavailable` rows from `triage_external.py`, because no
  model is implemented here. These include the mass-matrix problems, the segmented corpus problems and the
  JVP-only large problems.

## How PA-04 to PA-07 are addressed

- **PA-04: timing, output and tolerance contracts differed.**
  - Nothing is timed.
  - The output is `solve_dense` on exactly the cell's grid. `triage_external.py` checks that the times equal
    the requested grid bit for bit, so endpoint cells have two outputs on both sides.
  - The atol is the scalar `rtol * atol_scale`, compared with the cell atol exactly. The old vector atol
    `[rtol×1e-4, rtol×1e-10, rtol×1e-2]` is gone.
  - The problem definitions are bit-checked by `parity`.
  - Accuracy is matched in `triage_check.py`.
- **PA-05: allocation and step metrics were mislabelled.**
  - No allocation estimate exists.
  - The upstream counters are stored verbatim.
  - `accepted_steps = number_of_steps`. Error-test failures and nonlinear failures are not subtracted from it;
    they are reported separately as rejected attempts.
  - `nlu` is the setup counter, cross-checked against an LU factorization count. Nonlinear iterations stay
    under their upstream name and are not called linear solves.
- **PA-06: report and data disagreed, and failures were dropped.**
  - Every cell becomes a typed row, failures included. `triage_external.py` adds `unavailable` and
    `process_abort` rows and never skips a cell.
  - Each row carries the campaign id, the binary sha256 and the source commit.
  - Tables are generated from rows by `triage_check.py` and `triage_report.py`.
- **PA-07: references, physics notes and reproducibility.**
  - There is no self-reference. Errors are computed by `triage_check.py` against the independent references.
  - The Brusselator equations are stated as implemented.
  - The pins are exact, the `Cargo.lock` is committed and `--locked` is used.
  - The harness is a standalone workspace with no path dependencies. Its output goes to stdout and it writes
    no absolute output paths.
