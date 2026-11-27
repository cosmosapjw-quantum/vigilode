Locked offline release feature polynomial-events. Exact SHA files and input hashes. For timing pin CPU0 (nonexclusive). Prior RED commit intentionally fails focused assertion; final EXEC revision passes. Use separate worktree for RED.
```sh
export CARGO_TARGET_DIR="$HOME/.cargo-targets/vigilode-stage-lattice-20261124/kschain"
for t in native_stage_triangular_skips_actual_jacobian native_stage_goal_portfolio allocator_positive_and_zero_control; do
 taskset -c 0 cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts "$t" -- --exact --nocapture --test-threads=1
done
for t in component_replay guard_affine_graph_native_portfolio; do
 cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts "$t" -- --ignored --exact --nocapture --test-threads=1
done
for t in guard_affine_graph_vector_portfolio integrated_query_workspace_portfolio retained_output_arena_portfolio slab_replay_arena_portfolio shared_engine_portfolio terminal_adjoint_native_portfolio reduced_source_ode_comparator; do
 cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts "$t" -- --exact --nocapture --test-threads=1
done
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_general_guard_contracts --test dae_multi_guard_contracts --test dae_pending_guard_contracts --test dae_reduced_callback_contracts --test dae_interval_scratch_contracts --test dae_scalar_affine_contracts --test dae_conserved_chain_contracts --test causal_budget_contracts -- --nocapture --test-threads=1
python3 research/m2_dae_native_stage_adjoint_20261127/verify.py
python3 research/m2_dae_native_stage_adjoint_20261127/guide.py select
```
Map individual logs to names in RUN_MANIFEST. verify.py accepts raw or gzip; guide generate validates original MathHarness package and creates finite candidate design. All comparisons use measured finite rows, no online tuning. Whole epoch timer excludes materialization. Not a general proof of convergence or full workspace verification.
