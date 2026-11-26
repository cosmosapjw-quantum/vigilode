Locked offline Rust/cargo1.94.1. Use isolated worktree, never reset active tree. RED_SHA.txt, GREEN_SHA.txt, EXEC_SHA.txt main, GUIDE_SHA.txt selection, EXTRA_SHA.txt test-only. Exact commands:
```sh
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts scalar_reuse_reconstruct_allocation_contract -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_affine_graph_vector_portfolio -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts allocator_positive_and_zero_control -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_affine_graph_native_portfolio -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts scalar_reconstruction_portfolio -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts component_replay -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_general_guard_contracts --test dae_multi_guard_contracts --test dae_pending_guard_contracts --test dae_reduced_callback_contracts --test dae_interval_scratch_contracts -- --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_general_guard_contracts -- --nocapture --test-threads=1
```
Restore raw logs via RAW_LOG_CLEANUP.json and verify hashes in isolated scratch. guide.py generate checks MathHarness; guide.py select compares native/vector with m2_dae_guard_dual_inverse_20261126 archive and recon finite corpus. RESOURCE_COMPARE.json and RECONSTRUCTION_COMPARE.json store actual allocation/work comparisons. No holdout.
