# PM-4 Task-1 recovery map — referential closure enforced

Read in this order:

1. `README.md`
2. `CURRENT_STATE.json`
3. `ARCHIVE_AUTHORITY_CORRECTION.json`
4. `HANDOFF_COMPLETENESS_CORRECTION.json`
5. `AUDIT_COMPILED_EXEC_PLAN.yaml`
6. `P0_P1_THREAT_CATALOG.yaml`
7. `INVARIANT_TEST_MATRIX.yaml`
8. `EVIDENCE_CHAIN.md`
9. `templates/COMPLETION_EVIDENCE_SCHEMA.json`
10. `templates/COMPLETION_EVIDENCE_EXAMPLE.json`
11. `templates/VENDOR_VALIDATION_SCHEMA.json`
12. `acceptance/README.md`
13. `IMPLEMENTER_PROMPT.md`

Before any R5 implementation, run:

```bash
python3 -m unittest acceptance.test_handoff_completeness_contract -v
python3 -m unittest acceptance.test_completion_evidence_contract -v
python3 acceptance/test_completion_evidence_schema_contract.py \
  --schema templates/COMPLETION_EVIDENCE_SCHEMA.json \
  --instance templates/COMPLETION_EVIDENCE_EXAMPLE.json
```

Rules:

- Do not guess across a specification boundary.
- Every repository-local file named by a prompt or execution contract must exist and parse before execution begins.
- Files named `*_SCHEMA.json` must be actual JSON Schema Draft 2020-12 documents, not prose-shaped examples.
- Do not modify canonical `main`, force-push, merge PR #11, run wall timing, rank candidates, or start Task 2.
- Do not change the sealed Task-1 patch, Cargo configuration, lockfile, or dependencies.
- The sole accepted R4 outer archive SHA-256 is `6689544ee9b115fe4cb5c8ba14c179a17ee6615cb454555b0bb2f0ad1826b333`; the earlier `b33af0b8352aa0b3ccdcc83834cb4696fce787d0733a7e5ce9286e646994a095` declaration is withdrawn and is not an alternative.
- A successful `COMPLETION_EVIDENCE.json` must conform to the formal schema and pass `acceptance/validate_completion_evidence.py`; blocked or partial runs must not emit success evidence.
- A prose warning is not closure for P0/P1; require an executable test, mechanical invariant, or explicit STOP gate.
- If a required fact or artifact cannot be established from repository state, files, or commands, stop with `BLOCKED_BY_UNRESOLVED_SPEC` and preserve evidence.
