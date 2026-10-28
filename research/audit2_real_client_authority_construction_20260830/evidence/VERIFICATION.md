# Verification record

This record separates candidate-free construction checks, final host replay,
local scientific execution, and immutable remote readback.

## Source identity

```text
stack base PR                 #38 OPEN / DRAFT / UNMERGED
stack base remote head        f954e39130e5141256731d0745666a872c0267ea
stack base remote tree        4314da2f9e1533737d4169526ebd2d84515ab19d
equivalent local parent       5cf4189be67458cfd998eceba39363462d59ca6d
equivalent local parent tree  4314da2f9e1533737d4169526ebd2d84515ab19d
implementation head           cac7d1b7337a6dff25a60072009658f6ddf155d9
implementation tree           c23abbee0d47e2dbe002e01516bf34e2481bc333
final branch head/tree        RECORDED_IN_POST_COMMIT_PR_RECEIPT
draft stacked PR              RECORDED_IN_POST_COMMIT_PR_RECEIPT
remote checks                 RECORDED_IN_POST_COMMIT_PR_RECEIPT
```

## Frozen construction observations

- Exact harness SHA-256 values are recorded in `../TOOL_PROVENANCE.md`.
- The canonical manifest has two operator cases, six frozen scenarios, and
  forbids candidate execution during construction.
- The stdlib-only verifier recomputes exact-rational reference bounds, both
  W digests, both PC identities, scenario order, and fixed budget admission
  predicates. Exact manifest bytes plus canonical Rust equality bind every
  frozen numerical budget field.
- Admission binds exact manifest SHA-256 `673045bf...c360`, verifier SHA-256
  `542715ca...487d`, and candidate-free proof-receipt SHA-256
  `057cceba...ab12` before constructing the opaque one-shot authority.
- Its candidate-free result was `AUTHORITY_CONSTRUCTION_VERIFIED`, maximum
  reference L2 bound `2.075243427511439e-17`, declared bound `1e-15`, and
  `candidate_executions: 0`.
- No Oregonator holdout file, fixture, or case was opened or run.

## TDD and P1 history

- Missing public Bateman symbols produced the first Rust RED state (exit 101).
- Missing manifest and verifier artifacts produced separate fail-closed RED
  states before their implementations.
- The candidate-free Rust authority suite first reached 4/4, then 5/5, and
  passed its source-freeze expanded 6/6 replay; the Python exact-authority
  suite first reached 3/3 and then passed its final expanded 4/4 replay.
- Dedicated CI/readiness scope first failed its new feature assertion, then
  passed after `audit2-bateman-authority` was added explicitly.
- An independent pre-execution review found a P1: the PR #38 output gate used
  `E_reference <= B + u`. A frozen nonzero-uncertainty killing test with
  `(0.9,1.0,0.2)` distinguished the valid rule. The repaired pre-commit gate
  records the independent budget and `E_reference + u` separately and accepts
  only a declared upper bound satisfying `E_reference + u <= B`.
- Estimate-only uncertainty cannot produce categorical admission.
- A fresh review found a P2 in the first repair: nearest-even arithmetic could
  round `1 + 2^-54` back to `1`, and ordinary difference/L2/budget operations
  were not one-sided bounds. Before candidate execution, a bounded repair
  outward-bounded candidate/reference differences and L2, outward-rounded the
  uncertainty sum, and inward-rounded the output budget. The regression case
  must reject under final replay.
- A separate one-shot runner review found two P2 receipt gaps and one P3
  partial-failure gap. The repair adds an explicit mismatch disposition,
  retains the candidate step, requires an accepted candidate step with a
  rejected independent budget for the strict-fallback contract, and preserves
  cache/work in terminal partial failures. The local receipt validator was
  added test-first and remains candidate-free.
- One fresh final review found one P1: the compact validator could false-pass
  empty `WorkCounters`, a fake state digest, minimally self-asserted budgets,
  and an incomplete late-failure apply ledger, while runtime admission lacked
  a candidate-free live W/PC rebinding contract. Five validator regressions
  produced the expected candidate-free RED failures, and the missing runtime
  binding surface produced Cargo exit `101`. The bounded repair added exact
  work/state/budget/cache/late-ledger validation plus live shifted-W and PC
  rebinding for both operator cases. No candidate executed.

No Bateman candidate was called by these tests. No model parameter, reference,
uncertainty, digest, preconditioner, budget, trial stage, or scenario identity
was fitted to a candidate output.

## Final hosted commands and results

```text
python3 research/audit2_real_client_authority_construction_20260830/verify_authority_manifest.py

python3 tools/test_audit2_real_client_authority.py -v

python3 tools/test_audit2_bateman_local_receipt.py -v

cargo test --locked -p rodas5p-integrators \
  --features audit2-bateman-authority \
  --test audit2_real_client_authority_contracts \
  -- --nocapture --test-threads=1

cargo test --locked -p rodas5p-integrators \
  --features audit2-research \
  --test audit2_reusable_preconditioner_transaction_contracts \
  --test audit2_matrix_free_common_w_contracts \
  --test audit2_structured_correction_contracts \
  -- --nocapture --test-threads=1

AUDIT2_OUTPUT_DIR=<fresh-empty-directory> bash tools/check-audit2-readiness.sh

cargo clippy --locked -p rodas5p-integrators -p rodas5p-fair-ab \
  --all-targets --features rodas5p-integrators/audit2-bateman-authority -- -D warnings

cargo fmt --all -- --check

git diff --check
```

