# P1-01R v2.6 Attempt-Causal Trace — PHYS–MATH–CODE AUDIT

## Equation-to-code mapping
`G4S5B0RjfAttemptRow` records every R-JF trial with attempt index, accepted-step count before the trial, proposed `h`, error norm/failure, acceptance, wall time, RHS/JVP/matvec work. Accepted standard atlas rows are emitted only on successful acceptance.

The analyzer maps an event emitted after accepted step `j` to:
1. the accepted attempt that produced step `j`;
2. the first subsequent attempt, which is the runtime-causal target;
3. separately, the first subsequently accepted attempt, which is the old accepted-grid target.

## Code-path reality
- new solver runs: 12 family-sharded R-JF-only runs;
- E-K evaluations: 0;
- E-prefix evaluations: 0;
- active switching: false;
- N=2048: not executed.

The family-sharded runner prevents a long family from destroying receipts from completed families without changing numerical equations or tolerance contracts.

## Regression evidence
Focused fresh checks on the final implementation state before result closeout:
- `attempt_trace_smoke_reconstructs_committed_rjf_rows_and_retains_trials`: PASS;
- `generic_policy_redesign_attempt_trace_command_is_registered`: PASS;
- targeted integrator+CLI Clippy with `-D warnings`: PASS.

All 12 new solver shards returned rc=0. The analyzer's accepted-row parity gate compares against the **raw v2.5 JSON shards**, not CSV round-tripped values; all declared numerical fields are exact.

## Failure ledger
### P0 — accepted-grid future-information contamination
Confirmed by exact frozen k=1 policy events in both dimensions. This blocks accepted-grid prefix timing as runtime authority.

### P1 — k=2 is not causally robust
Two N=512 semilinear k=2 events target rejected/failed first proposals, with accepted-grid `h/h_first = 0.2`.

### P1 — k=3 survivor is development-only
Zero current misses is not independent holdout evidence and cannot authorize N=2048 promotion by itself.

### P1 — actual pexprb level-1 cost still unknown
Historical `exponential_prefix_wall_median` is the G4-S4 fused-Arnoldi acquisition prefix, not the actual pexprb54s4 U2/D2 dependency-level-1 probe. The next node must measure the latter on the first runtime proposal.

## Verdict
Mechanism/result: PASS.
Policy promotion: HOLD.
DAG consequence: open a prefix-only, actual-first-proposal node on N=128/N=512 development data; keep full E continuation and N=2048 sealed.
