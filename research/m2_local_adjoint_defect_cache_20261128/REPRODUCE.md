# M2.34 reproduction
Frozen source/candidates bbdabf330888171401c99c10888acd0d5d717e12; Rust 1.94.1, offline locked release polynomial-events. CARGO_TARGET_DIR=$HOME/.cargo-targets/vigilode-stage-lattice-20261124/kschain. Verify EXEC_INPUT_SHA256SUMS before/after.
For each test use taskset -c 0 cargo test --locked --offline --release -p vigilode --features polynomial-events --test spatial_adjoint_contracts <name> -- --exact --nocapture --test-threads=1:
- exact_cache_and_local_replay_contract -> contract.log
- local_capture_capacity_and_cache_edges -> edge.log
- bounded_dense_capture_generic_ode -> ode.log
- cross_call_cache_and_local_actual_portfolio -> portfolio.log
Confirm one test passed per invocation. python3 research/m2_local_adjoint_defect_cache_20261128/verify.py; python3 research/m2_local_adjoint_defect_cache_20261128/guide.py select. Raw logs can be gzip. Prior M2.33 replay source rows/initial identity used by verifier; exact time and goal key. Cache paired timings run in portfolio on CPU0 six cyclic repeats after warming same key. No local wall-clock claim.
Regression: cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_secant_goal_contracts --test spatial_adjoint_contracts -- --test-threads=1. Native: cargo test --locked --offline --release -p rodas5p-integrators --features polynomial-events --test client_stage_system_contracts --test accept_control_contracts --test fast_vector_atol_contracts --test output_observer_contracts -- --test-threads=1. Read actual exits and RUN_MANIFEST.
