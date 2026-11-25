# Loop 2 completed: repeated linear solves
RED existing solve_rows misclassified NaN RHS after calling faer; GREEN
typed precondition refusal. Added explicit bounded multi-column LuBatchWorkspace.
16 n/RHS pairs demonstrated zero warm allocations for the low-level
caller-buffer solve, after allocation of owner and caller buffers; not zero
allocation for the whole PreparedRhs call. FaerBatch retains most recent
shape scratch; changed shape rebuilds. ExactSupport stores every nonzero
coefficient including subnormal values, in original column order. Dense
owner keeps dense verifier if support would exceed half its matrix entries.
Both verifier routes use the same original inverse-gap and current RHS.
All 3 core and 8 integrator tests passed, 2 timing cases ignored of which
preflight_measure_exact_support separately ran. Paired support timing and
work records retained, no claim of full IVP speed. Exact harness ledger
90 finite/algebraic claims, fresh replay; Sturm and exact matrix fixtures
separately checked. User scope: prepare M0 only. Next fix benchmark metric
boundary bugs and add actual experiment manifest/combination harness.
