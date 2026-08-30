# Verification record

This file separates implemented contracts, expected host replay, and immutable
remote readback. A final branch commit cannot contain its own SHA, so those
values are recorded in the post-commit GitHub PR receipt.

## Source identity

```text
exact stack base head  17fcd447c1dadcea978f241ff3ba94635f9c2bd4
exact stack base tree  1152e0c74235afd7ae30c3b6de6315634fa49a59
implementation head    a2115e2dcaeb418185a3bccda62e60b6c4ff16ab
implementation tree    e560b501edff43043bfdd376a3490b924e504c65
final branch head/tree RECORDED_IN_POST_COMMIT_PR_RECEIPT
draft stacked PR       RECORDED_IN_POST_COMMIT_PR_RECEIPT
remote checks          RECORDED_IN_POST_COMMIT_PR_RECEIPT
```

Recovered archive SHA-256:

```text
c112309cab3e431ca563dd11dc1f67d95df0bfa85c8081251c33bea16ca44cfb
```

The archive is an incomplete transcript recovery snapshot and is not a test
receipt.

## TDD history

- The first cache contract was written before its public cache/identity types;
  the unresolved imports recorded the RED state.
- The minimal exact-identity cache then satisfied the same-binding reuse case.
- The whole-attempt contract was written before the transactional public API;
  unresolved imports recorded the second RED state.
- Further contracts cover setup failure, partial setup-work accounting,
  validation before work, one prepared RHS snapshot, atomic candidate commit,
  late apply failure, nonfinite admission arithmetic, protected fallback, and
  full rejection.
- The one allowed fresh review found one P1: all-ones Jacobi bits could pass via
  the provider's default false `is_identity()` self-report, and a provider could
  mutate after commit. Two frozen regression cases produced 11 PASS/2 FAIL and
  exit 101 before the single bounded repair. The cache now classifies identity
  from returned bits, owns the immutable exact diagonal, revalidates provider
  identity before reuse, and withholds caller output unless provider application
  matches the frozen map bit-for-bit. The repaired suite produced 13/13 PASS.

The final dedicated suite is 13/13 GREEN. The semantic-key mutation removed
the frozen-W digest comparison from the reuse predicate. The focused test then
failed with exit 101 because the stale handle remained pointer-identical; the
one-line comparison was restored and the same test passed with exit 0.

## Host interruptions retained

1. The first readiness attempt stopped after 12 Python scope tests because the
   host lacked `mpmath`. No Rust or scientific failure was inferred. The fixed
   `mpmath==1.3.0` dependency was installed in a scratch-only target.
2. The next readiness attempt reached clippy, which rejected 11 test-only
   `&vec![...]` allocations under `-D warnings`. They were replaced by slice
   literals without changing values or criteria.
3. The pre-review complete readiness command was restarted from the beginning
   and passed 75 Rust tests and 20 Python tests, default-example readback,
   clippy, and fmt.
4. After the single bounded review repair added two contracts, the complete
   readiness command was restarted again and passed 77 Rust tests and 20 Python
   tests, default-example readback, clippy, and fmt.

No tolerance, budget, identity field, problem size, or expected disposition was
changed after observing a numerical result.

## Required fresh host commands

```text
cargo test --locked -p rodas5p-integrators --features audit2-research \
  --test audit2_reusable_preconditioner_transaction_contracts \
  -- --nocapture --test-threads=1
expected 13 tests; actual 13/13 PASS

cargo test --locked -p rodas5p-integrators --features audit2-research \
  --test audit2_matrix_free_common_w_contracts \
  -- --nocapture --test-threads=1
expected 6 tests; actual 6/6 PASS

cargo test --locked -p rodas5p-integrators --features audit2-research \
  --test audit2_structured_correction_contracts \
  -- --nocapture --test-threads=1
expected exact-base 15 tests; actual 15/15 PASS

AUDIT2_OUTPUT_DIR=<fresh-empty-directory> bash tools/check-audit2-readiness.sh
actual 77 Rust tests + 20 Python tests PASS; exit 0

cargo clippy --locked -p rodas5p-integrators -p rodas5p-fair-ab \
  --all-targets --features rodas5p-integrators/audit2-research -- -D warnings
exit 0

cargo fmt --all -- --check
exit 0

git diff --check
exit 0
```

## Required structural readback

Record evidence that:

- the public surface remains feature-gated and no production/default dispatcher
  calls the new entry;
- cache reuse requires both the durable semantic frozen-W digest and runtime
  exact operator identity, plus the declared exact diagonal map;
- the returned preconditioner is exact, finite, diagonal, and nonidentity;
- a candidate commits its state and pending cache lease only after all gates;
- candidate rejection and late apply failure roll back before the isolated
  identity-preconditioned fallback;
- complete rejection exposes no selected step and leaves the base state intact;
- all setup/candidate/failure/fallback/rejection work remains monotone;
- target preparation samples the batched RHS once, closing P2 in this node;
- PR #35's earlier P2 deferral remains untouched historical evidence.

Structural review status: `ONE_FRESH_REVIEW; ONE_P1_REPAIRED_ONCE; P0_NONE;
OTHER_P1_NONE_ESTABLISHED; NO_SECOND_FRESH_REVIEW`.

## Publication/readback receipt

```text
push mode             NON_FORCE; CONFIRMED_IN_POST_COMMIT_PR_RECEIPT
draft PR URL          RECORDED_IN_POST_COMMIT_PR_RECEIPT
PR base               research/audit2-matrix-free-common-w-20260830
PR head               research/audit2-reusable-preconditioner-transactional-step-20260830
merge state           MUST_REMAIN_OPEN_DRAFT_UNMERGED
check names/states     RECORDED_IN_POST_COMMIT_PR_RECEIPT
mock probe PR URL      https://github.com/cosmosapjw-quantum/vigilode/pull/37
mock probe checks      audit2-feature-and-usage=SUCCESS; fresh-clone-build=SUCCESS
mock probe merge SHA   5467b8d6954d7acb87385e591470670588c2f970
mock scientific value NONE_EXPECTED
```

The disposable mock connectivity merge, if performed, is not scientific
evidence and must not target `main`, PR #31, PR #35, or this scientific branch.

## Unexecuted scientific work

The repository contracts do not establish a real physical client result. The
remaining run requires a client frozen independently of the outcome, an
independent reference with stated uncertainty authority, and precommitted
output/embedded/original-target budgets. Its local-only instructions are in
`../CODEX_START_HERE.md`.

No test duration supports a timing claim. No current result supports
scalability, Krylov-basis reuse, a general/production preconditioner,
production dispatch, dense-output correctness, general event handling, or
end-to-end integration transactionality.

Claim ceiling:

> `EXPLORATORY_NONAUTHORITATIVE_REUSABLE_PRECONDITIONER_TRANSACTIONAL_STEP_SUBSTRATE`
