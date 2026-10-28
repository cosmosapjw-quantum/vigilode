# Task 4 report: t-verify
Status: DONE_WITH_CONCERNS (concern: checker exit 1 from pre-existing L-0124 hash mismatches, outside the node).
Branch production/t-verify (pushed). Commits: 5811ca9 (preregistration, before run), 81069f9 (script + RESULTS.json; the ledger `commit`), f2250b7 (ledger row L-0137 + docs/plan/nodes/t-verify.md).
Result: all 6 claims PASS; ledger verdict PASS. Details with observed values: docs/plan/nodes/t-verify.md.
Checker: tools/check-research-node.py --base origin/production/vigilode-plan-20261021 exit 1; only failures are L-0124 outputs research/production_audit_20261018/CONTINUATION_DAG.json and CONTINUATION_FINDINGS.json sha256 mismatches (dirs identical to base). Not modified.
Plan corrections (not edited): Cor 2a identity T=(1-h/h_old)(P^-1 - I) has the sign of P^-1 W - I (norm unaffected); the scalar constant is t/(1+t)<1, so the factor 2 is loose; Lemma 3 monotonicity is grid evidence (10^4 pts, 50 digits), not a proof.
A dry run to a scratch path preceded the recorded run (disclosed in the prereg Results section).
