#!/usr/bin/env bash
# Scoped software/usage checks. No campaign, holdout, timing, or claim promotion.
set -euo pipefail
cd "$(dirname "$0")/.."
export PYTHONDONTWRITEBYTECODE=1
OUT=${AUDIT2_OUTPUT_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/vigilode-audit2-readiness.XXXXXX")}
mkdir -p "$OUT"
for name in solve-stiff.json solve-stiff-budget-exhausted.json; do
  if test -e "$OUT/$name"; then
    echo "Refusing to overwrite existing example output: $OUT/$name" >&2
    exit 2
  fi
done

python3 tools/test_a1_receipt_ci_scope.py -v
python3 tools/test_audit2_output_policy_research.py -v
python3 tools/test_audit2_real_client_authority.py -v
python3 tools/test_audit2_stage_certificate_handoff.py -v
python3 tools/test_audit2_stage_certificate_repair_handoff.py -v
python3 tools/test_audit2_bateman_local_receipt.py -v
python3 tools/test_audit2_bateman_local_validation_runner.py -v
python3 tools/test_adjudicate_audit2_bateman_local_validation.py -v
cargo test --locked -p rodas5p-fair-ab --test global_error_contracts --test output_accuracy_assessment_contracts
cargo test --locked -p rodas5p-integrators --features audit2-research --test audit2_structured_correction_contracts --test audit2_matrix_free_common_w_contracts --test audit2_reusable_preconditioner_transaction_contracts --test dense_output_v2_contracts --test homotopy_numerical_contracts
cargo test --locked -p rodas5p-integrators --features audit2-bateman-authority --test audit2_real_client_authority_contracts
cargo check --locked -p rodas5p-integrators --features audit2-bateman-authority --example audit2_bateman_local_six_case
cargo test --locked -p rodas5p-integrators --no-default-features --example solve_stiff
cargo check --locked -p rodas5p-integrators --no-default-features
cargo clippy --locked -p rodas5p-integrators -p rodas5p-fair-ab --all-targets --features rodas5p-integrators/audit2-bateman-authority -- -D warnings
cargo fmt --all -- --check

cargo run --quiet --locked -p rodas5p-integrators --no-default-features --example solve_stiff > "$OUT/solve-stiff.json"
set +e
cargo run --quiet --locked -p rodas5p-integrators --no-default-features --example solve_stiff -- --max-attempts 1 > "$OUT/solve-stiff-budget-exhausted.json"
failure_exit=$?
set -e
# A compiler/tool failure is not proof of the expected numerical partial result.
test "$failure_exit" -eq 1
python3 - "$OUT" <<'PY'
import json
import pathlib
import sys
root = pathlib.Path(sys.argv[1])
complete = json.loads((root / "solve-stiff.json").read_text())
partial = json.loads((root / "solve-stiff-budget-exhausted.json").read_text())
assert complete["success"] and complete["complete_output"] and complete["demo_accuracy_pass"]
assert not complete["audit2_correction_used"]
assert complete["counters"]["jacobian_builds"] == 0
assert complete["counters"]["direct_factorizations"] == 0
assert complete["output_clipped_steps"] == 0
assert not partial["success"] and not partial["complete_output"] and not partial["demo_accuracy_pass"]
assert partial["diagnostics"]["attempts"] == 1
assert partial["counters"]["rhs_evaluations"] > 0 and partial["y"]
print("Default solver example: complete solve and typed partial failure exercised.")
print("This checks a documented narrow example, not production/general accuracy or speedup.")
PY
printf 'AUDIT2_USAGE_OUTPUT_DIR=%s\n' "$OUT"
