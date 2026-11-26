#!/bin/bash
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
P=research/m2_dae_independent_validation_20261126
test ! -e "$P/challenge.log"
test ! -e "$P/VERDICT.json"
sha256sum --quiet -c "$P/INPUT_SHA256SUMS"
sha256sum --quiet -c "$P/BINARY.sha256"
B=$(cut -d' ' -f3- "$P/BINARY.sha256")
test -x "$B"
set +e
"$B" sealed_validation --ignored --exact --nocapture --test-threads=1 > "$P/challenge.log" 2>&1
rc=$?
echo "$rc" > "$P/challenge_exit.txt"
# The committed verdict script is the first reader of the challenge rows.
PYTHONDONTWRITEBYTECODE=1 OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 "$P/verdict.py" > "$P/verdict.log" 2>&1
vr=$?
echo "$vr" > "$P/verdict_exit.txt"
set -e
printf 'challenge process exit=%s; verdict process exit=%s\n' "$rc" "$vr"
test "$vr" = 0
cat "$P/verdict.log"
test "$rc" = 0
