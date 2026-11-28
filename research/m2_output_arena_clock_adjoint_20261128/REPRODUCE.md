# M2.29 reproduction
Frozen code 415e5bac800f4ef6fc1d1d1ab3b8b267b2093db4. Rust 1.94.1, release locked offline, feature polynomial-events. CARGO_TARGET_DIR=$HOME/.cargo-targets/vigilode-stage-lattice-20261124/kschain. Check EXEC_INPUT_SHA256SUMS before and after.
For each pair run taskset -c 0 cargo test --locked --offline --release -p vigilode --features polynomial-events --test <bin> <name> -- --exact --nocapture --test-threads=1. Capture output/exit:
- dae_secant_goal_contracts structured_clock_adjoint_contract -> adjoint-contract.log
- spatial_adjoint_contracts native_arena_zeroheap_contract -> arena-contract.log
- spatial_adjoint_contracts native_arena_actual_portfolio -> arena.log
- spatial_adjoint_contracts native_arena_generic_ode -> ode.log
- dae_secant_goal_contracts structured_clock_actual_portfolio -> clock.log
- dae_secant_goal_contracts tiny_log_actual_portfolio -> tiny.log
Then python3 research/m2_output_arena_clock_adjoint_20261128/verify.py and python3 research/m2_output_arena_clock_adjoint_20261128/guide.py select. Archived generation output guide.py generate was before execution freeze. All raw logs support gzip storage.
Regression: cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_secant_goal_contracts -- --test-threads=1; same for spatial_adjoint_contracts. Native regression: cargo test --locked --offline --release -p rodas5p-integrators --features polynomial-events --test client_stage_system_contracts --test accept_control_contracts --test fast_vector_atol_contracts --test output_observer_contracts -- --test-threads=1. Read actual exit and RUN_MANIFEST status. No timing authority.
