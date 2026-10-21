# Independent O-35 work-accounting addendum

Reviewer: `benchmark_audit`, not an author of these changes. Method: bounded read-only source review of the new monitored-GMRES progress path, the private lagged caller, and the relevant core counting wrappers; inspection of the new hostile test cases. No additional native tests were executed or counted by this reviewer.

Decision: **no blocking source finding** in the reviewed O-35 changes. Native validation belongs to the author's execution receipts and must be reported separately.

## Progress and output ownership

`GmresMonitoredProgress` resets before input validation. An attempted column is counted before its preconditioner. Its operator application is counted only after the column preconditioner succeeds and immediately before the operator call. A completed column and `linear_iterations` are recorded after Arnoldi construction, before least squares or the monitor can fail. Least-squares attempts/completions have separate fields. Consequently an operator, preconditioner, least-squares or monitor error retains the completed prefix in the workspace.

The output buffer is still copied only in the `Accept` branch. An unaccepted happy breakdown cannot generate another Arnoldi direction, so clean exhaustion is appropriate; it neither claims convergence nor commits the candidate. A zero initial residual similarly returns unaccepted when no monitor admission exists. This changes the monitored refusal/error behavior deliberately and does not change successful monitored solves or non-monitored numerical arithmetic.

The shared `arnoldi_column` adds an optional counter parameter. Existing non-monitored calls pass `None`; their operator/preconditioner, orthogonalization and update order is unchanged. The larger workspace adds progress storage, not extra heap operations in this path.

## Lagged-caller accounting identities

The private `StageOperator` and `StoredLu` increment their atomic call counts before the actual callback/LU solve. Core `apply_counted` and `apply_preconditioner` count only successful calls. The lagged caller snapshots the success counters immediately before `gmres_stage`, so

`failed = atomic_attempts - (successful_counter_after - successful_counter_before)`

is nonnegative in this private path. The monitor uses only those same operator and preconditioner wrappers; it cannot introduce unrelated increments into the delta. This fact is essential: the formula would require reconsideration if a future monitor performed other counted operator work. Shape validation precedes the private atomic increment, but workspace/system validation and fixed private operator dimensions prevent a shape-failed column from violating the partition.

Failed applications are added exactly once to the corresponding generic work counts. The original successful counts stay untouched. All LU attempts are added once to `direct_solve_calls`, as before. The distinction between `attempted_columns`, `completed_columns`, and `column_operator_applications` correctly handles a failed column preconditioner without subtracting a nonexistent matvec.

The work partition is

`operator_applications = gmres_column_applications + residual_applications`.

Both cycle-start and explicit monitor/initial residual applications belong to the second term. Column progress is read before propagating `outcome?`; failures preserve it. If the initial residual fails or already passes before a monitored call, `gmres_stage` does not consume stale workspace progress, and the outer delta remains zero columns. Completed columns abandoned on an error are added to `abandoned_iterations` once.

## Reviewed hostile cases and limits

The tests cover a second-column operator failure, a second-column preconditioner failure, monitor failure after completed columns, resetting progress on the next invalid call, happy-breakdown Continue, and an exact initial iterate. The end-to-end lagged test injects the fourth JVP failure at the first monitor residual and compares callback count, generic counters and the operator/residual partition while checking successful recovery.

As with the existing `WorkCounters`, these identities assume counters do not overflow their `u64` representation. This addendum does not extend generic callbacks to attempted-work accounting globally, assert a wall-time speedup, or turn the monitored stopping heuristic into a validated numerical certificate.