Final status: `HOST_CANDIDATE_FREE_FINAL_PASS`.

Evidence directory:

```text
/workspace/scratch/43ab8f4e0aeb/verification/audit2-authority-source-freeze.Ic445U
```

The complete readiness replay exited `0` with:

```text
Python 36/36 PASS = 12 + 8 + 4 + 12
Rust   86/86 PASS = 8 + 9 + 6 + 16 + 15 + 15 + 6 + 6 + 5
clippy -D warnings PASS
format check PASS
Bateman local example COMPILED_ONLY_NOT_EXECUTED
Bateman candidate executions 0
```

The Rust terms correspond respectively to the two fair-ab suites,
matrix-free common-W, reusable transaction, original-target bridge, dense
output, homotopy, Bateman authority, and default-example unit tests. The 12
local-receipt Python tests use synthetic frozen reports and do not execute the
scientific example. The actual Rust uncertainty enum is serialized as uppercase
`DECLARED_UPPER_BOUND`; kebab-case is rejected.

### Final-attempt failure history

1. `audit2-authority-final.p9f1Kf` stopped after 12 passing Python scope tests
   because `mpmath` was missing from the active import path. This was a host
   dependency failure and produced no candidate evidence.
2. `audit2-authority-final-retry.7idxY5` reached clippy after the candidate-free
   Python and Rust tests; clippy rejected an unboxed
   `Audit2BatemanPartialFailure` under `result_large_err` with `-D warnings`.
   The error value was boxed without changing scientific inputs, criteria, or
   scenario semantics.
3. `audit2-authority-post-review-pass.oFT5VV` was interrupted after the fresh
   reviewer caught the wrong kebab-case uncertainty spelling in the synthetic
   path. The actual Rust JSON enum is uppercase `DECLARED_UPPER_BOUND`; the
   interrupted run is not final evidence.
4. `audit2-authority-post-review-final.2rlYQV` reached clippy and failed
   `needless_range_loop` in the live W/PC rebinding loop. Replacing the indexed
   loop with iterator `enumerate()` was mechanical and did not change a
   scientific input, criterion, or expected disposition.
5. The complete replay was restarted in
   `audit2-authority-source-freeze.Ic445U` and passed all 36 Python tests, all
   86 Rust tests, clippy, and formatting with exit `0`.

## Final candidate-free structural readback

The hosted checks establish that:

- the Bateman authority is behind a distinct non-default feature and no
  production/default dispatcher calls it;
- canonical construction accepts only the exact checked-in manifest;
- both frozen-W digests and exact nonidentity Jacobi maps are recomputed and
  rebound to exact live runtime contexts in a candidate-free 6/6 authority
  suite;
- authority construction and ordinary hosted checks do not execute a Bateman
  candidate;
- transaction admission uses conservative f64 bounds to consume a declared
  reference upper bound inside the independent output budget, the
  `1 + 2^-54` rounding counterexample rejects, and estimate-only uncertainty
  rejects;
- protected PR #31, fixtures, prior scientific-validity evidence,
  `rodas5p-core`, `Cargo.lock`, and holdouts have no diff;
- the local runner accepts no replacement thresholds/reference/digest/PC/case
  order, emits schema `vigilode-audit2-bateman-local-six-case-report/v1`, and
  retains a partial report on failure;
- the example compiled successfully but was not run, so no candidate receipt,
  Bateman state hash, or Bateman `WorkCounters` was generated.

Fresh final review disposition:

```text
ONE_FRESH_FINAL_REVIEW
ONE_P1_FOUND
CANDIDATE_FREE_RED = FIVE_VALIDATOR_FAILURES + RUNTIME_BINDING_COMPILE_EXIT_101
ONE_BOUNDED_REPAIR
SOURCE_FREEZE_REPLAY_PASS
CANDIDATE_EXECUTIONS_0
```

## Residual compact-receipt limitations

- The compact report does not carry raw embedded-error or original-target
  vectors. Consequently, its validator cannot independently recompute the
  reported embedded L2, original-target residual L2, or contraction scalar; it
  validates finite values, exact declared limits, subgate booleans, and
  cross-field consistency instead.
- SHA-256 checks provide byte-integrity binding. They do not attest the host,
  executor, authorship, execution time, or truth of an external provenance
  claim.

## Local six-case status

```text
status              NOT_RUN_DURING_CONSTRUCTION
candidate receipts  NONE
Bateman state hashes NONE
Bateman work counters NONE
handoff             ../CODEX_START_HERE.md
```

The local run must bind the exact published implementation head/tree and
preserve raw results and hashes outside the repository. A successful local run
still requires independent receipt review before any bounded Bateman-specific
statement is considered.

## Publication/readback status

```text
push mode          NON_FORCE_REQUIRED
PR base            research/audit2-reusable-preconditioner-transactional-step-20260830
PR head            research/audit2-real-client-authority-construction-20260830
merge state        MUST_REMAIN_OPEN_DRAFT_UNMERGED
head/tree/checks   RECORDED_IN_POST_COMMIT_PR_RECEIPT
```

Claim ceiling:

> `EXPLORATORY_NONAUTHORITATIVE_REUSABLE_PRECONDITIONER_TRANSACTIONAL_STEP_SUBSTRATE`
