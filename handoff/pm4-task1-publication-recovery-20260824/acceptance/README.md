# Acceptance contract for the R5 repair

Codex must create an R5 directory containing at least:

```text
publish_pm4_task1.sh
validate_vendor_source.py
test_validate_vendor_source.py
README.md
STATE.json
MANIFEST.json
SHA256SUMS
PM4_TASK1_SCHEMA_BOUNDARY.patch
TASK1_FILE_SHA256SUMS
```

## Mandatory handoff referential-closure gate

Before any R5 implementation, run:

```bash
python3 -m unittest acceptance.test_handoff_completeness_contract -v
python3 -m unittest acceptance.test_completion_evidence_contract -v
python3 acceptance/test_completion_evidence_schema_contract.py \
  --schema templates/COMPLETION_EVIDENCE_SCHEMA.json \
  --instance templates/COMPLETION_EVIDENCE_EXAMPLE.json
```

These checks reject missing repository-local `templates/` or `acceptance/` references, reject schema-named files that are not actual JSON Schema Draft 2020-12 documents, validate the positive completion example, and prove that forbidden merge, missing command evidence, retained P0/P1 findings, or unresolved blockers cannot be reported as success.

## Archive authority contract

Run:

```bash
python3 acceptance/test_archive_authority_contract.py \
  --archive PATH/VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz \
  --sidecar PATH/VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz.sha256
```

It must bind the sole canonical outer SHA-256 `6689544ee9b115fe4cb5c8ba14c179a17ee6615cb454555b0bb2f0ad1826b333`, verify the sidecar, internal `SHA256SUMS`, sealed Task-1 patch, and R4 script, and reject the withdrawn `b33af0...` declaration.

## Vendor helper API and CLI

Python API:

```python
validate_vendor_source(
    vendor_dir: pathlib.Path,
    required_packages: tuple[str, ...],
) -> dict
```

CLI:

```bash
python3 validate_vendor_source.py \
  --vendor-dir PATH \
  --require-package faer-0.24.4 \
  --json-out VENDOR_VALIDATION.json
```

## Required Cargo directory-source classification

At one immediate level:

- ignore dot-prefixed directories;
- only directories containing `Cargo.toml` are package candidates;
- every package candidate must contain parseable `.cargo-checksum.json`;
- manifestless directories are ignored;
- no exact global package count is enforced;
- required package-directory names must be present;
- `cargo metadata --frozen --format-version 1` is the dependency-closure authority.

Required deterministic JSON fields are defined by `templates/VENDOR_VALIDATION_SCHEMA.json`.

Required helper tests:

- valid 262 package candidates: PASS;
- valid 308 package candidates: PASS;
- hidden and manifestless incidental directories ignored: PASS;
- manifest-bearing package missing checksum: FAIL;
- missing `faer-0.24.4`: FAIL;
- deterministic JSON: PASS.

RED before implementation:

```bash
python3 -m unittest acceptance.test_vendor_validator_contract -v
```

GREEN after creating R5:

```bash
PM4_R5_DIR=/absolute/path/to/R5 \
python3 -m unittest acceptance.test_vendor_validator_contract -v

python3 acceptance/test_publication_script_contract.py \
  --r5-dir /absolute/path/to/R5
```

## Completion evidence

After successful publication, write `COMPLETION_EVIDENCE.json` conforming to `templates/COMPLETION_EVIDENCE_SCHEMA.json` and run:

```bash
python3 acceptance/test_completion_evidence_schema_contract.py \
  --schema templates/COMPLETION_EVIDENCE_SCHEMA.json \
  --instance COMPLETION_EVIDENCE.json

python3 acceptance/validate_completion_evidence.py \
  --evidence COMPLETION_EVIDENCE.json
```

A blocked or partial run must not emit a success evidence object.
