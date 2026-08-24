# Fresh-context reviewer prompt — PM-4 Task-1

Review only. Do not repair on the first pass and do not trust the implementer's
summary. Inspect the actual repository, Git diff, remote refs, and logs.

Inputs:

- handoff contract and identity policy;
- canonical main and previous feature SHA;
- final feature SHA/tree;
- `main...final` diff;
- required logs and `COMPLETION_EVIDENCE.json`.

Verify independently:

1. source authority was established through Git refs/tree/blob and actual diff;
2. no packaging SHA mismatch was misclassified as a scientific blocker;
3. any working-tree byte mismatch was traced through attributes/filters/LFS or
   shown absent before commit;
4. the tracked payload was not modified;
5. final diff is exactly the declared M/A/A/D four-file surface;
6. all authority flags remain false and no benchmark result is fabricated;
7. Cargo metadata ran frozen without tracked dependency/config/lock mutation;
8. focused five tests, all-target compile, Clippy, rustfmt, and diff checks pass;
9. the push is a normal feature fast-forward after ref recheck;
10. main is unchanged and PR #11 remains open/draft/unmerged;
11. no wall timing, ranking, merge, or Task 2 occurred;
12. every claimed log exists and matches the stated command/result.

Output findings only, classified P0/P1/P2/P3 with exact file/line or command
reproducer. Do not fix. Pass requires P0=0 and P1=0.
