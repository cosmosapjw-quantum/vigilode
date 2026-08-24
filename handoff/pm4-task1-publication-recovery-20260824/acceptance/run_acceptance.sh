#!/usr/bin/env bash
set -euo pipefail
: "${PM4_R5_DIR:?set PM4_R5_DIR to the candidate R5 directory}"
: "${PM4_R4_ARCHIVE:?set PM4_R4_ARCHIVE to the canonical R4 archive}"
: "${PM4_R4_SIDECAR:?set PM4_R4_SIDECAR to its .sha256 sidecar}"

python3 acceptance/test_archive_authority_contract.py \
  --archive "$PM4_R4_ARCHIVE" \
  --sidecar "$PM4_R4_SIDECAR"

python3 acceptance/test_completion_evidence_schema_contract.py \
  --schema templates/COMPLETION_EVIDENCE_SCHEMA.json

PM4_R5_DIR="$PM4_R5_DIR" \
python3 -m unittest acceptance.test_vendor_validator_contract -v

python3 acceptance/test_publication_script_contract.py \
  --r5-dir "$PM4_R5_DIR"

if [[ -n "${PM4_COMPLETION_EVIDENCE:-}" ]]; then
  python3 acceptance/test_completion_evidence_schema_contract.py \
    --schema templates/COMPLETION_EVIDENCE_SCHEMA.json \
    --instance "$PM4_COMPLETION_EVIDENCE"
fi
