#!/usr/bin/env bash
set -uo pipefail
runroot=/home/cosmosapjw/vigilode-nonstiff-run-Ioh017
source_tree=/home/cosmosapjw/vigilode-nonstiff-source-WjxDXM
runtime=/workspace/scratch/0dc1e721cf82/task_runtime
if [[ -x "$runtime/installed/bin/rustc" ]]; then export PATH="$runtime/installed/bin:$PATH"; fi
if [[ -d "$runtime/cargo" ]]; then export CARGO_HOME="$runtime/cargo"; fi
export CARGO_TARGET_DIR="$runroot/build"
export CARGO_BUILD_JOBS=2
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
export PYTHONDONTWRITEBYTECODE=1
export MPLCONFIGDIR="$runroot/matplotlib-cache"
source_commit=45037fc24e8f8101605f309e1fdb3a9a4e3df56d
cd "$source_tree" || exit 99
[[ "$(git rev-parse HEAD)" == "$source_commit" ]] || exit 98
[[ -z "$(git status --porcelain=v1)" ]] || exit 97
[[ "$(cat "$runroot/build-measurement.exit.txt")" == 0 ]] || exit 96
sha256sum -c "$runroot/binary-sha256.txt" > "$runroot/binary-prebenchmark-check.log" 2>&1 || exit 95
if [[ -e "$runroot/full-comparison" ]]; then exit 94; fi
date -u +%FT%TZ > "$runroot/full-comparison.started.txt"
python3 research/nonstiff_explicit_20261023/run_bench.py \
  --binary "$CARGO_TARGET_DIR/measurement/examples/nonstiff_bench" \
  --out "$runroot/full-comparison" \
  --source-commit "$source_commit" \
  --external --full --budget-seconds 7200 \
  > "$runroot/full-comparison-runner.log" 2>&1
rc=$?; printf '%s\n' "$rc" > "$runroot/full-comparison-runner.exit.txt"; date -u +%FT%TZ > "$runroot/full-comparison.ended.txt"
if ((rc != 0)); then exit "$rc"; fi
date -u +%FT%TZ > "$runroot/full-comparison-analysis.started.txt"
python3 research/nonstiff_explicit_20261023/analyze_bench.py "$runroot/full-comparison" \
  > "$runroot/full-comparison-analysis.log" 2>&1
rc=$?; printf '%s\n' "$rc" > "$runroot/full-comparison-analysis.exit.txt"; date -u +%FT%TZ > "$runroot/full-comparison-analysis.ended.txt"
if ((rc != 0)); then exit "$rc"; fi
if [[ -e "$runroot/ensemble-comparison" ]]; then exit 93; fi
date -u +%FT%TZ > "$runroot/ensemble-comparison.started.txt"
python3 research/nonstiff_explicit_20261023/run_ensemble_bench.py \
  --binary "$CARGO_TARGET_DIR/measurement/examples/nonstiff_ensemble_bench" \
  --out "$runroot/ensemble-comparison" \
  --source-commit "$source_commit" --budget-seconds 180 \
  > "$runroot/ensemble-comparison-runner.log" 2>&1
rc=$?; printf '%s\n' "$rc" > "$runroot/ensemble-comparison-runner.exit.txt"; date -u +%FT%TZ > "$runroot/ensemble-comparison.ended.txt"
if ((rc != 0)); then exit "$rc"; fi
sha256sum -c "$runroot/binary-sha256.txt" > "$runroot/binary-postbenchmark-check.log" 2>&1 || exit 92
git rev-parse HEAD > "$runroot/final-source-head.txt"
git rev-parse 'HEAD^{tree}' > "$runroot/final-source-tree.txt"
git status --porcelain=v1 > "$runroot/final-source-status.txt"
