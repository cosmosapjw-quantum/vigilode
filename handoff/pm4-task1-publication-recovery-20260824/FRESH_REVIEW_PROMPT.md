# Fresh-context reviewer prompt — PM-4 Task-1 R5

You are a fresh reviewer. Do not trust the implementer's explanation. On the first pass, do not modify code.

Inputs:

- canonical main SHA;
- pre-repair and final feature SHA;
- `ARCHIVE_AUTHORITY_CORRECTION.json`;
- `AUDIT_COMPILED_EXEC_PLAN.yaml`;
- `P0_P1_THREAT_CATALOG.yaml`;
- final Git diff;
- R4 and R5 archive manifests and SHA-256 values;
- all RED/GREEN, Cargo metadata, test, Clippy, rustfmt, publication, and remote-verification logs.

Review only for:

1. a contract violation;
2. a P0/P1 failure mode omitted from the compiled contract;
3. evidence that a claimed gate did not actually run against the published bytes or actual vendor;
4. any remote mutation outside the exact normal fast-forward to the feature branch;
5. any hidden change to Task-1 bytes, Cargo configuration, lockfile, dependencies, scientific code, timing authority, or claim boundary.

Mandatory independent archive-authority check:

```text
accepted R4 outer SHA-256
6689544ee9b115fe4cb5c8ba14c179a17ee6615cb454555b0bb2f0ad1826b333

withdrawn prior declaration — must not be accepted
b33af0b8352aa0b3ccdcc83834cb4696fce787d0733a7e5ce9286e646994a095
```

Verify the local sidecar, internal `SHA256SUMS`, Task-1 patch hash `705646496b3594adb4f655829dfe2756aca57ce061fef0cae3b080399104f7a3`, and R4 publication-script hash `63c4ae3ca493a6b4ffe03db50a1b1e23850dacf5dcd0f502f594464cbd67ddb7`. Any ambiguity is P0.

Mandatory independent commands include:

```bash
python3 acceptance/test_archive_authority_contract.py --archive <R4_ARCHIVE> --sidecar <R4_SIDECAR>
git merge-base --is-ancestor <main> <final-feature>
git diff --name-status <main>...<final-feature>
git show <final-feature>:<each Task-1 file> | sha256sum
python3 acceptance/test_publication_script_contract.py --r5-dir <R5_DIR>
PM4_R5_DIR=<R5_DIR> python3 -m unittest acceptance.test_vendor_validator_contract -v
cargo metadata --frozen --format-version 1
```

Adversarial cases to reproduce:

- wrong R4 outer hash;
- altered sidecar;
- one altered internal member;
- structurally valid 262-package vendor;
- structurally valid 308-package vendor;
- hidden and manifestless incidental directories;
- one missing checksum;
- missing `faer-0.24.4`;
- stale main ref;
- stale feature ref;
- one-byte Task-1 patch mutation;
- fifth changed repository path;
- literal `262` or withdrawn `b33af0...` reintroduced in receipt or gate.

Output only:

- `P0/P1/P2/P3` findings;
- exact file:line or artifact path;
- violated invariant;
- reproducer and observed result;
- required correction;
- final verdict: `PASS_IMPLEMENTATION_REVIEW_READY` only when `P0=0` and `P1=0`.
