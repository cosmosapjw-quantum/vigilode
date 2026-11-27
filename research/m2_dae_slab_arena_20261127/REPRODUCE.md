# Reproduce
Locked offline release on authorized remote x86_64, exact RED/GREEN/EXEC/GUIDE revision files and input hashes.
```sh
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts compact_slab_driver_profile -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts slab_dense_power_into_actual_noalloc -- --exact --nocapture --test-threads=1
taskset -c 0 cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts slab_replay_arena_portfolio -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts allocator_positive_and_zero_control -- --exact --nocapture --test-threads=1
RUST_MIN_STACK=524288 cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts slab_replay_stack_smoke -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts component_replay -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_affine_graph_native_portfolio -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_affine_graph_vector_portfolio -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts integrated_query_workspace_portfolio -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts retained_output_arena_portfolio -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_general_guard_contracts --test dae_multi_guard_contracts --test dae_pending_guard_contracts --test dae_reduced_callback_contracts --test dae_interval_scratch_contracts --test dae_scalar_affine_contracts --test dae_conserved_chain_contracts -- --nocapture --test-threads=1
```
Also stack smoke RUST_MIN_STACK=1048576 and2097152. guide.py generate verifies original MathHarness archive/154 manifest files; select and verify.py read raw or gzipped logs. Profile replay component counts are not additive trace. Timing scope full audited epoch/drop only, excludes legacy materialization. Independent Fraction manufactured analytic checks and actual legacy restoration identity.
