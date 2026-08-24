# R5 acceptance contract

Codex must create an R5 working directory containing at least:

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

## Archive authority gate

Before any RED/GREEN or R5 implementation work, run:

```bash
python3 \
  handoff/pm4-task1-publication-recovery-20260824/acceptance/test_archive_authority_contract.py \
  --archive ~/vigilode/VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz \
  --sidecar ~/vigilode/VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz.sha256
```

Required identity:

```text
outer SHA-256
6689544ee9b115fe4cb5c8ba14c179a17ee6615cb454555b0bb2f0ad1826b333

Task-1 patch SHA-256
705646496b3594adb4f655829dfe2756aca57ce061fef0cae3b080399104f7a3

R4 publication script SHA-256
63c4ae3ca493a6b4ffe03db50a1b1e23850dacf5dcd0f502f594464cbd67ddb7
```

The earlier `b33af0b8352aa0b3ccdcc83834cb4696fce787d0733a7e5ce9286e646994a095` declaration is withdrawn and must not be accepted as an alternative. The test also extracts the archive and runs internal `SHA256SUMS`.

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

## Cargo directory-source semantics required by this contract

At one immediate directory level:

- ignore dot-prefixed directories;
- only a directory containing `Cargo.toml` is a package candidate;
- every package candidate must contain a parseable `.cargo-checksum.json`;
- manifestless directories are ignored as non-packages;
- no exact global package count is enforced;
- explicitly required package-directory names must be present;
- Cargo `--frozen` metadata remains the dependency-closure authority.

Required deterministic JSON fields:

```text
schema = vigilode-cargo-directory-source-validation-v1
vendor_dir
immediate_directory_count
package_directory_count
checksum_record_count
ignored_hidden_directory_count
ignored_manifestless_directory_count
required_packages_present
missing_checksum_packages
exact_package_count_enforced = false
```

## RED/GREEN commands

Before R5 exists:

```bash
PM4_R5_DIR=/nonexistent \
python3 -m unittest discover \
  -s handoff/pm4-task1-publication-recovery-20260824/acceptance \
  -p 'test_vendor_validator_contract.py' -v
```

After R5 exists:

```bash
PM4_R5_DIR=/absolute/path/to/R5 \
python3 -m unittest discover \
  -s handoff/pm4-task1-publication-recovery-20260824/acceptance \
  -p 'test_vendor_validator_contract.py' -v

python3 \
  handoff/pm4-task1-publication-recovery-20260824/acceptance/test_publication_script_contract.py \
  --r5-dir /absolute/path/to/R5
```

The implementation is not acceptable merely because helper tests pass. The complete isolated transaction must also pass `cargo metadata --frozen --format-version 1`, Task-1 tests, all-target compilation, Clippy, rustfmt, exact diff checks, and remote ref-drift gates before push.
