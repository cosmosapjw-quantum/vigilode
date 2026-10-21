VERDICT: ALL ADDRESSED

Scope: fix round 1 for wave-3 Task 1a (e1b-dense), diff 0700689..58e7f0f. I read the diff, the findings, the report's "Fix round 1" section and build.rs. I re-ran no tests.

### Finding Verdicts

- **Important 1 (A2 evidence could not tell "option dropped" from "no interior check fired"; no test that the mode reaches the facade)** — ADDRESSED.
  - arms.rs (hunk at `vigilode_dense`, around the new `let mut ran = json!(...)`): `native.ran` is built from `st.driver_work` with `if let DriverWork::SequentialDense{control, samples, interior_rejections, max_accepted_estimate, ..} = &st.driver_work`. It records the driver's own `control` (formatted from the driver value), not the request. The old `interior_error` echo remains, but only as the request.
  - There is no exhaustive match. Rows from any other route get `null` fields. R-C5 holds. `DriverWork` is only imported and destructured by `if let`.
  - harness.rs `InteriorControl` gains `interior_rejections_enforce`, `interior_rejections_off` and `max_estimate_enforce`. `interior_control_rows` fills them from `e.native["ran"]` and `o.native["ran"]`.
  - report.rs adds three matching columns. Every value in the Markdown comes from the JSON, so the "every number is in the JSON" test still holds.
  - New test `dense_arms_record_the_interior_control_that_ran` (tests/adapters.rs, +173 to +194):
    - It asserts `out.native["ran"]["control"] == "WFilteredEnforce"` for `VigilodeDense` and `VigilodeBandedDense`.
    - It asserts `== "Off"` for `VigilodeDenseOff` and `VigilodeBandedDenseOff`.
    - It covers Robertson and Brusselator n=20, and asserts `interior_rejections.is_u64()`.
  - Enforce vs Off check: the compared value is `DriverWork::SequentialDense.control` taken from the solve's stats. An arm that dropped the option would report `Off` (or `null` if the route changed), and the Enforce assertion would fail. The test does not depend on a rejection occurring. Echoing the request back would not pass: `ran.control` is a separate field from `interior_error`, and `null` never equals the expected string.
  - Residual: `max_accepted_estimate` is recorded for the Off arm in `native` but not as an `interior_control` column. That is acceptable, because the Off arm reports `samples = 0` and no estimate.

- **Important 2 (handoff not runnable on fresh Ubuntu)** — ADDRESSED.
  - LOCAL_E1B_HANDOFF_W3.md, the apt block: `apt-get update`, then `install -y build-essential gfortran libsundials-dev libopenblas-dev binutils util-linux git curl python3`. The `2>/dev/null || true` is gone. A `dpkg -s` loop exits on a missing package. A rustup bootstrap is added.
  - The dependency list matches build.rs.
    - Tools invoked (build.rs:25 `nm`; :46 `sha256sum`; :55 and :150 `gfortran`; :87 `objcopy`; :106 `gcc`; :117 `ar`; :161 `rustc`). The doc names all seven, with `gcc` and `ar` from build-essential, `nm` and `objcopy` from binutils, and `sha256sum` from coreutils.
    - `-l` link flags at build.rs:123-133: `sundials_cvode`, `sundials_nvecserial`, `sundials_sunmatrixdense`, `sundials_sunmatrixband`, `sundials_sunlinsoldense`, `sundials_sunlinsolband`, `blas`, `gfortran`, `m`. The doc lists exactly these. Each of the six sundials libraries ships with libsundials-dev, `blas` comes from libopenblas-dev via alternatives, `gfortran` from the gfortran package, and `m` from libc.
    - Runtime tools outside build.rs: `git` (main.rs:156 and :166), and `taskset` (util-linux). Both are covered.
  - `command -v rustup || curl | sh -s -- -y && . ~/.cargo/env` parses as `(a || b) && c`, which is the intended behavior.

- **Minor 1 (R-C5 rerun only a precondition)** — ADDRESSED. The VALIDATION_MATRIX gets the row "R-C5 gate at SHA_UNDER_TEST: cargo test --locked and cargo clippy --all-targets -- -D warnings, exit 0 both". It also gets a `native.ran.control` expectation row.

### Unfixed Minors, safe to park given T1b runs the corpus once

- **Minor 2 (ProtocolViolation path never tested)** — safe to park.
  - A clean corpus run takes no violation path. The matrix gates `protocol_violation_rows = 0` and `output_clipped_steps = 0` on the output, so a violation would be visible there.
  - If a violation did occur, the status logic that Check 1 found correct would classify it. A bug in that path could only mislabel a row the gate already flags.
  - A unit test is still worth adding before the final review.
- **Minor 4 (limitation 3 lacks a cause)** — safe to park. It is prose only. After the run it can cite `interior_rejections` from the recorded data.
- **Minor 5 (`break` on first `None` leaves a partial `states` vector)** — safe to park. The review judged it correct for the status logic, and the length mismatch is what raises `ProtocolViolation`.

### New Breakage in the Fix Diff

None. Minor observations only:
- The test would fail loudly if a future fast dense route produced `ran.control = null`. That is the intended behavior until the `if let` branch is added.
- The dpkg loop exits the pasted shell on a missing package. That is the intended behavior.

### Out-of-Scope Observations

None.

### Verdict

**Fix round:** All findings addressed, no new Critical/Important breakage.
