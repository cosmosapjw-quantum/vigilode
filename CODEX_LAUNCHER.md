# CODEX LAUNCHER

Paste the following into a fresh Codex session opened in the local `vigilode` clone:

```text
Resume VigilODE A1 inner-tolerance parity from the durable handoff.

Repository:
~/vigilode

Required remote refs:
- main
- research/a1-inner-tolerance-parity
- handoff/a1-inner-tolerance-parity-20260825

The handoff branch is read-only and must never be merged. Create a detached
worktree for it, read its AGENTS.md, follow the complete mandatory read order,
and execute IMPLEMENTER_PROMPT.md.

Canonical intake state:
- main: 4e3a75e5b2843dc1e135dcadba72edb1d09be94c
- main tree: c6d4e20b54f84e6894b1954fc61681b881350b85
- feature head: 67ec3ad77d0a88f3ff9c096b309d3a12da72b600
- feature tree: 4d5070a35cbc546efc1dd350feeb4a45e08c7e01
- existing PR: #18, open/draft/unmerged

Important correction: the feature branch already contains an A1 candidate.
Audit it first. Do not recreate it, do not open another PR, and do not add
retrospective process documents to the implementation diff. Make a minimal
repair commit only when a concrete A1 contract violation is demonstrated.

Before any mutation, run the handoff acceptance contract and callsite discovery
script. If remote identity moved, stop with BLOCKED_BY_REMOTE_DRIFT. If more
than one production authority law exists, stop with BLOCKED_BY_UNRESOLVED_SPEC.

Do not implement A2 or A3. Do not alter timing, controller, dependencies,
fixtures, equations, convergence logic, preconditioners, or work accounting.
Do not merge, tag, release, rank wall time, claim speedup, or activate switching.

Reuse PR #18 and leave it draft and unmerged. Return A1_REVIEW_READY or
A1_REPAIRED_REVIEW_READY only after every invariant in the handoff passes on
the exact final head.
```

After the implementation/recovery pass reaches a stable exact head, open a separate fresh Codex context and give it `FRESH_REVIEW_PROMPT.md`. The fresh reviewer must be read-only.