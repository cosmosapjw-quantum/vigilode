# FRESH REVIEW PROMPT — PR #18 A1 Two-Arm Receipt

Use a fresh context. Review only; do not fix code in the first pass.

Inputs:

- canonical base SHA;
- final candidate SHA/tree;
- full `base..head` diff;
- this handoff contract;
- A1, E4, and two-arm workflow logs;
- all 12 atomic cell JSON files;
- aggregate JSON/Markdown;
- committed receipt JSON/Markdown;
- forbidden-diff report.

Do not trust the implementation agent's explanation. Recompute the scientific and provenance verdict from source and atomic evidence.

## Mandatory questions

1. Does the ordinary committed path remain legacy-fixed until the explicit decision?
2. Is the candidate reachable only through a receipt-only path?
3. Are there exactly two arms and six unique families under `EnforcedBudgetHoldout320`?
4. Are tau, persistence, prefix budget, continuation budget, GMRES structure, and historical results unchanged?
5. Are aggregate totals and sets recomputed from atomic rows?
6. Are all event keys, finite zeta34 values, signed margins, recommendations, unsafe recommendations, and audit unsafe events retained?
7. Is the Hires positive control explicitly and correctly classified for both arms?
8. Is the decision exactly one of the predeclared classes and mechanically implied by the evidence?
9. Is wall time excluded from scientific authority and deterministic identity?
10. Do workflow artifacts and committed receipts bind the same source/head/tree/toolchain and cell content?
11. Did any test, threshold, expected output, or historical receipt change merely to obtain a pass?
12. Are A2/A3, G1/G3 extension, ranking, speedup, switching, tag, release, and merge absent?

## Output

First produce findings only:

```text
P0/P1/P2/P3
exact file:line or artifact path
violated invariant
reproducer
scientific or provenance impact
minimal correction boundary
```

Then give one verdict:

```text
APPROVE_RECEIPT_ONLY
CHANGES_REQUIRED
BLOCKED_BY_UNRESOLVED_SPEC
```

`APPROVE_RECEIPT_ONLY` means the receipt and authority decision are review-ready. It does not authorize merge, performance claims, A2/A3, or switching beyond what the committed decision explicitly supports.
