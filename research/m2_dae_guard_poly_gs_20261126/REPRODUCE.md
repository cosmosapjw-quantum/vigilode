Rust/cargo1.94.1, AMD Ryzen9 5900X 12C/24T x86_64. Locked offline project toolchain. Use isolated scratch worktree, never reset an active tree. Baseline RED9905f6c8, preliminary GREENe05c1b1c, final EXEC_SHA.txt. Exact commands:
```sh
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_poly_ownership_allocation_contract -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_gs_shared_kernel_noalloc -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_poly_gs_vector_portfolio -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts allocator_positive_and_zero_control -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_poly_gs_native_portfolio -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts component_replay -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_general_guard_contracts --test dae_multi_guard_contracts --test dae_pending_guard_contracts -- --nocapture --test-threads=1
cargo test --locked --offline --release -p rodas5p-integrators --features polynomial-events --lib bounded_polynomial_scratch_tests -- --nocapture --test-threads=1
cargo test --locked --offline --release -p rodas5p-integrators --features polynomial-events --lib guard_defect_scratch_contracts -- --nocapture --test-threads=1
```
In an isolated research dir restore raw logs according to RAW_LOG_CLEANUP.json and verify raw SHA256. guide.py generate verifies MathHarness archive and manifest; guide.py select compares current native/vector log against prior m2_dae_guard_defect_20261126 archive. Component comparison likewise uses prior components.log.gz. No protected holdout used. BINARY.sha256 binds native integration test binary.
