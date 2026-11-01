# VigilODE standalone G0 R7: native transactional q1/q2 vs protected RODAS5P

Date: 2026-11-01 KST. Parent HEAD 87ae3f8b102fdcebb6ccf9950ed0fce3ae6a8e1d, PR #179. Commit and publish this preregistration BEFORE fresh native R7 computation. Active stage: R7 discriminating validation, stage-local hostile check and handoff.

## Question and immutable source
Does the real native transactional q1/q2 adaptive dense solver beat the protected matrix-free RODAS5P on full original IVPs at independently measured *matched achieved global error*? Previous unaccelerated frontier_q2 is NOT a substitute for native transactional q1/q2.
- Original original-source WP3 example: crates/rodas5p-integrators/examples/vigilode_wp3_cubic_rodas5p.rs Git blob 5dc8df34d314c0d8562c6f548a574625cef442ff, with frozen original 4 labels, RHS and analytic JVP. Refined asymmetric_2 is only a resolution control.
- Transactional engine: crates/rodas5p-integrators/src/transactional_q1_q2.rs Git blob 420eea69619ecfb5c150a22af41e946c13de1b85.
- Original dense integrator: crates/rodas5p-integrators/src/dense_output_v2.rs Git blob 9ac5e6984fae836408a496f43aac18e6e023254e.
- Original raw-U source Git blob d68239d53977e36f98b186b627aa0c4c5c7de7cb. Tableau SHA256 211255f6125b648c6955833c3da537d9c031ec78168b2918563c29fa88c8bce6.
- Horizon 0 to 0.1. Original step h=0.1/steps, steps 64,128,128,128; all labels i*h (i=0..steps).
- Both algorithms use same native OdeProblem with original RHS, JVP, analytic family, identical adaptive config with initial_step=h, max_step=0.1, max_attempts=100000, and rtol=1e-4,1e-6,1e-8, atol=rtol*0.01. TransactionalQ1Q2Config default, threads=1 (not parallel-speed claim); no relaxed gates.
- Full dense requested original output grid, bit identity and state vectors. Native traces include all original counters, rejected/fallback/critical-depth work, raw durations measured up to three times per method/case/tolerance; failure is typed, preserved and not silently retried with different settings.
- Independent numerical reference: same original cubic RHS parsed from Rust and two independent SciPy solve_ivp methods Radau and DOP853, rtol=2e-13 atol=2e-15 on identical represented grid. Require state-wise max cross-reference deviation <=2e-10. If violated, error comparison blocked. This is numerical reference, not formally interval certified.
- 3-point achieved global error vs wall/work precision. No ranking without overlapping achieved errors; report error, time, all attempts, accepted/rejected, JVP/RHS, fallback and q-lane. Code-build/measurement hardware recorded. Entire solver runtime includes callbacks/diagnostics/dense output, but reference Python costs excluded.
- Existing fused exponential-Rosenbrock pexprb54s4 retained in inventory. Its current API lacks the same dense output capability; do NOT rank endpoint-only runtime against dense-grid native runs. Future separate comparable challenger.
- Newton-free cubic homotopy from R5 is a different implicit BE/RK stage system; preserve as optional algebraic fast provider only with the original stage residual. Native fallback remains protected.

## Stop / gate
If build, source byte identity, output grid or requested error fails: save typed failure, preserve strongest protected route. Do not call a source-scoped first-step screen a full-IVP/production win. No core solver, tolerances, default, main, common authority checker or original 23-node DAG changes.

Official DAG 7/23, successor admitted 0/12, production HOLD. Scope independent standalone VigilODE only, excluding applied REI/BASS branches. No merge, no auto next R8, no tolerance relaxation.
