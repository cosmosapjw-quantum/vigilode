# VigilODE A1 Audit Full-E Evidence Closure — Agent Map

This branch is a **read-only handoff**. Do not merge it and do not implement on it.

## Mandatory read order

1. `CURRENT_STATE.json`
2. `SUPERSEDING_REPAIR_32906175896.md`
3. `README.md`
4. `WORKFLOW_PROVENANCE.md`
5. `AUDIT_COMPILED_EXEC_PLAN.yaml`
6. `P0_P1_THREAT_CATALOG.yaml`
7. `INVARIANT_TEST_MATRIX.yaml`
8. `IMPLEMENTER_PROMPT.md`
9. `FRESH_REVIEW_PROMPT.md`
10. `acceptance/test_handoff_contract.py`

## Implementation target

Work only on:

```text
research/a1-inner-tolerance-parity
```

Reuse PR #18. Do not open another PR.

## Supersession rule

`SUPERSEDING_REPAIR_32906175896.md`, the updated `CURRENT_STATE.json`, and the updated `IMPLEMENTER_PROMPT.md` supersede any conflicting older handoff prose.

## Governing principles

- Workflow run `32906175896` and aggregate digest `7665718c60ff9c1e0d1e86d1ff4464e8eb71d806dd0e6ce5c4f6ac0501f027a1` are diagostic evidence only. They are invalid for any authority decision.
- The ordinary committed arm remains `legacy-fixed` throughout this node.
- Runtime recommendation shadow evidence and independent audit full-E evidence are distinct channels. Never derive one from the other.
- Missing audit execution is unknown evidence, not `audit_unsafe = false` and not proof that the positive control disappeared.
- No scientific decision may be emitted until arm-specific audit full-E evidence is complete under the repaired schema.
- Any load-bearing schema, runner, or aggregation change invalidates the old scientific execution and requires a new `H_exec` and a fresh twelve-cell workflow.
- Historical v3.5–v3.7 artifacts are immutable legacy evidence. Never rewrite them to fit the candidate.
- No threshold, persistence, prefix-budget, continuation-budget, expected-output, GMRES-structure, dependency, or timing retuning.
- No A2/A3, G1/G3 expansion, wall-time ranking, speedup, active switching, candidate activation, tag, release, or merge.
- Preserve the cycle-free provenance model: `H_exec -> R_exec -> H_receipt -> external R_verify`.
- A SHA mismatch is only a byte-identity signal. Packaging hashes are not scientific classifiers.
- Do not ask the user questions. Do not guess across a scientific, API, dependency, or Git-history boundary. Use `BLOCKED_BY_UNRESOLVED_SPEC` with exact evidence when a genuine boundary cannot be resolved from the repository.
