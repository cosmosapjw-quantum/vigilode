# VigilODE A1 Two-Arm Authority Receipt — Agent Map

This branch is a **read-only handoff**. Do not merge it and do not implement on it.

## Mandatory read order

1. `CURRENT_STATE.json`
2. `README.md`
3. `WORKFLOW_PROVENANCE.md`
4. `AUDIT_COMPILED_EXEC_PLAN.yaml`
5. `P0_P1_THREAT_CATALOG.yaml`
6. `INVARIANT_TEST_MATRIX.yaml`
7. `IMPLEMENTER_PROMPT.md`
8. `FRESH_REVIEW_PROMPT.md`
9. `acceptance/test_handoff_contract.py`

## Implementation target

Work only on:

```text
research/a1-inner-tolerance-parity
```

Reuse PR #18. Do not open another PR.

## Governing principles

- The committed production arm remains `legacy-fixed` unless the complete receipt is classified `ADMISSIBLE_AND_DISCRIMINATING` under the predeclared rule.
- The outer-scaled arm is an experimental replay arm, not timing authority and not a proof of equal outer-error contribution.
- Historical v3.5–v3.7 artifacts are immutable legacy evidence. Never rewrite them to fit the candidate.
- No threshold, persistence, prefix-budget, continuation-budget, or expected-output retuning.
- No A2/A3, G1/G3 expansion, wall-time ranking, speedup, active switching, tag, release, or merge.
- Record candidate head, base, tested synthetic merge SHA/tree, toolchain, workflow run, and final artifact identity separately. Never call the PR merge `GITHUB_SHA` the feature head.
- A SHA mismatch is only a byte-identity signal. Packaging hashes are not scientific blockers. Git commit/tree/blob identity, deterministic content manifests, numerical invariants, and explicit provenance are the relevant gates.
- Do not ask the user questions. Do not guess across a scientific, API, dependency, or Git-history boundary. Use `BLOCKED_BY_UNRESOLVED_SPEC` with exact evidence when a real boundary cannot be resolved from the repository.
