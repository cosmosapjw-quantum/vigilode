# Loop 2: prepared linear solve and residual work
Existing solve_rows delegates nonfinite input to faer instead of rejecting at
the same precondition boundary as solve/solve_in_place. The warm prepared
FaerBatch route still allocates new aligned matrix and permutation scratch at
each call. Its verifier scans structural zeros with directed arithmetic on
every RHS even when exact known sparsity makes these terms identically zero.
Keep old dense verifier as an explicit oracle and avoid threshold pruning.
Retain same represented matrix identity and the same strict dominance bound.
Measure warm cached workspace allocations and dense vs exact-support full
verified time. This is not a production auto-route change.
