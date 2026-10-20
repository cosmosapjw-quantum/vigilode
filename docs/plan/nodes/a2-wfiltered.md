# Node a2-wfiltered: W-filtered interior error control

Plan section 5 T3, gate A2 of section 6. Branch `production/a2-wfiltered` from `9b5d083`.

**Kill gate result: FAIL** (step count). The interior-error half of the gate passes.

## What was done

- `DenseErrorControl::WFilteredReport` and `DenseErrorControl::WFilteredEnforce` in
  `crates/rodas5p-integrators/src/dense_output_v2.rs`.
- Estimate E = (M − hγJ₀)⁻¹ (h/2) d(1/2), with d the existing midpoint defect of `scaled_midpoint_defect`,
  J₀ the Jacobian of the step itself and γ = 0.21193756319429014 from `rodas5p_coefficients()`.
- The factorization comes from the step. `sequential.rs` gains a crate-internal
  `sequential_step_capturing_iteration_matrix`, which runs the same body as `sequential_step` (through a shared
  `sequential_step_impl`) and keeps a copy of the step's LU of W when the stage solves made one (Direct method or
  Direct preconditioner). Otherwise it keeps the step's explicit W, which the estimator factorizes once per
  sampled step and records in `direct_factorizations`. No new Jacobian is evaluated in either case. `StepResult`
  is unchanged.
- `WFilteredReport` records samples and never changes the run; its work goes to the report counters only.
  `WFilteredEnforce` passes max(embedded error, estimate) to the normal controller for an accepted step that
  contains an interior output time. An estimate above one rejects the step through the controller, and an estimate
  below one still shrinks the next step. Its work is charged to the run counters as well.
- The W-filtered modes refuse `IntegrationMethod::Sabr` (InvalidInput), since SABR does not expose its stage
  matrix and the only alternative would be a new Jacobian. A matrix-free Jacobian (no explicit W) counts the step as
  `unavailable`, as the existing filtered modes do.
- New public helpers: `rodas5p_wfiltered_iteration_matrix` (M − hγJ formed as `ShiftedOperator` forms it) and
  `rodas5p_dense_wfiltered_interior_error_estimate` (one RHS, one back-solve, counted as one `linear_solves` and
  one `direct_solve_calls`).
- `Off`, `ReportDefect`, `Report` and `Enforce` are unchanged.

## Measured numbers

Status: diagnostic counts and errors in WRMS tolerance units, deterministic. No wall times.
Toolchain: `RUSTUP_TOOLCHAIN=stable` (cargo 1.97.0). The pinned 1.94.1 is not available here.

(a) Lemma 3. On y' = λy with a constant defect, the ratio R = estimate / true interior error at θ = 1/2:

| z = hλ | −1e-3 | −0.1 | −1 | −10 | −1e3 | −1e6 |
|---|---:|---:|---:|---:|---:|---:|
| R | 1.000038 | 1.003931 | 1.048525 | 1.613758 | 2.348106 | 2.359174 |

All values lie in [1, 1/(2γ)] = [1, 2.3592], and R is monotone. The estimator uses the library's filter matrix.

(b) Prothero–Robinson, λ = −1e5, t ∈ [0, 2], 201 outputs, atol = 1e-2·rtol, Direct linear solver:

| rtol | interior Off / Enforce / WFilteredEnforce | accepted steps | attempts | Jacobians | LU |
|---|---|---|---|---|---|
| 1e-4 | 3.235 / 0.921 / 0.373 | 7 / 7 / 8 | 7 / 8 / 8 | 7 / 14 / 8 | 7 / 14 / 8 |
| 1e-6 | 311.467 / 1.052 / 0.350 | 7 / 12 / 15 | 7 / 26 / 15 | 7 / 50 / 15 | 7 / 50 / 15 |
| 1e-8 | 208.636 / 1.381 / 0.351 | 11 / 31 / 39 | 11 / 83 / 39 | 11 / 164 / 39 | 11 / 164 / 39 |

