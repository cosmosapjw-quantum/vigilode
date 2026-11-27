Frozen code 98b2ac862b195e0cd1758a68c6b171fa7571f1df. Rust 1.94.1, locked offline release, polynomial-events. CARGO_TARGET_DIR=$HOME/.cargo-targets/vigilode-stage-lattice-20261124/kschain. Before runs: python3 research/m2_weighted_clock_adaptive_20261127/guide.py generate and sha256sum -c EXEC_INPUT_SHA256SUMS at execution checkout.
Run exact single-thread tests via taskset -c 0 cargo test --locked --offline --release -p vigilode --features polynomial-events --test <bin> <test> -- --exact --nocapture --test-threads=1:
- dae_secant_goal_contracts weighted_clock_actual_contract -> contract.log
- dae_secant_goal_contracts weighted_clock_actual_portfolio -> tube.log
- dae_secant_goal_contracts adaptive_signed_actual_portfolio -> goal.log
- spatial_adjoint_contracts ring_local_actual_portfolio -> residual.log
- spatial_adjoint_contracts ring_local_native_actual_portfolio -> native.log
Regression: cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_secant_goal_contracts -- --skip goal_budget_actual_portfolio --skip cost_aware_goal_actual_portfolio --skip cost_aware_actual_portfolio --skip goal_escalation_actual_portfolio --skip structured_goal_actual_portfolio --skip signed_goal_actual_portfolio --skip weighted_clock_actual_contract --skip weighted_clock_actual_portfolio --skip adaptive_signed_actual_portfolio --test-threads=1 (actual command omitted redundant cost_aware_goal_actual_portfolio skip; 6 tests).
python3 research/m2_weighted_clock_adaptive_20261127/verify.py and guide.py select; raw or gzip logs accepted. Inputs hash original preregistration; results appended only in final evidence commit. Timing not an acceptance criterion.
