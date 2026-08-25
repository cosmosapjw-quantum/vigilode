# Workflow Provenance Contract — Cycle-Free Execution with Scientific Invalidation

## Identity classes

The cycle-free model remains:

```text
H_start
  -> H_exec
  -> R_exec and atomic artifacts
  -> H_receipt
  -> external R_verify
  -> fresh review
  -> optional approval-gated activation
  -> merge gate
```

A tracked receipt must not embed its own later commit/tree or post-receipt workflow IDs.

## New admissibility rule

A successful workflow is not automatically a valid scientific execution. `H_exec` and `R_exec` become receipt-eligible only when the load-bearing atomic schema, runner semantics, audit evidence, and aggregate validator satisfy the scientific contract.

Run `32906175896` is explicitly invalidated for authority because its atomic cells omitted independent audit full-E evidence and used runtime shadow execution to derive `audit_unsafe`.

## Invalidated execution rule

```text
H_bad = 755b31750c1f0e026bbe11aca24efb71e6242624
R_bad = 32906175896
status = STOP_INVALID_NON_AUTHORITY
```

Preserve its artifacts for diagnosis. Do not create `H_receipt` from them. Do not reuse their aggregate decision.

Any change to the following is load-bearing and requires a new `H_exec` and a fresh twelve-cell execution:

- atomic event schema;
- independent audit full-E runner;
- event eligibility semantics;
- audit completion/admissibility/failure semantics;
- positive-control logic;
- aggregate decision logic.

## Required scientific execution tuple

Every new atomic cell, artifact manifest, aggregate, and committed receipt records:

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

The tracked receipt must not contain:

```text
receipt_commit_sha
receipt_commit_tree
external_verification_run_id
external_verification_run_attempt
```

## Scientific audit completeness

The aggregate may emit a predeclared scientific decision only if:

1. all twelve cells share one scientific execution identity;
2. every event row contains an explicit audit eligibility/status record;
3. every audit-eligible event has completed audit full-E evidence, or the run terminates `STOP_INVALID`;
4. Hires positive-control evidence is complete for both arms;
5. runtime shadow evidence is not used as a substitute for audit evidence;
6. audit execution does not alter recommendation, budgets, controller state, or the committed arm.

## Final merge relation

At the later merge gate, verify:

```text
final merged main tree == reviewed final PR merge tree
```

with unchanged base and reviewed feature head. This evidence remains external to the tracked scientific receipt.
