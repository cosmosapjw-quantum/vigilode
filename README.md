# VigilODE A1 Inner-Tolerance Parity Handoff

This branch repairs a missing durable handoff ref. It is intentionally separate from both `main` and the A1 implementation branch.

## Live state captured at publication

```text
main
4e3a75e5b2843dc1e135dcadba72edb1d09be94c

tree
c6d4e20b54f84e6894b1954fc61681b881350b85

implementation branch
research/a1-inner-tolerance-parity

implementation head
67ec3ad77d0a88f3ff9c096b309d3a12da72b600

implementation tree
4d5070a35cbc546efc1dd350feeb4a45e08c7e01

existing PR
#18, open, draft, unmerged
```

The feature branch already contains the A1 candidate and has exact-head GitHub Actions evidence. Therefore this handoff directs Codex to **audit first**, not to recreate the implementation. A source change is permitted only for a demonstrated A1-only defect.

## Quick start

From a local clone:

```bash
git fetch origin \
  main \
  research/a1-inner-tolerance-parity \
  handoff/a1-inner-tolerance-parity-20260825

git worktree add --detach ../vigilode-a1-handoff \
  origin/handoff/a1-inner-tolerance-parity-20260825

git worktree add -B research/a1-inner-tolerance-parity \
  ../vigilode-a1-implementation \
  origin/research/a1-inner-tolerance-parity

python ../vigilode-a1-handoff/acceptance/test_handoff_contract.py \
  --repo ../vigilode-a1-implementation \
  --handoff ../vigilode-a1-handoff
```

Read `AGENTS.md` before doing anything else.

## Intended result

The normal no-repair outcome is:

```text
A1_REVIEW_READY
base = 4e3a75e5b2843dc1e135dcadba72edb1d09be94c
head = 67ec3ad77d0a88f3ff9c096b309d3a12da72b600
PR = #18 draft/unmerged
A1 contract = PASS
G4/S5B0 regressions = PASS
fresh-clone online/offline = PASS
A2/A3 = NOT PERFORMED
wall-time ranking = NOT PERFORMED
merge/tag/release = NOT PERFORMED
```

This handoff must not be merged.