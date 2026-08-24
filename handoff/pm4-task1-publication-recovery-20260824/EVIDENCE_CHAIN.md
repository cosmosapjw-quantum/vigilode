# PM-4 Task-1 publication failure chain

## Stable Task-1 payload

```text
PM4_TASK1_SCHEMA_BOUNDARY.patch
SHA-256 705646496b3594adb4f655829dfe2756aca57ce061fef0cae3b080399104f7a3
```

The payload remained unchanged throughout R2–R4 and both Codex blockers.

## R2 — surface-accounting false blocker

```text
STOP: changed-file surface mismatch
crates/rodas5p-integrators/src/lib.rs
```

Plain `git diff --name-only` omitted two untracked additions and one staged deletion. The permanent guard is index-aware application plus exact staged/final M/A/A/D checks. Remote mutation: none.

## R3 — isolated-clone vendor bridge missing

Cargo resolved the checked-in relative directory source to a sibling of the temporary clone, but R3 did not create that sibling link. The permanent guard is exact `readlink -f` equality before Cargo. Remote mutation: none.

## R4 — historical exact-count false blocker

```text
STOP: Cargo vendor package-count mismatch: 308 (expected 262)
```

A rehearsal-specific inventory count became a universal correctness condition. The permanent guard is structural package-candidate validation plus `cargo metadata --frozen --format-version 1`; observed counts remain evidence only. Remote mutation: none.

## First Codex blocker — archive authority conflict

The original handoff required `b33af0b8352aa0b3ccdcc83834cb4696fce787d0733a7e5ce9286e646994a095`, while the authenticated host archive and matching sidecar identify:

```text
6689544ee9b115fe4cb5c8ba14c179a17ee6615cb454555b0bb2f0ad1826b333
```

Codex correctly stopped. `ARCHIVE_AUTHORITY_CORRECTION.json` now makes `668954...b333` the sole accepted hash; the prior value is withdrawn. Internal `SHA256SUMS`, the sealed Task-1 patch, and R4 script hashes are also required. Remote mutation: none.

## Second Codex blocker — dangling completion-evidence contract

The corrected archive gate passed and the R4 count failure was reproduced. Codex then found that `IMPLEMENTER_PROMPT.md` required:

```text
templates/COMPLETION_EVIDENCE_SCHEMA.json
```

but the executable repository-local handoff branch omitted it. Codex correctly refused to invent an evidence contract. Remote mutation: none.

## Permanent referential-closure correction

The handoff now includes:

- a real JSON Schema Draft 2020-12 completion-evidence contract;
- a positive completion example;
- a formal vendor-validation schema and example;
- a pure-standard-library semantic completion validator;
- positive/negative completion-evidence tests;
- a scan that rejects every dangling `templates/` or `acceptance/` reference in load-bearing handoff documents;
- an aggregate acceptance runner.

Before R5 work:

```bash
python3 -m unittest acceptance.test_handoff_completeness_contract -v
python3 -m unittest acceptance.test_completion_evidence_contract -v
python3 acceptance/test_completion_evidence_schema_contract.py --schema templates/COMPLETION_EVIDENCE_SCHEMA.json --instance templates/COMPLETION_EVIDENCE_EXAMPLE.json
```

After successful publication:

```bash
python3 acceptance/validate_completion_evidence.py --evidence COMPLETION_EVIDENCE.json
```

Partial or blocked execution must not emit success evidence.

## Current production authority

```text
main    140f6b5c078c3d8fcd5b6c52310c063ee233dc12
feature b2d5ec41cb147e01aadbc9c42928da8abfa75c58
PR #11  OPEN / DRAFT / UNMERGED / ZERO FILE DIFF
```

No merge, wall timing, candidate ranking, or Task 2 occurred in the failure chain.
