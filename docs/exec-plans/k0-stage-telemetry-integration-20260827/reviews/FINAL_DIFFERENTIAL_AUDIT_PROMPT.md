# Final Differential Audit Contract

Audit only the frozen implementation base through final SHA.

Inputs:

- `BASE_SHA..FINAL_SHA`
- `plan.json` and work-unit JSON files
- final evidence bundle
- fresh-review findings and repair delta
- Jira/Confluence readback
- unresolved blockers

Ask only:

1. Did the implementation violate a compiled invariant or scope boundary?
2. Did the contract miss a P0/P1 failure class visible in the delta?
3. Is any evidence fabricated, stale, incomplete, rebound, or misclassified?
4. Did scope expand into shared Krylov, structured predictors, timing, homotopy certification, production activation, or historical rewrites?
5. Did the fresh reviewer issue a false PASS or false blocker?
6. Are GitHub, Jira, and Confluence pointers consistent?

Do not restart a general VigilODE audit. Every novel P0/P1 becomes one durable primary guard, preferably by strengthening an existing test/invariant rather than adding a new process layer.

Allowed states: PASS, PASS_WITH_NONBLOCKING_FINDINGS, BLOCKED_BY_UNRESOLVED_SPEC, BLOCKED_BY_AUTHORITY_DRIFT, BLOCKED_BY_ATLASSIAN_AUTHORITY_DRIFT, BLOCKED_BY_P0, BLOCKED_BY_P1, ATLAS_SYNC_PENDING, PARTIAL, FAILED.
