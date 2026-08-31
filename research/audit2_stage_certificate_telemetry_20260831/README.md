# Candidate-free stage-certificate telemetry: local Codex handoff

This directory is a small, analysis-only execution contract. It does not
recover the workspace-pruned implementation and it contains no candidate,
holdout, solver, or formal-backend result.

The next node is intentionally delegated to a fresh local Codex job. That job
must generate the feature-gated synthetic telemetry contract and the F01--F05
formal evidence directly from the published Draft PR head. Raw stdout,
compiler output, caches, and intermediate data remain outside Git. Only source,
tests, proof source, normalized analysis, hashes, and compact receipts may be
committed.

Start with `CODEX_START_HERE.md`. The machine-readable controls are:

- `EXECUTION_CONTRACT.json`: allowed work, stop rules, tool and storage policy;
- `PUBLICATION_SCHEMA.json`: compact result and formal-receipt schemas;
- `handoff.json`: repository/stack/publication handoff;
- `FORMAL_SCOPE.md`: exactly F01--F05 and their nonclaims;
- `RAW_DATA_POLICY.md`: external raw-data and checked-in size boundary;
- `CLAIM_LEDGER.md`: claims that can and cannot change.

The handoff itself requires:

```bash
python3 tools/validate_audit2_stage_certificate_handoff.py
python3 tools/test_audit2_stage_certificate_handoff.py -v
```

Publication control C1 is commit
`193dcb8c0fb7c1042183739ecef627ae5df38612`, tree
`f40f7f3a43d7ad24c28142ea61ba2e3698d13030`, in Draft PR #41. The
documentation-binding C2 may advance the PR head but must retain C1 as an
ancestor and pass `HANDOFF_INPUT_LOCK.json` readback.

The local job must keep the claim ceiling unchanged:

`EXPLORATORY_NONAUTHORITATIVE_REUSABLE_PRECONDITIONER_TRANSACTIONAL_STEP_SUBSTRATE`
