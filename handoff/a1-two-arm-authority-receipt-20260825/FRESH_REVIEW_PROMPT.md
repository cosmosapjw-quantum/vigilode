# FRESH REVIEW PROMPT — PR #18 A1 Two-Arm Receipt

Use a fresh context. Review only; do not fix code in the first pass.

## Inputs

Scientific execution evidence:

- canonical base SHA/tree;
- frozen `scientific_execution_head_sha/tree`;
- tested execution merge SHA/tree;
- execution workflow run ID/attempt and logs;
- all 12 atomic cell JSON files;
- artifact content manifest;
- aggregate JSON/Markdown.

Receipt closure evidence:

- later `receipt_commit_sha/tree`;
- full `base..receipt_commit` diff;
- committed receipt JSON/Markdown;
- post-receipt A1, E4, and receipt-validation run IDs/logs;
- forbidden-diff report;
- this handoff contract.

Do not trust the implementation agent's explanation. Recompute the scientific
and provenance verdict from source and atomic evidence.

## Mandatory questions

1. Does the ordinary committed path remain legacy-fixed throughout the receipt
   node?
2. Is the candidate reachable only through a receipt-only path?
3. Are there exactly two arms and six unique families under
   `EnforcedBudgetHoldout320`?
4. Are tau, persistence, prefix budget, continuation budget, GMRES structure,
   and historical results unchanged?
5. Are aggregate totals and sets recomputed from atomic rows?
6. Are all event keys, finite zeta34 values, signed margins, recommendations,
   unsafe recommendations, and audit unsafe events retained?
7. Is the Hires positive control explicitly and correctly classified for both
   arms?
8. Is the decision exactly one of the predeclared classes and mechanically
   implied by the evidence?
9. Is wall time excluded from scientific authority and deterministic identity?
10. Do cells, aggregate, artifacts, and committed receipt bind the same frozen
    scientific execution head/tree, tested execution merge, toolchain, and
    execution workflow?
11. Is the receipt commit a descendant of the scientific execution head, with
    no load-bearing source or aggregation-semantic mutation after execution?
12. Does the tracked receipt avoid self-embedding its receipt commit/tree and
    post-receipt workflow run IDs?
13. Do external A1/E4/receipt-validation records bind exactly the later receipt
    commit?
14. Did any test, threshold, expected output, or historical receipt change merely
    to obtain a pass?
15. Are activation, A2/A3, G1/G3 extension, ranking, speedup, switching, tag,
    release, and merge absent?

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

`APPROVE_RECEIPT_ONLY` means the receipt and authority decision are
review-ready. It does not authorize activation, merge, performance claims, or
A2/A3.
