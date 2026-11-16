#!/usr/bin/env bash
# Verification commands of loop node V07 (stage a). Writes logs/checks.log: for each
# check its command, its full output and a line `CHECK <name> exit=<code>`.
# Run from anywhere; one cargo process at a time.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(git -C "$HERE" rev-parse --show-toplevel)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w2-v07}"
cd "$ROOT" || exit 2
LOG="$HERE/logs/checks.log"
PIN=(taskset -c 0-9,12-21)
OWN_TESTS=(--test stage_clients_speculative_contracts --test stage_clients_speculative_alloc
           --test stage_clients_speculative_problems --test stage_clients_speculative_measure)
OWN_RS=(crates/rodas5p-integrators/src/stage_clients/speculative.rs
        crates/rodas5p-integrators/tests/stage_clients_speculative_problems.rs
        crates/rodas5p-integrators/tests/stage_clients_speculative_contracts.rs
        crates/rodas5p-integrators/tests/stage_clients_speculative_alloc.rs
        crates/rodas5p-integrators/tests/stage_clients_speculative_measure.rs)
BASE=0443025895dabf350f46fd3f0cdd0cf1d7fbcf85
status=0
{
  echo "# git HEAD: $(git rev-parse HEAD)"
  echo "# uncommitted changes: $(git status --porcelain | wc -l)"
  echo "# rustc: $(rustc +1.94.1 --version)"
  echo "# CARGO_TARGET_DIR=$CARGO_TARGET_DIR"
  echo "# started: $(date -u +%Y-%m-%dT%H:%M:%SZ); load average: $(cut -d' ' -f1-3 /proc/loadavg)"
} > "$LOG"

check() {
  local name="$1"
  shift
  {
    echo
    echo "## $name"
    echo "# command: $*"
  } >> "$LOG"
  "$@" >> "$LOG" 2>&1
  local code=$?
  echo "CHECK $name exit=$code" >> "$LOG"
  echo "CHECK $name exit=$code"
  if [ "$code" -ne 0 ]; then
    status=1
  fi
}

check tests_speculative "${PIN[@]}" cargo +1.94.1 test --locked -j 4 --release \
  -p rodas5p-integrators "${OWN_TESTS[@]}"
check tests_sentinels "${PIN[@]}" cargo +1.94.1 test --locked -j 4 --release \
  -p rodas5p-integrators --test ppsw1_6b_contracts --test ptsw2b_contracts
check tests_client_kit "${PIN[@]}" cargo +1.94.1 test --locked -j 4 --release \
  -p rodas5p-integrators --test stage_clients_dense_identity --test stage_clients_alloc
check clippy "${PIN[@]}" cargo +1.94.1 clippy --locked -j 4 --release -p rodas5p-integrators \
  --lib "${OWN_TESTS[@]}" --no-deps -- -D warnings
check rustfmt rustfmt +1.94.1 --check --edition 2024 "${OWN_RS[@]}"
check loop_checker python3 -I research/production_loop_20261116/check_checkpoint.py
check strict_json python3 -I -c '
import json, sys
from pathlib import Path
def pairs(items):
    out = {}
    for key, value in items:
        if key in out:
            raise ValueError("duplicate key " + key)
        out[key] = value
    return out
def constant(value):
    raise ValueError("non-finite constant " + value)
root = Path("research/production_loop_20261116/speculative_jacobian")
files = sorted(root.rglob("*.json"))
for path in files:
    json.loads(path.read_text(), object_pairs_hook=pairs, parse_constant=constant)
    print("strict JSON ok:", path)
lines = 0
for log in sorted((root / "logs").glob("*.log")):
    for line in log.read_text().splitlines():
        for prefix in ("SPECJ ", "REFPY ", "REFJL ", "MODELCHECK "):
            if line.startswith(prefix):
                json.loads(line[len(prefix):], object_pairs_hook=pairs, parse_constant=constant)
                lines += 1
print("strict JSON ok:", lines, "record lines in logs")
sys.exit(0 if files else 1)
'
check allowed_paths python3 -I -c '
import fnmatch, subprocess, sys
base = sys.argv[1]
allowed = ["crates/rodas5p-integrators/src/stage_clients/speculative.rs",
           "crates/rodas5p-integrators/tests/stage_clients_speculative_*.rs",
           "research/production_loop_20261116/speculative_jacobian/*"]
changed = subprocess.check_output(["git", "diff", "--name-only", base, "HEAD"], text=True).split()
outside = [p for p in changed if not any(fnmatch.fnmatchcase(p, a) for a in allowed)]
print(len(changed), "paths changed since", base[:8])
for path in changed:
    print("  ", path)
print("outside the task allowlist:", outside)
sys.exit(1 if outside or not changed else 0)
' "$BASE"
# The measurement is deterministic: a second run prints the same records, byte for byte.
RERUN="$(mktemp -d "${TMPDIR:-/tmp}/specj-rerun.XXXXXX")"
check measurement_reproducible bash -c '
set -u
here="$1"; rerun="$2"
SPECJ_LOG_DIR="$rerun" bash "$here/run_measurements.sh" || exit 1
for program in measure_model_probes measure_adaptive measure_fixed_step; do
  if ! cmp <(grep "^SPECJ " "$here/logs/$program.log") <(grep "^SPECJ " "$rerun/$program.log"); then
    echo "records of $program differ from the committed log"; exit 1
  fi
  echo "$program: $(grep -c "^SPECJ " "$rerun/$program.log") records identical to logs/$program.log"
done
' _ "$HERE" "$RERUN"
rm -rf "$RERUN"
{
  echo
  echo "# finished: $(date -u +%Y-%m-%dT%H:%M:%SZ); load average: $(cut -d' ' -f1-3 /proc/loadavg)"
  echo "# overall status: $status"
} >> "$LOG"
exit "$status"
