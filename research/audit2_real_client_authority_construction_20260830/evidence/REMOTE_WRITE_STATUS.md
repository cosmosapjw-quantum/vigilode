# Remote write status

This docs-binding snapshot records the already-created immutable implementation
anchor:

```text
implementation head  cac7d1b7337a6dff25a60072009658f6ddf155d9
implementation tree  c23abbee0d47e2dbe002e01516bf34e2481bc333
parent               f954e39130e5141256731d0745666a872c0267ea
```

Every implementation blob SHA and the resulting tree matched the verified
local Git objects before the non-force branch creation. The later docs head,
Draft PR URL, check states, ancestry comparison, and merge state are necessarily
recorded after publication in the GitHub PR receipt.

The intended scientific branch is:

```text
research/audit2-real-client-authority-construction-20260830
```

It must be pushed non-force and opened as a Draft PR stacked directly on the
published PR #38 branch:

```text
base branch  research/audit2-reusable-preconditioner-transactional-step-20260830
base head    f954e39130e5141256731d0745666a872c0267ea
base tree    4314da2f9e1533737d4169526ebd2d84515ab19d
base state   OPEN / DRAFT / UNMERGED
```

The construction worktree's parent `5cf4189...` has the same tree but is not
the remote publication parent. Git data publication must bind the new remote
commit chain to `f954e391...` exactly.

The scientific PR must remain open, Draft, and unmerged. No merge, tag, release,
PM-7/K0 mutation, Jira/Confluence mutation, or holdout operation is authorized
by this node.
