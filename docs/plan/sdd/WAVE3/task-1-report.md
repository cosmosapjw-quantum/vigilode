# Task 1 (T1a) report: e1b-dense

Status: DONE_WITH_CONCERNS. Branch production/e1b-dense (base e9c5bbd, contains L-0141), pushed.

## Commits
- 219a140 harness: four facade dense arms, protocol check, interior-control report, stale-commit fix
- aaab7f1 PREREGISTRATION.md + corpus_v2.json (research/bench_wave3_20261021/), before any measurement
- 0700689 docs/plan/handoff/LOCAL_E1B_HANDOFF_W3.md

## Requirements
- Arms Arm::{VigilodeDense,VigilodeDenseOff,VigilodeBandedDense,VigilodeBandedDenseOff}, kebab ids as specified, Arm::ALL = 17: arms.rs (enum, ALL, vigilode_dense, facade_problem, dense_options). Only `==`/`matches!` on facade enums, no struct literal of output types (R-C5); `Options {.., ..Default::default()}` only.
- Protocol check: RunStatus::ProtocolViolation (kebab "protocol-violation"); clipped != 0, or successful run with at(t_k)==None, kept as a row. Harness counts them in `protocol_violation_rows`.
- Row records `native.driver_id` (stats.route.driver_id), entry_point, output_clipped_steps.
- Matched rows get `basis`: "dense" (headline: vigilode-dense / vigilode-banded-dense) and "grid-clipped" (v1 continuity arms): harness.rs (`dense_baseline_of`, matched_rows).
- Interior control (A2): harness `interior_control` JSON (+ `interior_control_tol_units` = 2) and two report sections ("Endpoint-only VigilODE (dense output)", "Interior control evidence (A2)"): report.rs.
- Minor 6 and B1b: no "a5a48f7" in any deviations() or output_interface() (arms.rs), also arms.rs facade_ode comment and corpus.rs comment. results_v1.json untouched.
- Corpus schema id kept v1 (B1c). corpus_v2.json = v1 with id e1b-dense-corpus-v2 and 17 arms, sha256 f85db964...d18d2623.
- harness timing_note no longer claims "shared 2-core host" (v2 runs on the workstation).
- Handoff: CODE_TASK, SCIENTIFIC_CONTRACT, VALIDATION_MATRIX, exact commands, toolchain table, results branch results/local-e1b-<sha> with results_v2.json, REPORT.md, host.json (JSON incl. timing_authority, B1a) + host.txt, SHA_UNDER_TEST placeholder, binary path under $CARGO_TARGET_DIR/measurement (B1e). Expected rows 132.
- L-0142 NOT appended (controller does it).

## Tests
In crates/vigilode-bench, `cargo test --locked -j 1`: 15 pass (5 lib, 6 adapters, 3 report, 1 sources). New: dense_arms_equal_facade_solve (bit-identical at(t_k), Robertson, HIRES, Brusselator, rtol 1e-6, all four arms), dense_arms_take_no_clipped_steps, fidelity_notes_name_no_stale_commit (deviations() and output_interface()), report test dense_sections_are_generated_from_the_json... (every Markdown number is in the JSON). `cargo clippy -j 1 --locked --all-targets -- -D warnings` clean.
Smoke (debug build, Brusselator n=100, rtol 1e-6, 1 rep): all four dense arms Success, 101 grid points, output_clipped_steps 0, protocol_violation_rows 0, driver_id rodas5p-sequential-dense-v2, 78 accepted steps each. The corpus was NOT run.

## Deviations / concerns
1. RED step not recorded: arms were written before the tests in this session; the tests were not run against a stub. They do cover the behaviour (bit-equality, clipped = 0, no stale commit).
2. Smoke observation, disclosed in PREREGISTRATION limitation 3: dense arms have error_own about 9 tolerance units on the Brusselator at rtol 1e-6, and WFilteredEnforce and Off give identical step counts there (78) and identical errors. The A2 "<= 2 tol units" column will read "no" for this row. The full run will show whether this is general.
3. Dense arms currently route to the sequential dense driver, banded arms too (Jacobian::banded just supplies the matrix); on this base, banded-dense equals dense bit for bit. The measured route changes when T3 merges; driver_id is recorded per row.
4. tools/check-research-node.py --base a5a48f7 currently FAILs by design: the new node research/bench_wave3_20261021/ has no ledger row until the controller appends L-0142 (with inputs_sha256 per B1d, date >= L-0141's, supersedes L-0140). Expect PASS after that.
5. The dense reference is still the grid-clipped `vigilode` arm at rtol 1e-12 (stated in the preregistration).
6. Per R-C5, T1b must rerun cargo test/clippy in crates/vigilode-bench after merging the post-T7 tip, before filling SHA_UNDER_TEST.

## Handoff commands
See docs/plan/handoff/LOCAL_E1B_HANDOFF_W3.md (build, host.json, wp pinned with taskset -c 2, report).

## Memory-worthy
E1b-dense node: branch production/e1b-dense, dir research/bench_wave3_20261021, L-0142 pending (supersedes L-0140), corpus_v2 sha256 f85db964...

## Fix round 1
- Important 1: arms.rs records `native.ran` {control, samples, interior_rejections, max_accepted_estimate} read from `st.driver_work` via `if let DriverWork::SequentialDense {..}` (a FastDense branch is to be added as another `if let` after p0-fastdense lands; not referenced now). interior_control gains interior_rejections (enforce, off) and max_estimate (enforce) columns in JSON and report. New test dense_arms_record_the_interior_control_that_ran. Bench tests: 16 pass, clippy clean.
- Important 2: handoff apt line now build-essential gfortran libsundials-dev libopenblas-dev binutils util-linux git curl python3, no stderr suppression, dpkg check loop, rustup bootstrap; build.rs tool and library dependencies listed (gfortran, gcc, ar, nm, objcopy, sha256sum; sundials x6, blas, gfortran, m).
- Minor 1 done (R-C5 gate and `native.ran.control` rows added to VALIDATION_MATRIX). Minor 2, 4, 5 not done.

## T1b prep
- Merged 6594d10 (production/wave3-integration, p0-fastdense in) with a merge commit (origin/production/wave3-integration is not a tracking ref in this clone; merged the commit id). p0-api not merged.
- arms.rs: added `if let DriverWork::FastDense { interior, .. }` branch. FastDense has no `control` field, so `native.ran.control` comes from `stats.options.interior_error` (source named in `ran.control_source`); samples, interior_rejections and max_accepted_estimate come from FastInteriorWork (samples, interior_rejections, max_estimate). The sequential branch remains for Enforce/other routes.
- Test dense_arms_record_the_interior_control_that_ran now also asserts samples > 0 exactly for the enforce arms (so a dropped mode would fail). Bench tests 16 pass, clippy clean.
- Driver ids on the dense arms: `rodas5p-fast-dense-v1` (dense) and `rodas5p-fast-banded-dense-v1` (banded).
- Smoke (debug build, Brusselator n=100, rtol 1e-6, 1 rep): all four arms Success, output_clipped_steps 0, protocol violations 0. Enforce and Off: 78 accepted, 9 rejected, error_own 9.0005 each (ratio 1.0, step ratio 1.0); Enforce samples 78, interior_rejections 0, max estimate 0.641; Off samples 0. Banded-dense equals dense. Same numbers as on the sequential route.
- Handoff VALIDATION_MATRIX row for driver ids still reads "list the distinct values"; expected now fast-dense ids. Not edited.
