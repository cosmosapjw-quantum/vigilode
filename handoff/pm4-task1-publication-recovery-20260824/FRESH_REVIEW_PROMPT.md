# Fresh-context reviewer prompt — PM-4 Task-1 R5

You are a fresh reviewer. Do not trust the implementer's explanation. On the first pass, do not modify code.

Inputs:

- canonical main SHA;
- pre-repair and final feature SHA;
- `AUDIT_COMPILED_EXEC_PLAN.yaml`;
- `HANDOFF_COMPLETENESS_CORRECTION.json`;
- `templates/COMPLETION_EVIDENCE_SCHEMA.json`;
- `COMPLETION_EVIDENCE.json` and its validator logs;
- `P0_P1_THREAT_CATALOG.yaml`;
- final Git diff;
- R5 archive plus manifest and SHA-256;
- all RED/GREEN, Cargo metadata, test, Clippy, rustfmt, publication, and remote-verification logs.

Review only for:

1. contract violations;
2. P0/P1 modes omitted from the compiled contract;
3. claimed gates not run against published bytes or the actual host vendor;
4. any remote mutation beyond one ordinary non-force feature fast-forward;
5. hidden Task-1, Cargo config/lock/dependency, scientific-code, timing-authority, or claim-boundary changes.

Mandatory independent checks:

```bash
python3 -m unittest acceptance.test_handoff_completeness_contract -v
python3 -m unittest acceptance.test_completion_evidence_contract -v
python3 acceptance/test_completion_evidence_schema_contract.py --schema templates/COMPLETION_EVIDENCE_SCHEMA.json --instance COMPLETION_EVIDENCE.json
python3 acceptance/validate_completion_evidence.py --evidence COMPLETION_EVIDENCE.json
git merge-base --is-ancestor <main> <final-feature>
git diff --name-status <main>...<final-feature>
git show <final-feature>:<each-Task-1-file> | sha256sum
python3 acceptance/test_publication_script_contract.py --r5-dir <R5_DIR>
PM4_R5_DIR=<R5_DIR> python3 -m unittest acceptance.test_vendor_validator_contract -v
cargo metadata --frozen --format-version 1
```

Adversarial cases:

- valid 262 package-candidate vendor;
- valid 308 package-candidate vendor;
- hidden/manifestless incidental directories;
- one missing checksum;
- missing `faer-0.24.4`;
- stale main or feature ref;
- one-byte Task-1 mutation;
- fifth changed repository path;
- literal `262` reintroduced;
- missing repository-local contract reference;
- `*_SCHEMA.json` replaced by an example instance;
- missing required command record;
- merge/timing/Task2 flag set true in success evidence.

Archive authority must be exactly `6689544ee9b115fe4cb5c8ba14c179a17ee6615cb454555b0bb2f0ad1826b333`; the withdrawn `b33af0...` hash is not an alternative. Any ambiguity is P0.

Output only:

- `P0/P1/P2/P3` findings;
- exact file:line or artifact path;
- violated invariant;
- reproducer and observed result;
- required correction;
- final verdict `PASS_IMPLEMENTATION_REVIEW_READY` only when `P0=0` and `P1=0`.
