# Review: wave-3 Task 1a, e1b-dense (e9c5bbd..0700689)

### Spec Compliance

- ❌ Issues found: two Important gaps (below). Every other brief requirement has a hunk; none is missing.
  - Arms, ids and `Arm::ALL` = 17: arms.rs diff.
  - Protocol check, `basis`, `interior_control` and the two report sections: harness.rs and report.rs.
  - Minor 6 and B1b: `a5a48f7` is gone from `deviations()`, `output_interface()`, the `facade_ode` comment and corpus.rs.
  - B1a, B1c and B1e are honoured.
  - corpus_v2.json sha256 `f85db964…d18d2623` matches the preregistration (I recomputed it).
- ⚠️ Cannot verify from the diff alone:
  - The report claims 15 tests pass and clippy is clean. I did not re-run the suite, as instructed.
  - The row count of 132 in the handoff is arithmetically right. 9 non-banded arms × 9 cells plus 17 arms × 3 cells on the Brusselator is 132, with 8 banded arms.
  - The checker failure for the missing L-0142 row is expected and not flagged.

### Strengths

- The dense arms step freely. `vigilode_dense` (arms.rs, new fn) builds `Options{dense_output:true, ..Default}`, solves over the span only and takes the grid solely through `sol.at(t)`.
  - The schedule in solve.rs is `[t0,t1]` only (solve.rs:127-131), so there are no tstops at grid points.
  - `output_clipped_steps` is read from the facade stats.
- The protocol status is correct.
  - `clipped != 0` takes precedence, then a successful run with `states.len() != grid.len()`.
  - Violation rows are kept. They are not `Success`, so they get no error value and drop out of the matched curves, but they stay as rows and are counted in `protocol_violation_rows`.
- All arms are scored on the same 101-point grid by the unchanged harness `max_wrms` against one reference.
- R-C5 compliance holds in the new code.
  - It uses only `==` and `matches!` on facade enums: `interior == InteriorError::Off`, `matches!(arm, ..)`.
  - It has no exhaustive `match` on a facade enum and no struct literal of an output type.
  - The only literal is `Options{.., ..Default::default()}`.
  - `Atol::Scalar(..)` is an input constructor.
  - It reads `st.counters`, `st.route.*` and `st.attempts` as fields only.
- The bit-for-bit test compares the arm with a direct `Rodas5p::solve` + `at` across three problems and all four arms.
- The preregistration discloses the smoke anomaly (limitation 3).
- The report test still enforces that every number in the Markdown is in the JSON.

### Check 1: measurement validity

- **Reference:** keep the grid-clipped `vigilode` arm at 1e-12.
  - Its landings are exact grid values, so it adds no interpolation error.
  - The dense arm at 1e-12 would put interpolant error into the reference.
  - The Diffsol cross-check and the dispute threshold still apply.
  - Limitation 2 of the preregistration states the choice.
- **Errors:** measured on the same grid for all arms. No finding.

### Check 2: smoke anomaly (WFilteredEnforce vs Off identical)

I ran one small targeted check, Brusselator n=100, rtol 1e-6, atol 1e-6, through `arms::facade_problem` and `dense_options`. It used a scratch crate in the scratchpad, with `RUSTUP_TOOLCHAIN=stable`, `CARGO_TARGET_DIR=/home/claude/targets/e1b-dense` and `-j 1`. `--locked` was not possible for the scratch crate, so it ran with `--offline`.

| Arm | accepted | rejected | `control` | `samples` | `interior_rejections` | `max_accepted_estimate` |
| --- | --- | --- | --- | --- | --- | --- |
| enforce | 78 | 9 | `WFilteredEnforce` | 78 | 0 | 0.64 |
| off | 78 | 9 | `Off` | 0 | 0 | none |

- The option is not dropped.
  - The arm passes `interior_error` to `Options`.
  - solve.rs:288-296 maps it to `DenseErrorControl` only when `dense_output` is true.
  - The sequential dense route is the one taken (`sequential = mass || dense_output`, solve.rs:131).
- The arms are identical because no interior check fails on this problem. The largest interior estimate is 0.64 tolerance units. That is a legitimate outcome.
- The error of about 9 tolerance units sits against an interior estimate of 0.64. The interior estimator does not see whatever dominates `error_own` here. The A2 table will show this, and the harness should not hide it.

