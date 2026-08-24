# PM-4 Task-1 recovery map

Read in this order:

1. `README.md`
2. `CURRENT_STATE.json`
3. `ARCHIVE_AUTHORITY_CORRECTION.json`
4. `AUDIT_COMPILED_EXEC_PLAN.yaml`
5. `P0_P1_THREAT_CATALOG.yaml`
6. `INVARIANT_TEST_MATRIX.yaml`
7. `EVIDENCE_CHAIN.md`
8. `acceptance/README.md`
9. `IMPLEMENTER_PROMPT.md`

Rules:

- Do not guess across a specification boundary.
- Do not modify canonical `main`, force-push, merge PR #11, run wall timing, rank candidates, or start Task 2.
- Do not change the sealed Task-1 patch, Cargo configuration, lockfile, or dependencies.
- The sole accepted R4 outer archive SHA-256 is `6689544ee9b115fe4cb5c8ba14c179a17ee6615cb454555b0bb2f0ad1826b333`; the earlier `b33af0...` declaration is withdrawn and is not an alternative.
- A prose warning is not closure for P0/P1; require an executable test, mechanical invariant, or explicit STOP gate.
- If a required fact cannot be established from repository state, files, or commands, stop with `BLOCKED_BY_UNRESOLVED_SPEC` and preserve evidence.
