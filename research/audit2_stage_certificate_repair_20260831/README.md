# Stage-certificate repair closeout: local Codex handoff

This is a control-only successor to Draft PR #41. It contains no repaired science,
raw backend output, candidate run, or claim promotion. It binds the observed
defects from the immutable STOP_INVALID run and delegates the bounded repair to
the user's local Codex job.

Read `CODEX_START_HERE.md`, then `EXECUTION_CONTRACT.json` (the sole
machine-readable execution authority), `FORMAL_SCOPE.md`, `CLAIM_LEDGER.md`,
and `handoff.json`.

The four supplied universal policy documents are not copied into the repository.
Their operative rules are recorded under `policy_adoption`: progress-first
claim discipline, compiled phase budgets, typed identity, durable resume, one
diagnostic retry, and one integrated review.

Validate without running science:

```bash
python3 tools/validate_audit2_stage_certificate_repair_handoff.py
python3 tools/test_audit2_stage_certificate_repair_handoff.py -v
```

The local job preserves the predecessor run and worktree, creates a fresh
external successor run, pushes only to the Draft PR named by `handoff.json`,
and keeps the claim ceiling unchanged.