### Issues

#### Critical (Must Fix)

None.

#### Important (Should Fix)

1. **The A2 evidence cannot tell "option dropped" from "no interior check fired", and nothing tests that the mode reaches the facade.**
   - arms.rs `vigilode_dense`, the `native` json: `interior_error` echoes the arm's request, not what ran.
   - The route's `DriverWork::SequentialDense{control, samples, interior_rejections, max_accepted_estimate}` is not recorded.
   - The `interior_control` rows hold only `error_own` and accepted steps. If Enforce equals Off, the full run cannot show why.
   - The lock-in tests (`dense_arms_equal_facade_solve` and the other two) cannot catch a dropped option either. Enforce and Off are bit-identical whenever no interior rejection occurs (Robertson, HIRES and Brusselator n=20 at 1e-6 are likely to be).
   - Fix, before T1b so that the corpus runs once:
     - Record `control`, `samples`, `interior_rejections` and `max_accepted_estimate` in `native`, and add `interior_rejections` and `max_estimate` columns to `interior_control`.
     - Read them with `if let DriverWork::SequentialDense{..} = &st.driver_work`, never an exhaustive match, so the T3 fast dense route does not break R-C5.
     - Add a test that `native.control` is `WFilteredEnforce` for the two enforce arms and `Off` for the two off arms.
2. **The handoff is not runnable on a fresh Ubuntu workstation** (LOCAL_E1B_HANDOFF_W3.md:51; the Comparator toolchain table above it).
   - build.rs needs `gcc`, `ar` and `nm` (build-essential, binutils) and links `-lblas` (build.rs:130). The `apt-get install` line has gfortran, libsundials-dev, binutils and util-linux. It lacks `build-essential` and `libblas-dev` or `libopenblas-dev`.
   - `2>/dev/null || true` hides an apt failure, so the failure shows up later as a link error.
   - There is no rustup, git or curl bootstrap, only "let rustup install it".
   - Fix:
     - Install `build-essential gfortran libsundials-dev libopenblas-dev git curl python3`.
     - Drop the stderr suppression.
     - Check `dpkg -s` for each package before the build.

#### Minor (Nice to Have)

1. LOCAL_E1B_HANDOFF_W3.md:5-7. The R-C5 rerun appears only as a precondition for filling `SHA_UNDER_TEST`. It is neither a controller checklist item nor a matrix row. Add "R-C5: `cargo test --locked` and `cargo clippy --all-targets -D warnings` in `crates/vigilode-bench` at SHA_UNDER_TEST, exit 0" as an explicit step.
2. The `ProtocolViolation` path is never exercised by a test. Both new tests check only the clean case. Add a unit test of the status logic, for example a mutated stats value or a tiny grid test that forces `at` to return `None`.
3. PREREGISTRATION.md.
   - The gates are concrete only for the protocol rule (`protocol_violation_rows = 0`, `output_clipped_steps = 0`) and the INCONCLUSIVE verdict.
   - The A2 "≤ 2 tol units" threshold carries no consequence and is declared as evidence only. That is acceptable for an INCONCLUSIVE node.
   - The commit order is right: aaab7f1 comes before any corpus run. The smoke run preceded it and is disclosed.
4. Limitation 3 says "identical errors" without the cause. After Important 1 is fixed it can cite `interior_rejections = 0`.
5. `vigilode_dense`: `break` on the first `None` from `at` leaves a partial `states` vector. This is correct for the status logic.

### Assessment

**Task quality:** Needs fixes

**Reasoning:** The measurement design is sound. The dense arms step freely, the status logic and the same-grid error are right, and R-C5 holds. The tests are real, but they would not have failed on a silently dropped interior mode. The harness records too little to interpret the Enforce versus Off comparison, and the handoff lacks build dependencies, so a fresh workstation build would fail. Both Important items are cheap to fix before the one-shot T1b run.

**Missing RED step (point 6):** The behavioural tests would fail without the arms, since the arms do not compile and the bit-equality and clipped-step claims then have nothing to check. As a gate on the interior mode they are weak, as covered in Important 1. I do not require a retroactive RED run. I do require the mode-passthrough assertion.
