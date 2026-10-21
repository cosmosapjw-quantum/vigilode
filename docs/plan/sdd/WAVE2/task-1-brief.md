## Task 1: C1-transaction — the Transaction and Telemetry types (node id `c1-transaction`)

Plan §4 idea 2, §6 C1, T8. New module `crates/rodas5p-integrators/src/transaction.rs` (plus `lib.rs` export).
No existing driver is migrated in this task.

- `Transaction<S>` over a committed state `S: Clone` (state, controller state, history, Krylov state are the
  caller's `S`): `begin(&committed) -> Trial`, `Trial::charge(WorkCounters)`, `Trial::commit(self) -> S`,
  `Trial::rollback(self)`; work charged to a trial stays in the ledger on rollback (attempted vs completed
  counters); dropping a `Trial` without commit or rollback is a rollback and is counted as such.
- `Budget` with a hard cap in work units reserved before scheduling (the `vigilode-coupled` `Work` ledger
  pattern, `crates/vigilode-coupled/src/lib.rs`), returning a typed `BudgetExhausted` with the work used.
- `AdmissionRule` implementing T8 Proposition 8: per-regime-bin Beta(α, β) posterior of candidate success
  updated only from charged attempts; admit when the upper confidence bound of p exceeds
  (C_a + C_v)/C_f; while the posterior is below threshold, allow at most one attempt per m accepted protected
  steps with m = ⌈10 (C_a + C_v)/C_f⌉. Costs are measured work units passed by the caller. Pure,
  deterministic, no wall time.
- `Telemetry` record: Ritz-value summary (spectral radius estimate, max |Im|/|Re| ratio), refresh q
  estimate, Jacobian drift estimate δ_J, rejection streak — plain data with serde, no driver logic.
- Tests: commit/rollback/drop accounting identities (attempted = committed + rolled back); budget
  reservation never exceeds the cap under concurrent reservation (use std threads with an AtomicU64 CAS like
  `Pexprb54s4BudgetedOperator`); AdmissionRule bounded-overhead property: for a candidate that always fails,
  charged candidate cost over N protected steps ≤ 0.10 × protected cost for N ∈ {100, 1000}; for a candidate
  that always succeeds and is cheaper, admission rate → 1 within 50 attempts.

