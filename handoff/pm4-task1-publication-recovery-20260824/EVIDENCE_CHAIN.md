# PM-4 Task-1 publication failure chain

## Stable payload

The Task-1 source payload remained unchanged across R2, R3, and R4:

```text
PM4_TASK1_SCHEMA_BOUNDARY.patch
SHA-256 705646496b3594adb4f655829dfe2756aca57ce061fef0cae3b080399104f7a3
```

## R2 — surface-accounting false blocker

Observed:

```text
STOP: changed-file surface mismatch
crates/rodas5p-integrators/src/lib.rs
```

Cause: plain `git diff --name-only` omitted two untracked additions and one staged deletion. R3 corrected this with index-aware application and `git diff --cached --name-status`.

Remote mutation: none.

## R3 — isolated-clone vendor sibling missing

Observed:

```text
failed to read root of directory source:
/tmp/vigilode-pm4-task1.<id>/rust-offline-rodas5p-rs-20260806/vendor
No such file or directory
```

Cause: checked-in `.cargo/config.toml` resolves its relative directory source to a sibling of the temporary clone. R3 did not bridge the persistent vendor. R4 added the symlink.

Remote mutation: none.

## R4 — historical exact-count false blocker

Actual host evidence:

```text
VENDOR_DIR=/home/cosmosapjw/Dropbox/rust/bundles/rust-offline-rodas5p-rs-20260806/vendor
rustc 1.94.1
cargo 1.94.1
STOP: Cargo vendor package-count mismatch: 308 (expected 262)
```

R4 had already passed archive checksums, toolchain identity, Task-1 file hashes, and staged surface checks. It stopped before commit creation or push.

Root cause: a rehearsal-specific inventory count (`262`) was promoted into a universal directory-source correctness invariant. The correct load-bearing proof is:

1. structural `.cargo-checksum.json` validation;
2. required observed package presence;
3. exact sibling symlink resolution;
4. Cargo dependency resolution under the checked-in config using `cargo metadata --frozen --format-version 1`;
5. existing Task-1 tests and remote ref gates.

Remote mutation: none.

## Current remote authority before Codex execution

```text
main    140f6b5c078c3d8fcd5b6c52310c063ee233dc12
feature b2d5ec41cb147e01aadbc9c42928da8abfa75c58
PR #11  OPEN / DRAFT / UNMERGED / ZERO FILE DIFF
```

## Permanent harness lesson

Do not patch only the literal `262`. Encode the failure class permanently:

- valid 262 and 308 fixtures pass;
- missing checksum and missing required crate fixtures fail;
- Cargo `--frozen` metadata is mandatory before push;
- observed inventory is evidence, not identity;
- receipts report actual counts and metadata digest;
- no remote mutation follows partial verification.
