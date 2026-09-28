# E-08 — WorkCounters vs atomic ground truth (pre-registration)
Written BEFORE any run. Binary: harness/src/bin/e08_work_counters.rs.
Wrapped closures (rhs/jvp/partial_t/jacobian) around prothero_robinson_problem(-1e4, 1, 0) (JVP-only and with-Jacobian variants)
and ScientificCorpusV2 robertson-ramped n=96 rtol 1e-8 (segment 0, span clipped to 10 time units) count every call with AtomicU64.
Arms: mf_adaptive_reject (rtol 1e-8, h0=max_step=0.1*span), mf_adaptive_gmresfail (maxiter 4, restart 4), the dense variants,
expo_pexprb54s4_fused, fixed_direct (h=span/50), adaptive_direct_reject, adaptive_gmres_gmresfail_generic. One dev-profile run too.
PASS/FAIL (fixed before running): FAIL if for any arm that returns counters, rhs_calls != atomic rhs, or ft_calls != atomic ft
(for problems with analytic partial_t), or the JVP evaluations actually performed (atomic) are not equal to
jvp_calls + linear_matvecs + recycle_refresh_matvecs + diagnostic_matvecs + block_matvecs + phi_krylov_vectors + mass_matvecs.
Any nonzero difference is reported with the code path; classification into severity is left to the report.

# Results (appended after the run; pre-registration above unchanged)
Status: **FAIL on the pre-registered JVP rule; PASS for rhs_calls and ft_calls** (0 mismatches in 24 arms, both profiles).
- Matrix-free path: `jvp_calls` == atomic JVP callbacks exactly (e.g. PR 760=760, robertson 29385=29385, forced-failure arm 9800=9800). It already includes the Krylov/diagnostic applies, so `jvp_calls + linear_matvecs + diagnostic_matvecs` over-counts by exactly `linear_matvecs + diagnostic_matvecs`.
- Generic path (`integrate_adaptive_observed_with_config` + GMRES, JVP-only problem): `jvp_calls` = 140 vs 760 real JVP callbacks (robertson 8841 vs 74897); the rest is visible only as `linear_matvecs`/`diagnostic_matvecs` (`OdeProblem::linearize` ClosureOperator has zero `application_work`, problem.rs:239-251 / operator.rs:123-125). With an explicit Jacobian `jvp_calls`=140 while 0 JVP callbacks ran. => `jvp_calls` means different things on different paths.
- Forced failures are counted (robertson: 84/114 GMRES failures, 93/121 rejected steps; generic GMRES arm 1228 failures) and never rolled back.
- Direct on a JVP-only problem: fixed path hard-errors and drops counters; generic adaptive path retries a configuration error 17-19 times to min_step ('maximum step count or minimum step reached').
- pexprb54s4 exponential arm: not runnable (non-autonomous problems rejected).
- Dev profile: counters/atomics bit-identical to measurement; no debug_assert fired.
Files: e08_out.jsonl (24 rows, full WorkCounters + atomics + checks), e08_dev_out.jsonl, stderr logs, build.log, results.json.
