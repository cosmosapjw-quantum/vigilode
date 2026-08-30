# HOST_CODEX_ONLY local execution prompt — real-client validation

Operate as `HOST_CODEX_ONLY`. Do not use a local LLM for code generation,
review, classification, claim admission, or evidence interpretation. This
prompt authorizes local, read-mostly execution for the remaining real-client
validation only. It contains no authorization to push, create or edit a PR,
merge, comment on GitHub/Jira/Confluence, change a tag/release, or otherwise
mutate remote state.

## 1. Bind the source before execution

Repository: `cosmosapjw-quantum/vigilode`

Required scientific branch after publication:

```text
branch  research/audit2-reusable-preconditioner-transactional-step-20260830
head    PENDING_PARENT_FILL
tree    PENDING_PARENT_FILL
base    17fcd447c1dadcea978f241ff3ba94635f9c2bd4
tree    1152e0c74235afd7ae30c3b6de6315634fa49a59
```

Stop with `SOURCE_IDENTITY_UNRESOLVED` if the final placeholders have not been
filled or the local checkout differs. Use an isolated clean worktree. Preserve
all existing checkouts, untracked files, logs, and worktrees.

The implementation-bearing paths are:

```text
README.md
crates/rodas5p-integrators/src/audit2_matrix_free_research.rs
crates/rodas5p-integrators/src/audit2_reusable_transaction_research.rs
crates/rodas5p-integrators/src/lib.rs
crates/rodas5p-integrators/tests/audit2_reusable_preconditioner_transaction_contracts.rs
tools/check-audit2-readiness.sh
tools/test_a1_receipt_ci_scope.py
research/audit2_reusable_preconditioner_transactional_step_20260830/**
```

Do not alter production/default routing, protected fixtures, historical
receipts, `Cargo.lock`, holdouts, or the published PR #31/#35 evidence. The P2
single-prepared-target fix belongs to this node; the old PR #35 receipt remains
historically correct at its own SHA.

## 2. Re-establish the host contract baseline

Use the repository-pinned Rust toolchain and retain complete logs plus SHA-256
digests. Run:

```bash
cargo test --locked -p rodas5p-integrators --features audit2-research \
  --test audit2_reusable_preconditioner_transaction_contracts \
  -- --nocapture --test-threads=1

cargo test --locked -p rodas5p-integrators --features audit2-research \
  --test audit2_matrix_free_common_w_contracts \
  -- --nocapture --test-threads=1

cargo test --locked -p rodas5p-integrators --features audit2-research \
  --test audit2_structured_correction_contracts \
  -- --nocapture --test-threads=1

bash tools/check-audit2-readiness.sh

cargo clippy --locked -p rodas5p-integrators -p rodas5p-fair-ab \
  --all-targets --features rodas5p-integrators/audit2-research -- -D warnings

cargo fmt --all -- --check
git diff --check
```

Expected focused counts are 13/13 for the reusable transaction contracts, 6/6
for the PR #35 matrix-free common-W contracts, and 15/15 for the exact-base
original-target bridge contracts. Record observed counts literally. Do not add
or delete tests to make a remembered total match. Any failure blocks the
real-client claim; do not widen or fit a tolerance.

## 3. Freeze the real-client protocol before observing results

The recovered thread archive is checksum-bound context but has an incomplete
transcript:

```text
c112309cab3e431ca563dd11dc1f67d95df0bfa85c8081251c33bea16ca44cfb
```

It does not select a client or numerical threshold. Before running the
candidate, create a local execution manifest outside the repository that fixes:

- the real physical client source and SHA-256, parameter set, initial state,
  interval, and output observables;
- an independent reference method/version/configuration, its source SHA-256,
  convergence study, and an asserted uncertainty bound or an explicit statement
  that only an estimate is available;
- output L2 absolute/relative budgets, embedded-error ceiling,
  original-target residual ceiling, and contraction ceiling;
- the exact semantic frozen-W digest schema and value;
- the exact diagonal nonidentity preconditioner construction, provider,
  revision, configuration bits, and expected inverse-diagonal bits;
- same-W reuse cases and changed-W invalidation cases;
- deterministic seeds, thread count, environment, compiler identity, and log
  destinations.

If no executable real client, independent reference, or defensible
precommitted budget is available, stop as `REAL_CLIENT_AUTHORITY_UNAVAILABLE`.
Manufactured-vector probes and historical rows are not substitutes. Do not open
a holdout to fill this gap.

## 4. Execute bounded local cases

Use only the feature-gated research API. Run at minimum:

1. one same-frozen-W sequence that demonstrates setup reuse only when both the
   semantic digest and runtime exact operator identity match;
2. one changed-W sequence that forces invalidation and rebuild;
3. one normal candidate attempt with all output, embedded, and original-target
   diagnostics reported under the frozen independent budget;
4. one intentionally over-strict budget that rejects the candidate and proves
   the protected sequential-JF identity fallback starts from the exact base
   state;
5. one late reusable-preconditioner apply failure that preserves partial
   candidate work, rolls back the pending lease, and runs the isolated fallback;
6. one terminal rejection that exposes no selected step, leaves numerical state
   unchanged, and retains monotonically increasing work counters.

For every case, retain the input manifest, terminal disposition, cache snapshot,
candidate/fallback receipts, numerical state hashes, all `WorkCounters`, and
full stderr/stdout. Verify that the candidate reports an exact nonidentity
diagonal map and that fallback reports the protected identity-preconditioned
path. Never describe allocation or setup reuse as Krylov-basis reuse.

## 5. Admission and stop rule

Classify output accuracy only from the precommitted budget and independent
reference uncertainty:

- an asserted upper uncertainty bound may support a bounded within/outside
  classification;
- an estimate-only uncertainty must remain `REFERENCE_UNRESOLVED`;
- missing or nonfinite derived bounds are failures, never automatic passes.

Even if every local case passes, the maximum local conclusion is a bounded
real-client observation for the frozen case. Do not claim speed, scalability,
Krylov reuse, a general/production preconditioner, production dispatch,
dense-output correctness, general event handling, or end-to-end integration
transactionality.

Write local results and log hashes to a new execution directory outside the
repository. Do not edit this handoff, commit results, or perform any remote
operation under this prompt. Return the local directory path, source/tree
identity, exact commands and exits, manifest hash, raw observations, failures,
and the unchanged claim ceiling:

```text
EXPLORATORY_NONAUTHORITATIVE_REUSABLE_PRECONDITIONER_TRANSACTIONAL_STEP_SUBSTRATE
```
