# Workflow Provenance Contract — PR Head vs Tested Merge Tree

GitHub `pull_request` workflows use the PR merge ref by default. Therefore:

```text
GITHUB_REF
refs/pull/<pr-number>/merge

GITHUB_SHA
synthetic PR merge commit

github.event.pull_request.head.sha
actual feature-branch head

github.event.pull_request.base.sha
actual base commit in the event payload
```

The A1 receipt must never label `GITHUB_SHA` as the feature head.

## Required identity tuple

Every atomic cell, artifact manifest, aggregate, and committed receipt must record:

```text
repository
pull_request
candidate_head_sha
candidate_head_tree
base_sha
base_tree
tested_merge_sha
tested_merge_tree
rustc_version
cargo_version
workflow_run_id
workflow_run_attempt
```

## Execution rule

The scientific 12-cell receipt should be generated from the code tree actually tested by the workflow. Under a normal `pull_request` checkout this is the synthetic merge tree. The receipt must also bind the feature head so the source branch can be reconstructed.

Before accepting the artifact:

1. confirm `candidate_head_sha == github.event.pull_request.head.sha`;
2. fetch the candidate head and compute `candidate_head_tree`;
3. confirm the checkout SHA equals `GITHUB_SHA`;
4. compute and retain the checkout `tested_merge_tree`;
5. confirm the merge commit has parents `(base_sha, candidate_head_sha)` in that order or record the exact parent relation returned by Git;
6. reject the artifact if the base or candidate head moved after the workflow began;
7. rerun the receipt after any feature-head or base-head change.

## Final merge relation

A successful PR workflow does not by itself prove the eventual merge commit SHA, because GitHub can create a new merge commit at merge time. The load-bearing invariant is tree/content continuity:

```text
final merged main tree == reviewed tested_merge_tree
```

subject to an unchanged base and exact reviewed feature head. Verify this only at the later merge gate.

## Security and trigger boundary

Use `pull_request`, not `pull_request_target`, for executing candidate code. If both branch and path filters are used, the workflow must be designed with the fact that both filters must match.

## Failure class

Any conflation of candidate head, synthetic merge SHA, or final merge SHA is a provenance P1 and becomes P0 if it causes evidence from one code tree to authorize another.
