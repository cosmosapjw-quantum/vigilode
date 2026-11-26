Locked offline release Rust/cargo on authorized x86_64 remote. Isolated worktree. Exact revision pointers RED_SHA.txt, GREEN_SHA.txt, EXEC_SHA.txt, GUIDE_SHA.txt, FINAL_GUIDE_SHA.txt. Commands:
```sh
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts nondominant_vector_actual_callbacks_noalloc -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts nondominant_vector_portfolio -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts source_aperiodic_portfolio -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts allocator_positive_and_zero_control -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts component_replay -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_affine_graph_native_portfolio -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_affine_graph_vector_portfolio -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_general_guard_contracts --test dae_multi_guard_contracts --test dae_pending_guard_contracts --test dae_reduced_callback_contracts --test dae_interval_scratch_contracts --test dae_scalar_affine_contracts -- --nocapture --test-threads=1
```
guide.py generate verifies original MathHarness archive/manifest. guide.py select and verify.py handle raw/compressed logs. Original and final guide inputs tracked separately because cross-parameter Pareto was added after the first measured replay. RAW_LOG_CLEANUP.json binds exact original raw bytes. Fraction comparator validates finite claims, not full general solver. Allocation counts thread-local; parallel scan worker allocations and pool setup excluded. No holdout.
