Locked offline Rust/cargo on authorized remote x86_64. Isolated worktree. Exact revision pointers RED_SHA.txt, FIRST_GREEN_SHA.txt, GREEN_SHA.txt, EXEC_SHA.txt, GUIDE_SHA.txt. RED and two GREEN attempts use the identical focused command.
```sh
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts bounded_affine_actual_callbacks_noalloc -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts bounded_affine_source_refusals_and_cancellation -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts source_picard_endpoint_portfolio -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts allocator_positive_and_zero_control -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts bounded_affine_root_portfolio -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts component_replay -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_affine_graph_native_portfolio -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_affine_graph_vector_portfolio -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_general_guard_contracts --test dae_multi_guard_contracts --test dae_pending_guard_contracts --test dae_reduced_callback_contracts --test dae_interval_scratch_contracts --test dae_scalar_affine_contracts -- --nocapture --test-threads=1
```
guide.py generate verifies original MathHarness archive and manifest; guide.py select and verify.py support compressed and raw logs. RAW_LOG_CLEANUP.json records original exact byte identities. The independent Fraction comparator checks original-root/closed-form endpoint enclosures, not whole driver. EVIDENCE_SHA256SUMS binds all retained files. No holdout.
