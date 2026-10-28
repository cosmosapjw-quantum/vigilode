# Reproduce the production-hardening review

Review branch: `research/production-hardening-20261023`.
Executed source commit: `5913fe062706c820e768b9e150e5cd1c274f7bf9`.
Executed source tree: `7a70caf85569dbe6c77ecb9a53fe666cd40ae618`.
The final documentation commit is a descendant; it is not the executed source.

Keep an existing checkout untouched. From any clone of this repository:

```bash
git fetch origin research/production-hardening-20261023
run_root=$(mktemp -d "${TMPDIR:-/tmp}/vigilode-hardening-XXXXXXXX")
git worktree add --detach "$run_root/checkout" 5913fe062706c820e768b9e150e5cd1c274f7bf9
cd "$run_root/checkout"
git status --porcelain
git rev-parse HEAD 'HEAD^{tree}'
```

Use Rust 1.94.1. Keep Cargo.lock unchanged. If the cache is incomplete, prepare
locked dependencies once (`cargo fetch --locked` on an authorized networked
host), or supply an exact matching vendored source. Do not relabel an offline
cache failure as a solver failure. This audit used the user-local official
1.94.1 compiler and 118 verified locked packages; paths in the environment
receipts are host-specific and need not exist on another host.

```bash
export CARGO_BUILD_JOBS=2
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
export CARGO_TARGET_DIR="$run_root/target"
python3 research/production_hardening_20261023/run_validation.py \
  --source-commit 5913fe062706c820e768b9e150e5cd1c274f7bf9 \
  --out "$run_root/validation"
cargo build --locked --offline --profile measurement -p vigilode \
  --example production_hardening_bench
python3 research/production_hardening_20261023/run_bench.py \
  --binary "$CARGO_TARGET_DIR/measurement/examples/production_hardening_bench" \
  --source-commit 5913fe062706c820e768b9e150e5cd1c274f7bf9 \
  --out "$run_root/benchmark" --budget-seconds 840
```

The default benchmark has 72 configurations, one warmup and three timed
repetitions per configuration. It records the source/tree, executable hash,
CPU affinity, command, stdout/stderr and timeout/missing cases. It uses analytic
references independently reimplemented in Python, not reported errors alone.
Output packaging and analysis are reported separately from solver timing.
Matplotlib is needed for figures. Results already published need not be rerun
unless changed source/assumptions require it.

Re-score preserved raw output without executing any solver:

```bash
python3 research/production_hardening_20261023/analyze_bench.py /path/to/campaign
```

The read-only latest external audit script and coefficient literal matcher
are in the final review branch under `evidence/`. They are analysis artifacts,
not inputs to the compiled source. Old external timings and this local paired
campaign are distinct cohorts. No NCP commands or credentials are required.

Research ledger additions and artifact manifest cover the final review files.
`tools/check-research-node.py --base origin/research/nonstiff-explicit-20261023`
checks the append-only ledger and admitted numeric outputs. Authority checks
should target this new node and ledger; historical unrelated exceptions are
not evidence against or for the new numerical implementation.
