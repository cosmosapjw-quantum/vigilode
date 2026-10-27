# Exact integer entry work policy: changed-dependency protocol

The user requested constants derived from stated mathematical objectives instead
of arbitrary tuning. The previous 12-arm parity control is retained verbatim,
including its source `selective_jvp_parity_executed.rs`, results and receipt.
This follow-up changes the representation of the strict entry decision, not
the model, accuracy contract, or a fitted numerical threshold.

The fixed existing integer work objective is
C = N_rhs + N_jvp + N_ft + n N_jacobian + N_linear_solve.
For two successfully qualified methods at the same checkpoint and replay
endpoint, minimizing that objective prefers the candidate iff C_candidate <
C_explicit; the break-even ratio is exactly one. Ties retain the incumbent.
The new independent default-false flag will calculate this inequality using
checked u128 arithmetic, so equal and adjacent counts beyond binary64 integer
precision remain distinguished. Legacy entry_cost_ratio=0.8, its validation,
and all callers with the new flag false remain unchanged.

Since already charged callback work L is a nondecreasing lower bound on final
candidate work C, L >= C_explicit makes strict improvement impossible. The
native guard stops after the callback that exceeds its integer ceiling, so
C_explicit-1 is the exact positive-cost ceiling. Zero incumbent cost also has
no possible strict winner. If the ceiling cannot be represented by the guard,
its optional early stop is declined; the existing attempt budget remains.
The callback that first crosses a ceiling is charged, as before.

Replace only the fourth example arm with the explicit strict policy and
otherwise unchanged seeded configuration. Keep all twelve model/configuration
arms in one binary. Preserve all original six- and nine-arm data. Verify each
old 12-arm numerical outcome, segment counter, interval, time and state is
identical after ignoring only descriptive policy metadata. No timing, holdout,
or general optimality claim. The benchmark's objective unit weights are fixed
accounting conventions; pressure, retry/backoff, return margin, step controller
safety factors, and observation windows are not established universal optimal
constants by this change.

Run focused arithmetic contracts (tie, one unit, large integer, weighted
counters, same replay qualification, conservative ceiling) plus the existing
checkpoint-seed and evidence-switch tests. Then build and execute the single
bounded twelve-arm example. Stop after this changed-dependency verification;
no parameter search.

Outputs: EXACT_WORK_RESULTS.json, EXACT_WORK_SUMMARY.json,
EXACT_WORK_EXECUTION_RECEIPT.json, EXACT_ENTRY_WORK_POLICY.md.
