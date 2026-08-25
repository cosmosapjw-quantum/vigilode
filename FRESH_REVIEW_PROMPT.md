# FRESH REVIEW PROMPT — VigilODE A1

Act as a fresh-context, read-only reviewer of PR #18. Do not trust the implementer's summary. Do not modify any branch, PR, issue, tag, release, workflow, or local tracked file.

## Bound state

```text
repository: cosmosapjw-quantum/vigilode
base: main@4e3a75e5b2843dc1e135dcadba72edb1d09be94c
candidate branch: research/a1-inner-tolerance-parity
intake head: 67ec3ad77d0a88f3ff9c096b309d3a12da72b600
handoff: handoff/a1-inner-tolerance-parity-20260825
PR: #18, draft, unmerged
```

First re-read live state. If the candidate head changed, review the exact new head only after confirming it is an ordinary descendant of the intake head and that exact-head verification exists. Otherwise return `BLOCKED_BY_REMOTE_DRIFT`.

## Review questions

1. Does the canonical base show one production phi authority law, or are there competing laws?
2. Does the candidate preserve the exact old arithmetic
   `max(3.0e-2*outer_rtol,1.0e-12)` and
   `max(3.0e-4*outer_rtol,1.0e-14)`?
3. Do linear GMRES and phi-Krylov consume the same stored relative and absolute values?
4. Are NaN, both infinities, zero, and negative outer tolerance rejected before solver work?
5. Are all nominal, retry, fallback, matrix-free, and alternate production call paths accounted for?
6. Is the source guard backed by executable behavior rather than standing alone?
7. Are GMRES method/restart/maxiter and phi structural settings unchanged?
8. Does the diff contain only the intended five A1 paths, or is every additional path explicitly justified by a minimal repair?
9. Is there any A2/A3, timing, controller, dependency, fixture, equation, convergence, preconditioner, or work-accounting contamination?
10. Are A1, G4/S5B0, v3.8-D schema, build, clippy, fmt, and E4 online/offline evidence green on the exact final head?
11. Is PR #18 still draft and unmerged, with an A1-only claim boundary?
12. Is any wall-time ranking, speedup, active switching, merge, tag, or release claim being smuggled in?

## Mandatory independent checks

Run the handoff acceptance and callsite discovery scripts. Re-run at least the focused A1 and G4/S5B0 contracts and inspect exact-head GitHub Actions. Independently compare the base and final candidate diff.

## Severity policy

- P0: authority ambiguity; incomplete production wiring; A2/A3 contamination; scientific/dependency/fixture drift; false exact-head evidence; unauthorized merge or performance claim.
- P1: invalid-domain leak; floor mismatch; one-ULP old-law regression; structural setting drift; stale PR evidence that obscures the final state.
- P2: narrow documentation or maintainability defect that does not affect A1 behavior.
- P3: style only.

## Output

Return exactly one top-level verdict:

```text
APPROVE_A1_ONLY
```

only when P0=0 and P1=0. State explicitly that this approves only the A1 tolerance-policy correction for further human merge consideration; it does not approve A2/A3, timing rankings, active switching, or any scientific tournament conclusion.

Otherwise return:

```text
BLOCK_A1
```

and list findings in descending severity with exact file/line, evidence, failure mode, and minimal condition for closure.

Never infer success from test count alone and never modify the repository.