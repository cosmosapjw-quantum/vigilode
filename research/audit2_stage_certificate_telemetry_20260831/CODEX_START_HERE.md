# Local Codex job: generate candidate-free telemetry and formal evidence

Operate as `LOCAL_CODEX_JOB_ONLY`. Do not use a local LLM. This is a fresh
generation task, not an attempt to recreate unpublished workspace-pruned
commits or to match their SHA. Read every file in this directory before acting.

Fixed execution mode: `FRESH_LOCAL_GENERATION`. A missing mandatory formal
tool terminates as `FORMAL_BACKEND_UNAVAILABLE`.

You are authorized to implement, test, commit, and push follow-up commits only
to the published branch and existing Draft PR named by `handoff.json`. You are
not authorized to merge, tag, release, change another PR, post unrelated
comments, touch Jira/Confluence, run a historical candidate, or access a
holdout.

## 1. Bind the published source

Repository: `cosmosapjw-quantum/vigilode`

Draft PR: `https://github.com/cosmosapjw-quantum/vigilode/pull/41`

Publication control C1 must be an ancestor:

```text
commit 193dcb8c0fb7c1042183739ecef627ae5df38612
tree   f40f7f3a43d7ad24c28142ea61ba2e3698d13030
```

The Draft PR must remain open, Draft, unmerged, based on
`research/audit2-bateman-local-execution-orchestrator-20260831`, and its base
commit must be
`426d37ce3c0f4e5b7843b163eaf772b8e55bfa87` with tree
`84ab302b6e7ec1318022753e9f31a669bdca4704`. Resolve the current Draft PR head
from GitHub, fetch it, and create an isolated clean worktree at that exact head.
Run the checked-in handoff validator and test before making changes.
They must validate every entry in `HANDOFF_INPUT_LOCK.json` against the fetched
PR head.

Stop as `SOURCE_OR_PUBLICATION_IDENTITY_UNRESOLVED` if the repository, branch,
stack, state, ancestry, or cleanliness differs. Preserve all existing
worktrees and untracked files.

Use the two supplied harness archives as process guidance after verifying their
bytes exactly:

```text
physmath-research-harness-gpt56.zip
sha256 9adde688f8020e7feb2c1c0304b3204dbe70dd01e2d87e64a5c4eb357c019934

physmath-coding-harness-gpt56.zip
sha256 6e67e999a0c19f6ed9de7c339067cc11691d5cf5cb662a11756d8fc393c849b4
```

Apply their research contract, evidence-before-narrative, scientific contract,
validation matrix, failure log, and independent diff-review phases. Keep their
initialized state under the external run directory; do not copy either archive
or a full harness tree into this repository.

## 2. Create fresh external state

Create a new nonexisting run directory below:

```text
${XDG_STATE_HOME:-$HOME/.local/state}/vigilode/stage-certificate-telemetry/<UTC_RUN_ID>
```

Keep the full command ledger, versions, stdout/stderr, build directories,
formal compiler products, hashes, and failures there. Never overwrite or
delete an earlier run. Generate an external manifest and `SHA256SUMS` for the
complete raw directory. Only their final SHA-256 values may appear in Git.

Before implementation, record availability and versions of Rust/Cargo,
Python, Wolfram Language, SageMath, Singular, Lean with mathlib, and Rocq. Tool
installation or repair is allowed only before evidence execution and must be
recorded. Do not copy a toolchain, package cache, or dependency archive into
the repository.

## 3. Implement the opt-in synthetic contract

Use test-driven, feature-gated work. Add Cargo feature
`audit2-stage-certificate`, depending on `audit2-research` but not on
`audit2-bateman-authority`. Default/no-default production behavior must remain
unchanged.

Add:

```text
crates/rodas5p-integrators/src/audit2_stage_certificate_research.rs
crates/rodas5p-integrators/tests/audit2_stage_certificate_contracts.rs
```

The v1 contract is deliberately restricted to canonical unscaled Euclidean
L2, schema `dimensionless-synthetic-l2/v1`, with every scale bit equal to
`1.0_f64.to_bits()`. Reject another norm or scale.

For an approximate solve `x`, residual `r=b-Wx`, and supplied upper bound
`kappa >= ||W^-1||`, recompute

```text
q_upper = ||x|| + kappa ||r||
```

with explicit upward binary64 multiplication and addition. Reject nonfinite,
negative, incomplete, or downward-rounded caller bounds. For nonnegative
strictly lower `T`, recompute `(I-T)^-1 q` by finite forward propagation and
then recompute endpoint and estimator contamination, `Ehat+Theta`, and
`max(Ehat-Theta,0)`. Do not accept caller-asserted derived quantities.

