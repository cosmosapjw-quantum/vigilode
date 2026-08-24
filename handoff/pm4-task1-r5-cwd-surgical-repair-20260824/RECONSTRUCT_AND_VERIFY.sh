#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ARCHIVE_NAME="VIGILODE_PM4_TASK1_CWD_SURGICAL_HANDOFF_20260824.tar.gz"
SIDECAR_NAME="${ARCHIVE_NAME}.sha256"
EXPECTED_SHA256="5ddb0f19be010d53187bac00d468c110c49a54b3cf168894207584bee04f1694"
OUTPUT_DIR="${HOME}/vigilode-pm4-cwd-surgical-runtime"
KEEP_TEMP=0

usage() {
  cat <<'EOF'
Usage:
  RECONSTRUCT_AND_VERIFY.sh [--output-dir PATH] [--keep-temp]

Reconstructs the complete PM-4 Task-1 CWD-surgical handoff archive from the
checksum-addressed repository-local base64 parts, verifies the exact archive
identity and package manifest, and writes PACKAGE_ROOT.txt under OUTPUT_DIR.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --output-dir)
      [[ $# -ge 2 ]] || { echo "STOP: --output-dir requires a path" >&2; exit 2; }
      OUTPUT_DIR="$2"
      shift 2
      ;;
    --keep-temp)
      KEEP_TEMP=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "STOP: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

OUTPUT_DIR="$(python3 -c 'import os,sys; print(os.path.abspath(os.path.expanduser(sys.argv[1])))' "$OUTPUT_DIR")"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/vigilode-pm4-cwd-handoff.XXXXXX")"
cleanup() {
  if [[ "$KEEP_TEMP" -eq 0 ]]; then
    rm -rf -- "$TMP_DIR"
  else
    printf 'temporary reconstruction directory retained: %s\n' "$TMP_DIR"
  fi
}
trap cleanup EXIT

for part in 00 01 02 03 04 05 06; do
  path="$SCRIPT_DIR/${ARCHIVE_NAME}.b64.part-$part"
  [[ -f "$path" ]] || { echo "STOP: missing transport part: $path" >&2; exit 1; }
done
[[ -f "$SCRIPT_DIR/$SIDECAR_NAME" ]] || {
  echo "STOP: missing archive sidecar: $SCRIPT_DIR/$SIDECAR_NAME" >&2
  exit 1
}

for part in 00 01 02 03 04 05 06; do
  cat -- "$SCRIPT_DIR/${ARCHIVE_NAME}.b64.part-$part"
done | base64 -d > "$TMP_DIR/$ARCHIVE_NAME"

actual_sha256="$(sha256sum "$TMP_DIR/$ARCHIVE_NAME" | awk '{print $1}')"
if [[ "$actual_sha256" != "$EXPECTED_SHA256" ]]; then
  printf 'STOP: reconstructed archive SHA-256 mismatch\nexpected=%s\nactual=%s\n' \
    "$EXPECTED_SHA256" "$actual_sha256" >&2
  exit 1
fi

cp -- "$SCRIPT_DIR/$SIDECAR_NAME" "$TMP_DIR/$SIDECAR_NAME"
(
  cd -- "$TMP_DIR"
  sha256sum -c "$SIDECAR_NAME"
  tar -tzf "$ARCHIVE_NAME" >/dev/null
)

rm -rf -- "$OUTPUT_DIR"
mkdir -p -- "$OUTPUT_DIR"
tar -xzf "$TMP_DIR/$ARCHIVE_NAME" -C "$OUTPUT_DIR"

mapfile -t roots < <(find "$OUTPUT_DIR" -mindepth 1 -maxdepth 1 -type d -print | sort)
if [[ "${#roots[@]}" -ne 1 ]]; then
  printf 'STOP: expected exactly one extracted package root, found %s\n' "${#roots[@]}" >&2
  printf '%s\n' "${roots[@]}" >&2
  exit 1
fi
PACKAGE_ROOT="${roots[0]}"

for required in \
  README_FIRST.md \
  AGENTS.md \
  AUDIT_COMPILED_EXEC_PLAN.yaml \
  IMPLEMENTER_PROMPT.md \
  FRESH_REVIEW_PROMPT.md \
  PACKAGE_MANIFEST.sha256; do
  [[ -f "$PACKAGE_ROOT/$required" ]] || {
    echo "STOP: extracted package missing required file: $required" >&2
    exit 1
  }
done

(
  cd -- "$PACKAGE_ROOT"
  sha256sum -c PACKAGE_MANIFEST.sha256
  python3 -m compileall -q acceptance
  bash -n acceptance/run_acceptance.sh
)

printf '%s\n' "$PACKAGE_ROOT" > "$OUTPUT_DIR/PACKAGE_ROOT.txt"
cat > "$OUTPUT_DIR/RECONSTRUCTION_RECEIPT.json" <<EOF
{
  "schema": "vigilode-pm4-task1-cwd-handoff-reconstruction-v1",
  "archive": "$ARCHIVE_NAME",
  "archive_sha256": "$actual_sha256",
  "package_root": "$PACKAGE_ROOT",
  "source_handoff_root": "$SCRIPT_DIR",
  "status": "PASS"
}
EOF

printf '\nRECONSTRUCTION_COMPLETE\n'
printf 'archive:      %s\n' "$TMP_DIR/$ARCHIVE_NAME"
printf 'sha256:       %s\n' "$actual_sha256"
printf 'package root: %s\n' "$PACKAGE_ROOT"
printf 'prompt:       %s\n' "$PACKAGE_ROOT/IMPLEMENTER_PROMPT.md"
printf 'review:       %s\n' "$PACKAGE_ROOT/FRESH_REVIEW_PROMPT.md"
printf 'receipt:      %s\n' "$OUTPUT_DIR/RECONSTRUCTION_RECEIPT.json"
