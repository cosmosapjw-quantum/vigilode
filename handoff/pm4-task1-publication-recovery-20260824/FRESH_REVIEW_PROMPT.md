# Fresh-context reviewer prompt — PM-4 Task-1 R5

You are a fresh reviewer. Do not trust the implementer's explanation. On the first pass, do not modify code.

Inputs:

- canonical main SHA;
- pre-repair and final feature SHA;
- `AUDIT_COMPILED_EXEC_PLAN.yaml`;
- `P0_P1_THREAT_CATALOG.yaml`;
- final Git diff;
- R5 archive, manifest, and SHA-256;
- all RED/GREEN, Cargo metadata, test, Clippy, rustfmt, publication, and remote-verification logs.

Review only for:

1. a contract violation;
2. a P0/P1 failure mode omitted from the compiled contract;
3. evidence that a claimed gate did not run against the published bytes or actual 308-directory vendor;
4. remote mutation outside the exact normal fast-forward to the feature branch;
5. hidden changes to Task-1 bytes, Cargo configuration, lockfile, dependencies, scientific code, timing authority, or claim boundary.

Mandatory independent checks:

```bash
git merge-base --is-ancestor <main> <final-feature>
git diff --name-status <main>...<final-feature>
git show <final-feature>:<each Task-1 file> | sha256sum
cargo metadata --frozen --format-version 1
```

Adversarial cases:

- structurally valid 262-directory vendor;
- structurally valid 308-directory vendor;
- one missing checksum;
- missing `faer-0.24.4`;
- stale main ref;
- stale feature ref;
- one-byte Task-1 patch mutation;
- fifth changed repository path;
- literal `262` reintroduced in gate or receipt.

Output only:

- P0/P1/P2/P3 findings;
- exact file:line or artifact path;
- violated invariant;
- reproducer and observed result;
- required correction;
- final verdict `PASS_IMPLEMENTATION_REVIEW_READY` only when P0=0 and P1=0.
