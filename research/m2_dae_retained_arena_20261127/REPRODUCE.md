# Reproduce
Locked offline release on authorized remote x86_64; exact RED_SHA.txt/GREEN_SHA.txt/EXEC_SHA.txt/GUIDE_SHA.txt and input SHA manifests. Test counting allocator preserves previous Counts schema; separate signed calling-thread live delta, not allocator internal temporary overlap, RSS or stack.
```sh
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts native_packet_borrowed_view_noalloc -- --exact --nocapture --test-threads=1
taskset -c 0 cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts retained_output_arena_portfolio -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts allocator_positive_and_zero_control -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts component_replay -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_affine_graph_native_portfolio -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_affine_graph_vector_portfolio -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts integrated_query_workspace_portfolio -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_general_guard_contracts --test dae_multi_guard_contracts --test dae_pending_guard_contracts --test dae_reduced_callback_contracts --test dae_interval_scratch_contracts --test dae_scalar_affine_contracts --test dae_conserved_chain_contracts -- --nocapture --test-threads=1
```
guide.py generate verifies original MathHarness archive and154 manifest files; guide.py select and verify.py accept raw/gzip. Actual full materialization and retained report identity checked by Rust before rows emitted; independent Fraction checks original native endpoint/stage bits, coefficient enclosures and alternate exact polynomial formula. No independent general physical theorem/holdout. Timing includes actual audited full epoch and drop, not materialization.
