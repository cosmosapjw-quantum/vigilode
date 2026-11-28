# M2.33 reproduction
Frozen portfolio production and tests d2e570339c2a709af1c356b974e00e312a868c5b; extra appended test-only edge contract ca1a1d84f8cd1ea454c0b28e89279ad9f2394765. Original EXEC_INPUT_SHA256SUMS recorded at portfolio; edge test source via EDGE_INPUT_SHA256SUMS. FINAL_INPUT_SHA256SUMS is combined identity with unchanged production and extended test.
Rust 1.94.1 project toolchain, locked offline release, polynomial-events. CARGO_TARGET_DIR=$HOME/.cargo-targets/vigilode-stage-lattice-20261124/kschain.
For each pair use taskset -c 0 cargo test --locked --offline --release -p vigilode --features polynomial-events --test <bin> <name> -- --exact --nocapture --test-threads=1:
- spatial_adjoint_contracts source_owned_primal_replay_contract -> replay-contract.log
- spatial_adjoint_contracts source_owned_primal_replay_portfolio -> replay.log
- dae_secant_goal_contracts split_decay_cost_and_dense_parameter_portfolio -> decay.log
- spatial_adjoint_contracts source_owned_primal_replay_floor_and_partial_contract -> edge.log (edge commit)
Original focused runs executed at frozen portfolio SHA, before edge extension; all exact 1 test counts. Then python3 research/m2_spatial_primal_replay_decay_cost_20261128/verify.py and python3 research/m2_spatial_primal_replay_decay_cost_20261128/guide.py select. Candidate generation preceded execution checkpoint. Raw logs may be gzip. Timing only direct carry, six cyclic repeats pinned CPU0, no wall-clock replay claim.
Regression: cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_secant_goal_contracts --test spatial_adjoint_contracts -- --test-threads=1; native: cargo test --locked --offline --release -p rodas5p-integrators --features polynomial-events --test client_stage_system_contracts --test accept_control_contracts --test fast_vector_atol_contracts --test output_observer_contracts -- --test-threads=1. Read actual exits and RUN_MANIFEST.
