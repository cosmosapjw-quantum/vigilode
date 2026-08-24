# PM-4 Task-1 Codex recovery handoff

This branch contains the machine-checkable handoff for repairing the R4 Cargo-vendor gate and publishing the already-sealed PM-4 Task-1 source patch to draft PR #11.

## Read order

Fetch the branch without merging it:

```bash
git fetch origin handoff/pm4-task1-publication-recovery-20260824
```

Then read these files directly from the branch, or check it out in a separate worktree:

```text
handoff/pm4-task1-publication-recovery-20260824/README.md
handoff/pm4-task1-publication-recovery-20260824/AUDIT_COMPILED_EXEC_PLAN.yaml
handoff/pm4-task1-publication-recovery-20260824/P0_P1_THREAT_CATALOG.yaml
handoff/pm4-task1-publication-recovery-20260824/INVARIANT_TEST_MATRIX.yaml
handoff/pm4-task1-publication-recovery-20260824/EVIDENCE_CHAIN.md
handoff/pm4-task1-publication-recovery-20260824/acceptance/test_vendor_validator_contract.py
handoff/pm4-task1-publication-recovery-20260824/acceptance/test_publication_script_contract.py
handoff/pm4-task1-publication-recovery-20260824/IMPLEMENTER_PROMPT.md
handoff/pm4-task1-publication-recovery-20260824/FRESH_REVIEW_PROMPT.md
```

The failing R4 input archive is already expected on the local host at:

```text
~/vigilode/VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz
```

The actual Cargo directory source is:

```text
/home/cosmosapjw/Dropbox/rust/bundles/rust-offline-rodas5p-rs-20260806/vendor
```

## Boundaries

- target PR: #11
- canonical main: `140f6b5c078c3d8fcd5b6c52310c063ee233dc12`
- expected feature head before Task-1 publication: `b2d5ec41cb147e01aadbc9c42928da8abfa75c58`
- no force push, merge, wall timing, candidate ranking, or PM-4 Task 2
- the Task-1 source patch is immutable
- this handoff branch is documentation/transport only and must not be merged as part of Task-1 publication
