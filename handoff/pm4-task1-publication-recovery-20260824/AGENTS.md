# PM-4 Task-1 recovery map

Read in this order:

1. `README.md`
2. `CURRENT_STATE.json`
3. `AUDIT_COMPILED_EXEC_PLAN.yaml`
4. `P0_P1_THREAT_CATALOG.yaml`
5. `INVARIANT_TEST_MATRIX.yaml`
6. `EVIDENCE_CHAIN.md`
7. `acceptance/README.md`
8. `IMPLEMENTER_PROMPT.md`

Rules:

- Do not guess across a specification boundary.
- Do not merge this handoff branch or PR #12.
- Do not mutate `main`, force-push, merge PR #11, run wall timing, rank candidates, or start Task 2.
- Do not change the sealed Task-1 patch, Cargo configuration, lockfile, or dependencies.
- A prose warning is not closure for P0/P1. Require an executable test, mechanical invariant, or explicit STOP gate.
- If a required fact cannot be established from repository state, local artifacts, or commands, stop with `BLOCKED_BY_UNRESOLVED_SPEC` and preserve evidence.
