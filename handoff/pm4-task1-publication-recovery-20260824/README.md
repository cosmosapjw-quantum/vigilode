# PM-4 Task-1 Codex recovery handoff — archive and handoff-completeness corrections applied

This branch contains the machine-checkable handoff for repairing the R4 Cargo-vendor gate and publishing the already-sealed PM-4 Task-1 source patch to draft PR #11.

## Read order

Fetch the branch without merging it, then read:

```text
handoff/pm4-task1-publication-recovery-20260824/AGENTS.md
handoff/pm4-task1-publication-recovery-20260824/CURRENT_STATE.json
handoff/pm4-task1-publication-recovery-20260824/ARCHIVE_AUTHORITY_CORRECTION.json
handoff/pm4-task1-publication-recovery-20260824/HANDOFF_COMPLETENESS_CORRECTION.json
handoff/pm4-task1-publication-recovery-20260824/templates/COMPLETION_EVIDENCE_SCHEMA.json
handoff/pm4-task1-publication-recovery-20260824/AUDIT_COMPILED_EXEC_PLAN.yaml
handoff/pm4-task1-publication-recovery-20260824/P0_P1_THREAT_CATALOG.yaml
handoff/pm4-task1-publication-recovery-20260824/INVARIANT_TEST_MATRIX.yaml
handoff/pm4-task1-publication-recovery-20260824/EVIDENCE_CHAIN.md
handoff/pm4-task1-publication-recovery-20260824/acceptance/README.md
handoff/pm4-task1-publication-recovery-20260824/IMPLEMENTER_PROMPT.md
handoff/pm4-task1-publication-recovery-20260824/FRESH_REVIEW_PROMPT.md
```

## R4 archive authority

The sole accepted authenticated-host R4 archive identity is:

```text
~/vigilode/VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz
SHA-256 6689544ee9b115fe4cb5c8ba14c179a17ee6615cb454555b0bb2f0ad1826b333
```

The earlier `b33af0b8352aa0b3ccdcc83834cb4696fce787d0733a7e5ce9286e646994a095` declaration is withdrawn and is not an alternative. The archive sidecar, internal `SHA256SUMS`, sealed Task-1 patch SHA `705646496b3594adb4f655829dfe2756aca57ce061fef0cae3b080399104f7a3`, and R4 script SHA `63c4ae3ca493a6b4ffe03db50a1b1e23850dacf5dcd0f502f594464cbd67ddb7` must all pass.

## Handoff-completeness correction

A second Codex run correctly stopped before mutation because `IMPLEMENTER_PROMPT.md` required `templates/COMPLETION_EVIDENCE_SCHEMA.json`, while the repository-local handoff branch omitted that file. The path is now present and mechanically bound:

```text
templates/COMPLETION_EVIDENCE_SCHEMA.json
SHA-256 8341e8201a6b426dedecbda00b12816e0fddc36656cb94040250a21e41b37b29
```

Validate it before implementation:

```bash
python3 acceptance/test_completion_evidence_schema_contract.py \
  --schema templates/COMPLETION_EVIDENCE_SCHEMA.json
```

The file is a concrete canonical key/type template. On successful publication, the produced `COMPLETION_EVIDENCE.json` must have the exact key set and types, contain observed lowercase hashes rather than placeholders, retain all fixed PASS/false boundaries, have `unresolved_blockers=[]`, and pass the same validator with `--instance`.

## Actual Cargo source

```text
/home/cosmosapjw/Dropbox/rust/bundles/rust-offline-rodas5p-rs-20260806/vendor
```

Cargo package candidates must be classified at one immediate level: ignore dot-prefixed directories, count only directories containing `Cargo.toml`, require a parseable `.cargo-checksum.json` for each candidate, and let `cargo metadata --frozen --format-version 1` prove locked offline dependency closure. No exact global package count is authority.

## Boundaries

- target PR: #11
- canonical main: `140f6b5c078c3d8fcd5b6c52310c063ee233dc12`
- expected feature head before Task-1 publication: `b2d5ec41cb147e01aadbc9c42928da8abfa75c58`
- no force push, merge, wall timing, candidate ranking, or PM-4 Task 2
- the Task-1 source patch is immutable
- this handoff branch and PR #12 are documentation/transport only and must not be merged as part of Task-1 publication
