# Workflow Provenance Contract — Cycle-Free Scientific Execution and Receipt Closure

GitHub `pull_request` workflows normally execute a synthetic PR merge ref. The
provenance model therefore separates four identities that occur at different
times and must never be collapsed into one another.

## Identity classes

### 1. Scientific execution identity

```text
scientific_execution_head_sha
scientific_execution_head_tree
base_sha
base_tree
tested_execution_merge_sha
tested_execution_merge_tree
execution_workflow_run_id
execution_workflow_run_attempt
```

`scientific_execution_head_sha` is the immutable feature-branch commit whose
load-bearing code generated the twelve atomic cells. It exists before the
scientific workflow is dispatched. Under a normal `pull_request` checkout,
`tested_execution_merge_sha` is `GITHUB_SHA`, while
`scientific_execution_head_sha` is
`github.event.pull_request.head.sha`.

### 2. Receipt commit identity

```text
receipt_commit_sha
receipt_commit_tree
```

The receipt commit is a later descendant that adds the validated JSON/Markdown
receipt and the predeclared decision. Its identity is late-bound: it does not
exist until after the receipt files have been committed.

The committed receipt MUST NOT contain `receipt_commit_sha`,
`receipt_commit_tree`, or any post-receipt verification run ID. Requiring a
tracked file to contain its own commit or tree creates a self-referential Git
fixed-point problem and is forbidden.

### 3. External verification identity

```text
external_verification_run_id
external_verification_run_attempt
verified_receipt_commit_sha
verified_receipt_commit_tree
```

A1, E4, receipt-validation, and fresh-review runs performed after the receipt
commit are late-bound external verification evidence. Record them in the PR
conversation, GitHub checks, Jira/Confluence mirrors, and the completion report.
Do not amend or recommit the scientific receipt merely to insert these values.

### 4. Optional activation identity

A later approval-gated activation commit may change the ordinary committed arm
only after an `ADMISSIBLE_AND_DISCRIMINATING` receipt and fresh review. It is a
separate DAG node and is not part of the receipt-generation node.

## Required scientific receipt tuple

Every atomic cell, artifact manifest, aggregate, and committed receipt records
the scientific execution tuple only:

```text
repository
pull_request
scientific_execution_head_sha
scientific_execution_head_tree
base_sha
base_tree
tested_execution_merge_sha
tested_execution_merge_tree
rustc_version
cargo_version
execution_workflow_run_id
execution_workflow_run_attempt
artifact_content_manifest
```

The receipt may additionally record its expected parent relation:

```text
receipt_parent_expected = scientific_execution_head_sha
```

but it must not predict or embed its own later Git identity.

## Cycle-free state machine

```text
H_start
  -> H_exec  (implementation, tests, receipt workflow; no final receipt files)
  -> R_exec  (12-cell scientific execution on H_exec / tested merge tree)
  -> H_receipt (validated receipt + decision, descendant of H_exec)
  -> R_verify  (A1/E4/receipt closure on H_receipt; external evidence)
  -> fresh review
  -> optional approval-gated activation commit
  -> merge gate
```

Rules:

1. Freeze and publish `H_exec` before dispatching the twelve-cell workflow.
2. Reject artifacts if the base or `H_exec` moves during execution.
3. Download and independently validate all twelve cells before creating
   `H_receipt`.
4. `H_receipt` records `H_exec`, the tested execution merge identity, the
   execution workflow run, and the artifact content manifest.
5. Do not rerun the scientific campaign merely because receipt files were added.
6. If any load-bearing scientific code, governed constant, runner semantics, or
   aggregation semantics changes after `H_exec`, discard the old artifacts and
   create a new execution head and workflow run.
7. Post-receipt A1/E4/receipt-validation runs verify `H_receipt`; their IDs remain
   external and are not scientific-execution provenance.
8. The ordinary committed arm remains `legacy-fixed` throughout this node.
   Candidate activation is a separate approval-gated node.

## Final merge relation

A successful PR workflow does not predetermine the eventual merge commit SHA.
At the later merge gate, verify:

```text
final merged main tree == reviewed final PR merge tree
```

with unchanged base and reviewed feature head. This merge-gate evidence is also
late-bound and external to the committed scientific receipt.

## Failure classes

- Conflating feature head, synthetic merge, receipt commit, activation commit,
  or final merge is P1 and becomes P0 when evidence from one code tree
  authorizes another.
- Requiring a committed receipt to embed its own commit/tree or a workflow run
  that can only occur after that commit is a control-plane specification defect,
  not a scientific-integrity failure.
