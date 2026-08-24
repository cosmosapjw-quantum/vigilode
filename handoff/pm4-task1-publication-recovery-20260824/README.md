# PM-4 Task-1 Codex recovery handoff — archive-authority correction applied

This branch contains the machine-checkable handoff for repairing the R4 Cargo-vendor gate and publishing the already-sealed PM-4 Task-1 source patch to draft PR #11.

## Read order

Fetch the branch without merging it:

```bash
git fetch origin handoff/pm4-task1-publication-recovery-20260824
```

Then read:

```text
handoff/pm4-task1-publication-recovery-20260824/AGENTS.md
handoff/pm4-task1-publication-recovery-20260824/CURRENT_STATE.json
handoff/pm4-task1-publication-recovery-20260824/ARCHIVE_AUTHORITY_CORRECTION.json
handoff/pm4-task1-publication-recovery-20260824/AUDIT_COMPILED_EXEC_PLAN.yaml
handoff/pm4-task1-publication-recovery-20260824/P0_P1_THREAT_CATALOG.yaml
handoff/pm4-task1-publication-recovery-20260824/INVARIANT_TEST_MATRIX.yaml
handoff/pm4-task1-publication-recovery-20260824/EVIDENCE_CHAIN.md
handoff/pm4-task1-publication-recovery-20260824/acceptance/README.md
handoff/pm4-task1-publication-recovery-20260824/IMPLEMENTER_PROMPT.md
handoff/pm4-task1-publication-recovery-20260824/FRESH_REVIEW_PROMPT.md
```

## R4 archive authority correction

The authenticated host contains one R4 archive and matching sidecar:

```text
~/vigilode/VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz
SHA-256 6689544ee9b115fe4cb5c8ba14c179a17ee6615cb454555b0bb2f0ad1826b333
```

The earlier handoff value:

```text
b33af0b8352aa0b3ccdcc83834cb4696fce787d0733a7e5ce9286e646994a095
```

is withdrawn for this recovery. Codex must not choose between contradictory hashes. It must require `668954...b333` exactly, run the local sidecar check, extract the archive, verify internal `SHA256SUMS`, and verify:

```text
Task-1 patch SHA-256
705646496b3594adb4f655829dfe2756aca57ce061fef0cae3b080399104f7a3

R4 publication script SHA-256
63c4ae3ca493a6b4ffe03db50a1b1e23850dacf5dcd0f502f594464cbd67ddb7
```

The first Codex run correctly stopped before mutation because the old handoff was internally contradictory. That failure is now a permanent P0 archive-authority regression gate.

## Actual Cargo source

```text
/home/cosmosapjw/Dropbox/rust/bundles/rust-offline-rodas5p-rs-20260806/vendor
```

Cargo package candidates must be classified at one immediate directory level: ignore dot-prefixed directories, count only directories containing `Cargo.toml`, require a parseable `.cargo-checksum.json` for every candidate, and let `cargo metadata --frozen --format-version 1` prove locked offline closure. No exact global package count is authority.

## Boundaries

- target PR: #11
- canonical main: `140f6b5c078c3d8fcd5b6c52310c063ee233dc12`
- expected feature head before Task-1 publication: `b2d5ec41cb147e01aadbc9c42928da8abfa75c58`
- no force push, merge, wall timing, candidate ranking, or PM-4 Task 2
- the Task-1 source patch is immutable
- this handoff branch is documentation/transport only and must not be merged as part of Task-1 publication
