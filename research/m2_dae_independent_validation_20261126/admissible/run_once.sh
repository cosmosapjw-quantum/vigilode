#!/bin/bash
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
Q=research/m2_dae_independent_validation_20261126/admissible
test ! -e "$Q/challenge.log" && test ! -e "$Q/VERDICT.json"
sha256sum --quiet -c "$Q/INPUT_SHA256SUMS"
sha256sum --quiet -c "$Q/BINARY.sha256"
B=$(cut -d' ' -f3- "$Q/BINARY.sha256");test -x "$B"
set +e
"$B" sealed_admissible_validation --ignored --exact --nocapture --test-threads=1 > "$Q/challenge.log" 2>&1
rc=$?;echo "$rc" > "$Q/challenge_exit.txt"
PYTHONDONTWRITEBYTECODE=1 OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 "$Q/verdict.py" > "$Q/verdict.log" 2>&1
vr=$?;echo "$vr" > "$Q/verdict_exit.txt"
set -e
printf 'challenge exit=%s verdict exit=%s\n' "$rc" "$vr"
test "$vr" = 0
cat "$Q/verdict.log"
test "$rc" = 0
