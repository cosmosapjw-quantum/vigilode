# Local test handoff — VigilODE wave-2 integration verification

You are a Claude Code session on the repository owner's workstation (Ryzen 9 5900X, 12 cores / 24 threads, 96 GB RAM). A cloud controller session hands you one job: run the long test suites on a pinned commit and push the logs back. The cloud sandbox has 2 cores and a tight disk, which is why this runs here.

## CODE_TASK

- Outcome: a pushed results branch holding full test logs plus a machine-readable summary for the commit under test.
- Purpose: verify the merged wave-2 nodes together before the final fix wave lands. The nodes are C1 transaction, B1b lagged GMRES, P0 facade, H hygiene and the E1 bench harness.
- In scope: build and test only.
- Out of scope:
  - editing code, fixing failures, or rerunning with altered flags to make something pass
  - merging, rebasing or force-pushing
  - pushing to any branch other than the results branch named below
  - touching `main`
- Approval boundary: stop and report in the summary rather than work around it. Examples: the toolchain install fails, the disk runs out, or a test hangs for more than 30 minutes with no CPU use.

## Setup

```bash
git clone https://github.com/cosmosapjw-quantum/vigilode.git vigilode-w2 || (cd vigilode-w2 && git fetch origin)
cd vigilode-w2
git fetch origin production/wave2-integration production/h-checker
git checkout --detach 00973c7          # commit under test; verify with git rev-parse HEAD
SHA=$(git rev-parse --short HEAD)
R=results/local_w2_$SHA
mkdir -p $R
```

The repository pins Rust 1.94.1 in `rust-toolchain.toml`. The cloud could not download the pin, so use it here: let rustup install it on the first cargo call. If the install fails, use `RUSTUP_TOOLCHAIN=stable` and record that in the summary.

## Commands (run in order, from the repo root, each once)

```bash
export CARGO_TARGET_DIR=$PWD/../vigilode-w2-target CARGO_INCREMENTAL=0
{ rustc -V; cargo -V; uname -a; nproc; date -Is; } > $R/env.txt

( time cargo test -j 12 --workspace --locked --no-fail-fast ) > $R/workspace.log 2>&1; echo "workspace $?" >> $R/exit_codes.txt
( cd crates/vigilode-hybrid && time cargo test -j 12 --locked ) > $R/hybrid.log 2>&1; echo "hybrid $?" >> $R/exit_codes.txt
( cd crates/vigilode-bench && export OPENBLAS_NUM_THREADS=1 && time cargo test -j 12 --locked ) > $R/bench.log 2>&1; echo "bench $?" >> $R/exit_codes.txt
python3 -m unittest tools/test_check_research_node.py > $R/checker_unittest.log 2>&1; echo "checker_unittest $?" >> $R/exit_codes.txt
python3 tools/check-research-node.py --base a5a48f77e7ac7a80b8ff86cd897110790066f7ee > $R/checker_gate.log 2>&1; echo "checker_gate $?" >> $R/exit_codes.txt

# ledger-checker fix, not yet merged (production/h-checker 4f92ec3); results dir is untracked so it survives checkout
git checkout --detach 4f92ec3
python3 -m unittest tools/test_check_research_node.py > $R/checker_unittest_hchecker.log 2>&1; echo "checker_unittest_hchecker $?" >> $R/exit_codes.txt
python3 tools/check-research-node.py --base a5a48f77e7ac7a80b8ff86cd897110790066f7ee > $R/checker_gate_hchecker.log 2>&1; echo "checker_gate_hchecker $?" >> $R/exit_codes.txt
git checkout --detach 00973c7
```

Do not add `--release`, because the cloud baseline is debug.

## SCIENTIFIC_CONTRACT (what counts as correct here)

- Numerical tests compare against fixed references with tolerances that live in the code. This run changes no tolerance.
- Known slow test: `probe_sweep_fails_closed_or_documents_findings` in `tests/triage_cli.rs` spawns `rodas5p triage-run` subprocesses. It took 794 s on the cloud in debug. A long runtime with live child processes is normal, not a stall.
- Allocation-counter tests failed under parallel test threads in wave 1. Wave 2 (H hygiene) moved them to thread-local counters, so they should now pass in parallel. A failure there is a real regression: report it, and do not rerun single-threaded to hide it.
- Rust 1.94.1 against the cloud's stable 1.97 is itself a comparison. List every failing test name so the controller can diff the two runs.

## VALIDATION_MATRIX

| Check | Expected | Source of expectation |
|---|---|---|
| workspace exit code | 0 | wave-2 per-node reviews; the 4 allocation-counter failures from wave 1 are fixed |
| workspace failed tests | 0 | same |
| workspace ignored tests | about 110 (wave 1: 109) | wave-1 integration log |
| hybrid exit code | 0 | wave-1 integration |
| bench exit code | 0 | E1 node review |
| checker_unittest at 00973c7 | 15 tests OK | pre-fix test file |
| checker_gate at 00973c7 | exit 1, only the two L-0124 lines | known pre-existing mismatch |
| checker_unittest_hchecker at 4f92ec3 | 16 tests OK | h-checker commit |
| checker_gate_hchecker at 4f92ec3 | exit 0, PASS | h-checker commit |

## Summary file

Write `$R/SUMMARY.json` with these fields:

- `sha_under_test`
- `toolchain`: the output of `rustc -V`
- `pinned_toolchain_used`: bool
- `suites`: for workspace, hybrid and bench, each an object `{exit, passed, failed, ignored, wall_seconds}`. Sum the `test result:` lines, and take the wall time from `time`.
- `failed_test_names`: a list
- `matrix`: one object per row above, `{check, expected, observed, status}`, where status is MATCH, DIFF or NOT_RUN
- `notes`: free text that labels facts separately from your interpretation

Never mark a check MATCH if it did not run.

## Return path

```bash
git checkout -b results/local-wave2-$SHA
git add -f $R
git commit -m "results: local wave-2 test run on $SHA"
git push -u origin results/local-wave2-$SHA
```

Then show the user the branch name and the filled matrix. The cloud controller fetches that branch and compares it with its own run.
