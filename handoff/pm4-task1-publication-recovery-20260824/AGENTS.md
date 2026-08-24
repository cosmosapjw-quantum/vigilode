# PM-4 Task-1 recovery map

Read in this order:

1. `README.md`
2. `CURRENT_STATE.json`
3. `ARCHIVE_AUTHORITY_CORRECTION.json`
4. `HANDOFF_COMPLETENESS_CORRECTION.json`
5. `templates/COMPLETION_EVIDENCE_SCHEMA.json`
6. `AUDIT_COMPILED_EXEC_PLAN.yaml`
7. `P0_P1_THREAT_CATALOG.yaml`
8. `INVARIANT_TEST_MATRIX.yaml`
9. `EVIDENCE_CHAIN.md`
10. `acceptance/README.md`
11. `IMPLEMENTER_PROMPT.md`

Before implementation, run both control-plane gates:

```bash
python3 acceptance/test_archive_authority_contract.py \
  --archive "$PM4_R4_ARCHIVE" \
  --sidecar "$PM4_R4_SIDECAR"

python3 acceptance/test_completion_evidence_schema_contract.py \
  --schema templates/COMPLETION_EVIDENCE_SCHEMA.json
```

Rules:

- Do not guess across a specification boundary.
- Every path required by the execution prompt must exist and pass its acceptance contract before implementation.
- Do not modify canonical `main`, force-push, merge PR #11, run wall timing, rank candidates, or start Task 2.
- Do not change the sealed Task-1 patch, Cargo configuration, lockfile, or dependencies.
- The sole accepted R4 outer archive SHA-256 is `6689544ee9b115fe4cb5c8ba14c179a17ee6615cb454555b0bb2f0ad1826b333`; the earlier `b33af0...` declaration is withdrawn and is not an alternative.
- `templates/COMPLETION_EVIDENCE_SCHEMA.json` is a concrete canonical key/type template. Copy every key exactly, replace placeholders with observed values, and validate the produced instance mechanically.
- A prose warning is not closure for P0/P1; require an executable test, mechanical invariant, or explicit STOP gate.
- If a required fact or artifact cannot be established from repository state, files, or commands, stop with `BLOCKED_BY_UNRESOLVED_SPEC` and preserve evidence.