The Off and Enforce columns reproduce `docs/DENSE_OUTPUT_ERROR.md` (3.2/312/209 and 0.92/1.05/1.38).
WFilteredEnforce rejected no steps on this problem. With the Direct solver the step's own LU is reused, so the
estimate adds no factorization: the Jacobian and LU counts equal the number of attempts.

Kill gate (preregistered, plan §6 A2, thresholds unchanged): it fails if the interior error exceeds 2 units at
rtol 1e-6, or if the accepted steps exceed 1.3x the `Off` count.
- Interior at 1e-6: 0.350 ≤ 2, **pass**.
- Steps at 1e-6: 15 / 7 = **2.143x > 1.3x, FAIL**. For reference, the ratio is 1.143x at 1e-4 and 3.545x at 1e-8.

(c) DP-01 nonlinear witness, y' = −1e6(y − sin t) + cos t + 10(y − sin t)², [0, 1], rtol 1e-8, atol 1e-11,
h₀ 1e-3, 101 outputs, GMRES with no preconditioner (the configuration of the audit probe):

| mode | max normalized interior error | accepted / rejected | RHS | Jacobians | LU | samples |
|---|---:|---|---:|---:|---:|---:|
| Off | 1801.6160 | 6 / 0 | 48 | 6 | 0 | 0 |
| Enforce | 1.4986 | 17 / 24 | 366 | 79 | 38 | 38 |
| WFilteredEnforce | 0.3350 | 21 / 0 | 187 | 21 | 19 | 19 |

Off and Enforce reproduce the DEFAULT_PATH_KO.md DP-01 record exactly. GMRES holds no LU, so the W-filtered
estimate factorizes W once per sampled step (19 LU). It evaluates no Jacobian beyond the 21 that the steps
already made.

## Commands (exit codes)

All runs used `RUSTUP_TOOLCHAIN=stable CARGO_TARGET_DIR=/home/claude/targets/shared CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0`, `-j 1` and `--locked`.

- RED: `cargo test -p rodas5p-integrators --test dense_wfiltered_contracts` failed to compile (exit 101, missing
  variants and helper).
- GREEN, one invocation: `cargo test -p rodas5p-integrators --test dense_wfiltered_contracts --test
  dense_error_control_contracts --test audit0930_accounting_contracts --test dense_output_v2_contracts` exited 0
  with 8 passed + 1 ignored, 4, 1 and 15 passed.
- `--include-ignored` on `dense_wfiltered_contracts`: exit 101. The only failure is the ignored kill-gate test
  (`2.143x the Off accepted-step count > 1.3x`).
- `cargo clippy -p rodas5p-integrators --all-targets --locked` exited 0 with no warnings in the touched files.
  `rustfmt --check` is clean on the touched files.
- `cargo test -p rodas5p-integrators --lib --test dense_wfiltered_contracts --test dense_error_control_contracts`:
  exit 0; lib 62 passed + 1 ignored, 4 passed, 8 passed + 1 ignored. The first attempt hit a stale artifact
  from another worktree and was rerun after `touch`.

## Deviations and notes

- The step gate counts accepted steps, which is what (b) names. Attempts give the same verdict (15 / 7).
  Plan §6 states the gate as "> 1.3x the protected step count", and the brief reads it against `Off`. Against
  `Enforce` the ratios would be 1.25x accepted and 0.58x attempts. The gate was scored as briefed, against `Off`.
- The estimate is formed only for steps that the embedded estimate accepts and that contain an interior output
  time, the same sampling rule as `Report`/`Enforce`. A step rejected by the embedded estimate is rejected anyway.
- Shared-target hazard: workspace path crates hash identically across worktrees, so another node's build of
  `rodas5p-integrators` can satisfy this node's freshness check. Once, a test binary failed to compile against another
  node's lib. Before each evidence run the touched sources were `touch`ed, and every target was built and run in one
  cargo invocation, which showed `Compiling rodas5p-integrators (/home/claude/wt/a2-wfiltered/...)`. The Task 6
  merge run should be aware of this.
