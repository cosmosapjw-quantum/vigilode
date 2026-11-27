Locked offline release, feature polynomial-events, remote x86_64. Exact RED/GREEN/EXEC/GUIDE SHA and input SHA manifests. Run with CARGO_TARGET_DIR as recorded in prior handoffs (or fresh build).
```sh
for t in shared_engine_actual_setup_reduces_alloc shared_engine_portfolio terminal_adjoint_native_portfolio reduced_source_ode_comparator allocator_positive_and_zero_control; do
 taskset -c 0 cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts "$t" -- --exact --nocapture --test-threads=1
done
for t in component_replay guard_affine_graph_native_portfolio; do
 cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts "$t" -- --ignored --exact --nocapture --test-threads=1
done
for t in guard_affine_graph_vector_portfolio integrated_query_workspace_portfolio retained_output_arena_portfolio slab_replay_arena_portfolio; do
 cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts "$t" -- --exact --nocapture --test-threads=1
done
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_general_guard_contracts --test dae_multi_guard_contracts --test dae_pending_guard_contracts --test dae_reduced_callback_contracts --test dae_interval_scratch_contracts --test dae_scalar_affine_contracts --test dae_conserved_chain_contracts --test causal_budget_contracts -- --nocapture --test-threads=1
```
guide.py generate verifies MathHarness archive and154 files. verify.py uses raw or gzip logs; generates ODE_ACCURACY.json prior to guide.py select. Exact Fraction enclosures verify source-derived gradient and forward samples. Forward sampled numerical accuracy is not full DAE slab qualification. No independent review or full workspace claim.
