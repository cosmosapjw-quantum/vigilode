# PM-4 Task-1 publication failure chain

## Stable scientific/code payload

The Task-1 source payload has remained unchanged across R2, R3, and R4:

```text
PM4_TASK1_SCHEMA_BOUNDARY.patch
SHA-256 705646496b3594adb4f655829dfe2756aca57ce061fef0cae3b080399104f7a3
```

It modifies only `lib.rs`, adds the v3.8-D schema module and focused contract test, and removes the temporary recovery marker in the proposed feature tree.

## R2 — surface-accounting false blocker

Observed stop:

```text
STOP: changed-file surface mismatch
crates/rodas5p-integrators/src/lib.rs
```

Cause: plain `git diff --name-only` omitted two untracked additions and one already-staged deletion. R3 corrected this using `git apply --index` and `git diff --cached --name-status`.

Remote mutation: none.

## R3 — isolated-clone vendor path missing

Observed stop:

```text
failed to read root of directory source:
/tmp/vigilode-pm4-task1.<id>/rust-offline-rodas5p-rs-20260806/vendor
No such file or directory
```

Cause: the checked-in relative directory source resolves to a sibling of the temporary clone; R3 did not bridge the persistent vendor into that location. R4 added the symlink.

Remote mutation: none.

## R4 — historical exact-count false blocker

Actual host invocation:

```text
VENDOR_DIR=/home/cosmosapjw/Dropbox/rust/bundles/rust-offline-rodas5p-rs-20260806/vendor
rustc 1.94.1
cargo 1.94.1
STOP: Cargo vendor package-count mismatch: 308 (expected 262)
```

The R4 script had already passed archive checksums, toolchain identity, Task-1 file hashes, and staged surface checks. It stopped before commit creation or push.

Cause: R4 promoted a rehearsal-specific inventory count (`262`) into a universal correctness invariant. Cargo directory-source validity is not defined by a fixed total directory count. The correct load-bearing proof is structural checksum validation plus Cargo resolution under the checked-in source replacement in `--frozen` mode.

Remote mutation: none.

## Current remote state before Codex work

```text
main    140f6b5c078c3d8fcd5b6c52310c063ee233dc12
feature b2d5ec41cb147e01aadbc9c42928da8abfa75c58
PR #11  OPEN / DRAFT / UNMERGED / ZERO FILE DIFF
```

## Required lesson absorbed into the harness

Do not patch only the literal `262`. Permanently encode:

- valid 262 and 308 fixtures both pass;
- malformed checksum and missing required crate fixtures fail;
- Cargo `--frozen` metadata is required before push;
- observed inventory is evidence, not identity;
- receipts report actual counts and metadata digest;
- no remote mutation follows partial verification.

## Codex blocker: outer archive identity conflict

The first Codex run correctly stopped because the original handoff required `b33af0b8352aa0b3ccdcc83834cb4696fce787d0733a7e5ce9286e646994a095` while the authenticated local R4 archive and its sidecar both identify `6689544ee9b115fe4cb5c8ba14c179a17ee6615cb454555b0bb2f0ad1826b333`. No push occurred.

The conflict is resolved by `ARCHIVE_AUTHORITY_CORRECTION.json`: `6689544ee9b115fe4cb5c8ba14c179a17ee6615cb454555b0bb2f0ad1826b333` is now the sole accepted outer archive identity for this recovery. This is not a guess: the archive extracts, internal `SHA256SUMS` passes, the sealed Task-1 patch hash is `705646496b3594adb4f655829dfe2756aca57ce061fef0cae3b080399104f7a3`, and the embedded R4 publication script hash is `63c4ae3ca493a6b4ffe03db50a1b1e23850dacf5dcd0f502f594464cbd67ddb7`. The old handoff value is withdrawn and must never be accepted as an alternative.

## Second Codex blocker — dangling completion-evidence contract path

The next Codex run passed the corrected R4 archive authority gate, reproduced the R4 `308 (expected 262)` failure, and rechecked the unchanged remote refs. It then stopped because `IMPLEMENTER_PROMPT.md` required:

```text
templates/COMPLETION_EVIDENCE_SCHEMA.json
```

but that path was absent from the repository-local handoff branch. The file existed in an earlier downloadable full package, but the branch was the executable source of truth used by Codex. Codex correctly refused to invent the completion-evidence contract.

Remote mutation: none.

Permanent correction:

- publish the exact concrete canonical key/type template;
- bind its SHA-256 (`8341e8201a6b426dedecbda00b12816e0fddc36656cb94040250a21e41b37b29`);
- add `acceptance/test_completion_evidence_schema_contract.py`;
- require template validation before implementation and produced-instance validation before completion;
- treat any future dangling required path as a high-severity handoff-completeness failure.
