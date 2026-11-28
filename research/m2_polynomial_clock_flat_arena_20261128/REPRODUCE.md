# M2.30 reproduction
Frozen code bc0d208ea4aff04d1f5cd25cea797e64640f5bdd. Rust 1.94.1, release locked offline, feature polynomial-events. CARGO_TARGET_DIR=$HOME/.cargo-targets/vigilode-stage-lattice-20261124/kschain. Check EXEC_INPUT_SHA256SUMS before and after.
For each pair run taskset -c 0 cargo test --locked --offline --release -p vigilode --features polynomial-events --test <bin> <name> -- --exact --nocapture --test-threads=1. Capture output/exit:
- dae_secant_goal_contracts polynomial_clock_fused_contract -> clock-contract.log
- spatial_adjoint_contracts flat_arena_native_contract -> flat-contract.log
- dae_secant_goal_contracts polynomial_clock_refusals -> refusal.log
- dae_secant_goal_contracts polynomial_clock_actual_portfolio -> clock.log
- spatial_adjoint_contracts flat_arena_generic_ode -> ode.log
- spatial_adjoint_contracts flat_arena_actual_portfolio -> arena.log
Then python3 research/m2_polynomial_clock_flat_arena_20261128/verify.py and python3 research/m2_polynomial_clock_flat_arena_20261128/guide.py select. Candidate generation preceded execution checkpoint. Raw logs support gzip. Timing from arena.log, 8 cyclic paired repeats pinned CPU0; no other task launched by this run while timing. Repeated medians and geometric means descriptive, not independent statistical proof.
Regression: cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_secant_goal_contracts --test spatial_adjoint_contracts -- --test-threads=1; native regression: cargo test --locked --offline --release -p rodas5p-integrators --features polynomial-events --test client_stage_system_contracts --test accept_control_contracts --test fast_vector_atol_contracts --test output_observer_contracts -- --test-threads=1; exact product unit: cargo test --locked --offline --release -p rodas5p-integrators --features polynomial-events --lib retro_dae_evolution::polynomial_clock::tests::exact_products_cover_extremes -- --exact --test-threads=1. Read actual exits and RUN_MANIFEST status.
