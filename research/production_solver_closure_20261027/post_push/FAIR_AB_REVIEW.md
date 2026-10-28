# PR92 post-push fair-ab budget CI review

Date: 2026-10-28 (Asia/Seoul). Read-only source/log review; no native build, test, retuning, or runtime edit.

## Finding

A1 parity CI job `122293696537` reached the fair-ab target and failed `external_certificate_and_hard_budget_cover_diagnostic_work`. The debug assertion at `crates/rodas5p-fair-ab/src/adapters.rs:218` compares `ledger.operator_total()` (2) with `budget.used()` (1). The target records five passes and one failure. This is a source-level conflict between **attempt accounting** and **admission accounting**, not evidence that a second physical matvec exceeded the budget.

The fixture, fair-ab adapter/budget/ledger implementation, GMRES/common routines, core operator/matrix/norms, dependency lock and toolchain are byte-identical between PR91 `206298b4068d923bb47c7160b111b7439098660c` and PR92 scientific source `78bb8775bf553ad2b514575a0ef4a025446f5d28`. Eleven exact SHA-256 pairs are in the accompanying JSON. This establishes inheritance of the inspected conflict; it does not claim a new PR91 CI execution.

## Exact mechanism

`apply_counted` checks dimensions and increments the solver-level category before entering `apply_with_work`. A budget refusal inside the provider therefore remains a counted attempt. `BudgetedOperator::apply` reserves its token before entering `inner.apply`; `reserve` rejects at the limit without incrementing `used`. `WorkLedger::from_counters` copies the attempt categories into its operator fields. The equality asserted by the fair-ab adapter equates different quantities.

For sequential counted scalar calls, valid shapes, no counter overflow and no raw uncounted applications, let A be attempted entries, E admitted inner entries and R budget refusals. Then exactly **A = E + R** and **E <= B**. The reported row has A=2, E=1, R=1 and B=1. A refused operation never enters `inner.apply`; an admitted inner callback failure does consume a token. The latter must not be confused with a pre-entry budget refusal.

This is not harmless debug instrumentation. If debug assertions are disabled, the original test subsequently requires `result.ledger.operator_total() <= 1`, which is also false when that field retains attempt semantics. Production/research consumers require an honest ledger even when release compilation removes the panic. The error path fails closed; no converged solution is being falsely certified here.

## Relation to the new Lagged accounting fix

The direct test selects `SolverKind::Gmres`. `solve_case` calls `solve_gmres_with_workspace`, then the core `BudgetedOperator` through counted operator calls. It does not invoke the Lagged integrator. The PR92 change in `rodas5p_lagged.rs` independently reconciles solver attempts and physical JVP/Jacobian work inside the Lagged stage driver; this test does not reach that hunk. Shared use of attempt accounting is a conceptual relation, not a causal call-path relation.

## Bounded local repair contract

1. Reproduce this focused native target with the pinned toolchain and preserve the original failure. No unrelated numerical oracle rerun is required for diagnosis.
2. Specify separate A/E/R quantities and category bookkeeping. Preserve terminal attempted work; expose admitted physical entries and budget refusals independently. Whether the public ledger keeps attempt fields or gains explicit admission fields is an API decision, not a tolerance tuning choice.
3. Enforce E<=B and A=E+R in their stated scope. Preserve BudgetExhausted status and failed certificate when diagnostic work cannot be admitted.
4. Verify direct callback-entry atomics for the original B=1 case, exhaustion during the external diagnostic certificate, and failure after a token has been admitted.
5. Check debug/test and release behavior, then the affected fair-ab suite and existing L-RELEASE gate. Do not silently rewrite archived performance evidence or claim it used the repaired consumer.

This task belongs to local release qualification (`Q-FAIR-BUDGET`), with at most three bounded corrections. It does not reopen or exceed the frozen adapter/structural/homotopy numerical correction budgets. Removing the assertion, erasing failed attempts, increasing the budget, or relying on release compilation alone would not repair the accounting contract.
