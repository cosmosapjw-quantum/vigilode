# PM-4 Task-1 final Codex handoff map

Read in this order:

1. `README.md`
2. `IDENTITY_POLICY.json`
3. `HASH_IDENTITY_POLICY.md`
4. `AUDIT_COMPILED_EXEC_PLAN.yaml`
5. `P0_P1_THREAT_CATALOG.yaml`
6. `INVARIANT_TEST_MATRIX.yaml`
7. `acceptance/README.md`
8. `IMPLEMENTER_PROMPT.md`

Before touching the implementation branch, run:

```bash
bash acceptance/run_preflight.sh
```

The handoff is Git-native. Do not reconstruct or authenticate a tar/zip/wheel.
The tracked patch blob, Git refs, final diff, Cargo closure, and scientific/code
checks are the authority.

Do not ask the user questions. Do not guess across a scientific or Git-history
boundary. Environment and path details may be discovered and adapted when the
underlying invariant is mechanically verified.