Decisions must be named only `SyntheticConsistentAccept` and
`SyntheticConsistentReject`, and receipt authority only
`SyntheticSchemaConsistencyOnly`. Preserve the complete frozen plan and trace,
their canonical JSON SHA-256 values, coefficient/norm/W/preconditioner/RHS
identities, residual-history completeness, restart/max-Arnoldi/iteration
policy, work fields, and partial traces after injected failure.

At minimum, focused tests must cover:

1. a positive deterministic strict-lower propagation fixture;
2. missing, nonfinite, negative, or inconsistent fields fail closed;
3. a non-unit norm scale is rejected;
4. downward-rounded product and sum bounds are rejected;
5. coefficient-bit tampering is rejected;
6. operator/preconditioner binding tampering is rejected;
7. incomplete/final-only residual history cannot produce a synthetic decision;
8. safe accept and safe reject are recomputed;
9. a late failure retains only completed stage traces and work;
10. receipt readback retains plan/trace and all provenance;
11. the feature graph and source surface cannot import or launch historical
    candidate or holdout paths.

No test may run a real client, historic one-shot controller, solver example,
or holdout. Synthetic matrices/vectors must be newly declared in this node.

## 4. Generate and compile F01--F05 proof sources

Implement the exact obligations in `FORMAL_SCOPE.md`. Expected proof-source
locations are:

```text
research/audit2_stage_certificate_telemetry_20260831/formal/wolfram/check_stage_certificate.wl
research/audit2_stage_certificate_telemetry_20260831/formal/sage/check_stage_majorant.sage
research/audit2_stage_certificate_telemetry_20260831/formal/singular/check_jacobi_numerator.sing
research/audit2_stage_certificate_telemetry_20260831/formal/lean/StageCertificate.lean
research/audit2_stage_certificate_telemetry_20260831/formal/rocq/StageCertificate.v
```

Use exact integers/rationals/symbols in CAS checks. Compile Lean against the
actual local mathlib project and compile Rocq with the recorded toolchain.
Singular is a numerator-pattern cross-check only. xAct is outside scope because
there is no tensor-calculus theorem here.

Build one orchestration command that writes every raw log and compiler product
outside the repository. It must emit a compact `formal_receipt.json` containing
tool versions, exact argv, proof-source SHA-256, exit code, stdout/stderr
SHA-256 and byte count, and expected success tokens. PASS requires all five
mandatory backends. Never fabricate a token or treat unavailable as skipped
PASS.

## 5. Verify software independently

Run focused tests first, then the repository readiness, clippy with
`-D warnings`, format check, and `git diff --check`. Use isolated target and
temporary directories outside the worktree. Record exact commands, exits, and
observed test counts; do not normalize counts to remembered values.

Perform two read-only reviews after the final integrated diff:

- software-contract review: schema, rounding, failure preservation, feature
  isolation, provenance, and negative tests;
- formal-scope review: F01--F05 proof coverage, tool-role accuracy, receipt
  binding, and nonclaims.

Any P0/P1 finding requires a bounded fix and fresh integrated verification.

## 6. Publish only compact analysis

Follow `RAW_DATA_POLICY.md` and `PUBLICATION_SCHEMA.json`. Commit proof source,
implementation, tests, small validators, `evidence/analysis_summary.json`,
`evidence/formal_receipt.json`, `evidence/ANALYSIS.md`, and
`evidence/SHA256SUMS`. Do not commit raw logs, raw histories, build products,
archives, datasets, or copied dependencies.

Before committing, fail closed unless:

- candidate executions are exactly zero and holdout access is unchanged;
- every checked-in result is derived from the current clean source and external
  raw manifest;
- every file and total-directory size is within the frozen limits;
- the forbidden suffix and path audit is empty;
- result JSON validates against the checked-in publication schema;
- source, formal, and receipt hashes read back exactly.

Use separate coherent commits for implementation and evidence binding. Push
only to the handoff branch and update only the existing Draft PR. Do not merge.

## 7. Return complete readback

Return the final commit/tree and ancestry, Draft PR URL/state/base/head, remote
check results, all local commands and exits, actual test counts, formal backend
versions and dispositions, external state path and manifest hashes, checked-in
byte total, forbidden-file audit, failed-attempt history, and:

```text
local_llm_used: false
candidate_executions: 0
holdout_access: NOT_OPENED_OR_EXECUTED
raw_data_committed: false
claim_ceiling: EXPLORATORY_NONAUTHORITATIVE_REUSABLE_PRECONDITIONER_TRANSACTIONAL_STEP_SUBSTRATE
```

If any mandatory backend is unavailable, preserve the run and publish only an
honest unavailable analysis if the compact schema permits it. Do not weaken an
obligation or fit a result.
