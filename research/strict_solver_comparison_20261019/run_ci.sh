#!/usr/bin/env bash
set -euo pipefail
OUT=$(realpath "$1")
NODE=$(cd "$(dirname "$0")" && pwd)
set +e
python3 "$NODE/scripts/test_contract.py" > "$OUT/contract_tests_RED.log" 2>&1
CODE=$?
set -e
printf '{"stage":"C2_RED","test_exit":%s,"benchmark_executed":false}\n' "$CODE" > "$OUT/status.json"
cat "$OUT/contract_tests_RED.log"
[[ "$CODE" -ne 0 ]]
