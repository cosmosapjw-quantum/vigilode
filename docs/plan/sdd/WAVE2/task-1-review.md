# Wave-2 Task 1 review: c1-transaction (148b515..f07c4d7)

### Spec Compliance

- ✅ Spec compliant (with the controller's ruling that the overhead guard is the faithful form of Proposition 8).
  Every brief item has a hunk: `Transaction`/`Trial` begin/charge/commit/rollback/drop (transaction.rs:180-349),
  `Budget` + typed `BudgetExhausted` (353-435), `AdmissionRule` with per-bin Beta, charged-only updates, UCB > τ,
  m = ⌈10τ⌉ (443-835), `RegimeTelemetry`/`RitzSummary` serde data (840-863), `lib.rs` export (lib.rs:30, 52-56),
  and the four required test groups (c1_transaction_contracts.rs).
- ⚠️ Cannot verify from diff: test output pristineness (report says clippy is clean in touched files, warnings
  elsewhere pre-existing). The controller may want the exact `cargo test` stderr attached to the report.

### Named-risk checks

1. Drop vs explicit finish, no double count. Checked transaction.rs:307-349: `commit` and `rollback` both
   `take()`/clear `state` before `finish`, and `Drop` calls `finish(Dropped)` only when `state.take().is_some()`.
   Each trial reaches `finish` exactly once. Test :990-1025 mixes the three endings (20 each) and asserts
   rolled_back = 40, dropped = 20. Holds.
2. Budget cap under concurrency and overflow. Checked transaction.rs:390-417: `checked_add` and `next <= cap`
   are evaluated against the value the CAS then compares (`compare_exchange_weak(current, next)`), and a failed
   CAS retries from `observed`. No path stores a value above `cap`. Overflow refuses without a store (test
   :1108-1120). Relaxed ordering is enough for a single counter. Holds.
3. Bounded overhead generalizes. Checked transaction.rs:763-777 and 800-833: every admission requires
   `net_overhead + (C_a+C_v) <= protected_base/10` with `protected_base` nondecreasing, so for an always-failing
   candidate with measured = estimated cost, charged ≤ 0.1 × protected after every step, for any N and any
   costs. The Wilson bound plays no role in the guarantee (it only selects the arm). The test's pass is a
   consequence of this invariant, not of the chosen grid. Holds, up to the documented one-attempt
   estimate/measure gap.
4. No wall time or nondeterminism. Grep of transaction.rs and the test file for Instant, SystemTime, rand,
   HashMap: no hits. Bins use `BTreeMap`; arithmetic is closed form; test :1455-1485 compares two runs'
   decision traces and serialized state. Holds.

### Strengths

- The single-finish structure of `Trial` (Option + consuming methods + guarded Drop) makes risk 1 structurally
  impossible rather than tested-for.
- The UCB doc comment (transaction.rs:640-660) states plainly that Wilson approximates and does not bound the
  Beta quantile, with numbers, and states that the overhead guarantee does not rely on it.
- Decline statuses are type-distinct and tested (:1512-1546); invalid costs/configs are typed errors.
- The mutation check (guard disabled → 12% at τ = 0.4) shows the bounded-overhead test can fail.

### Issues

#### Critical (Must Fix)
None.

#### Important (Should Fix)
None.

#### Minor (Nice to Have)
- transaction.rs:804-809, 763-764 — banked credit and no forgetting. A bin that has had many successes holds a
  large negative `net_overhead` and a high α; if the regime changes inside the bin, the guard and UCB admit
  roughly credit/(C_a+C_v) consecutive failures unthrottled (e.g. 550 successes at τ = 0.2 → about 2200
  failures at 20% overhead in that phase). The cumulative 1.1× bound still holds; the per-phase bound does not.
  Record it for C4 (decay or windowed accounting), or document it on `AdmissionRule`.
- transaction.rs:781-793 — causality is enforced by a "charged_units > 0" check on a caller-supplied f64, not by
  a link to a `Trial`'s ledger. A caller can record an attempt that no trial charged. Consider documenting that
  `charged_units` must come from `Trial::charged_units`, or adding a helper that takes the finished trial's units.
- c1_transaction_contracts.rs:1181-1241 — the test charges exactly the estimated cost, so it never exercises the
  documented "exceeded by at most one attempt's estimation error" case. A test where measured > estimated would
  pin that bound.
- c1_transaction_contracts.rs:1300-1307 — `declines <= m` depends on `k·C_f/10 >= C_a+C_v` and `ceil(10τ)`
  rounding the same way. They can differ by one when 10τ is near an integer. This does not affect the tested
  cases; widen the bound to m + 1 or compute the expected count from the guard itself.

### Assessment

**Task quality:** Approved

**Reasoning:** The four named risks hold by construction: one finish per trial, a CAS against a checked
successor, an invariant overhead guard independent of N and costs, and no clock or RNG. The remaining items are
documentation points and test-precision points for C4.
