# Preregistration: the R4 runtime remainder named tests (R4-VERIFY-DEV-01)

Written and committed before the run. Results are appended below the line at the end after the run.

## Question

The R4 audit's local execution stopped 20 named tests at a bounded timeout and skipped 40 more for lack of total
budget (`research/adversarial_reaudit_20261001_r4/evidence/runtime/LOCAL_REMAINDER_PLAN.json`). Do all 60 pass
when each is run alone with a larger budget?

## Source and command

- Source commit: `96639dbf9b6be7c288028534570674d4decfcfcb` (branch `claude/jolly-wozniak-7wl15h-wu24-reaudit-r4`), rustc 1.94.1.
- Command:
  `CARGO_TARGET_DIR=<dir> CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 RAYON_NUM_THREADS=1 python3 tools/r4_run_named_tests.py --budget-seconds 3600 --output research/r4_runtime_remainder_20261001/NAMED_TESTS.json`

## Changed identity

The plan names debug with all features, one job and one codegen unit. This run uses debug with default features and
the workspace's codegen settings, so the identity differs. The named tests need no feature. The difference is
recorded in the output and is not hidden.

## Gate

Each test is classified PASS, FAIL, TIMEOUT or NOT_FOUND. A timeout is never a pass.

**PASS** if all 60 tests are PASS. Otherwise the verdict is **FAIL**, and every non-pass is listed with its log tail.
The plan's four ignored tests are listed and not run. One of them writes a tracked fixture.

## Prior information

The same targets ran in this branch's full workspace test runs, all at once rather than one by one. Their outcome
there is known, so this run measures per-test status and wall time under the stated budget. It is not a blind test.
