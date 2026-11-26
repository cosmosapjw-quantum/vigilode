Rust/cargo1.94.1, AMD Ryzen9 5900X 12C/24T x86_64. Locked offline project toolchain. Use isolated scratch worktree, never reset an active tree. RED937e0ea2, preliminary GREEN5f003bdb, main EXEC_SHA.txt, unit-only follow-up UNIT_EXEC_SHA.txt. Commands:
```sh
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_borrowed_c_allocation_contract -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_centered_vector_portfolio -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts allocator_positive_and_zero_control -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_centered_native_portfolio -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts component_replay -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_general_guard_contracts --test dae_multi_guard_contracts --test dae_pending_guard_contracts -- --nocapture --test-threads=1
cargo test --locked --offline --release -p rodas5p-integrators --features polynomial-events --lib retro_dae_evolution:: -- --nocapture --test-threads=1
```
Restore compressed raw logs via RAW_LOG_CLEANUP.json in isolated scratch, verify raw SHA256. guide.py generate checks exact MathHarness archive and all154 files; guide.py select verifies native/vector core, mutations and previous m2_dae_guard_poly_gs_20261126 native/vector logs. Component comparison uses its components.log.gz. BINARY.sha256 binds integration test binary; UNIT_INPUT_SHA256SUMS binds separate unit-only execution commit. No holdout.
