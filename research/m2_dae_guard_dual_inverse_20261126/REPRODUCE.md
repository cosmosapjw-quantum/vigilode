Locked offline Rust/cargo1.94.1, AMD Ryzen9 5900X 12C/24T x86_64. Main execution at EXEC_SHA.txt; guide-only GUIDE_SHA.txt; unit-only UNIT_EXEC_SHA.txt. Use isolated worktree, never reset an active tree. Exact commands:
```sh
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts bounded_guard_inverse_workspace_contract -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_dual_vector_portfolio -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts allocator_positive_and_zero_control -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_dual_native_portfolio -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts component_replay -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_general_guard_contracts --test dae_multi_guard_contracts --test dae_pending_guard_contracts -- --nocapture --test-threads=1
cargo test --locked --offline --release -p rodas5p-integrators --features polynomial-events --lib retro_dae_evolution:: -- --nocapture --test-threads=1
```
Restore raw log files using RAW_LOG_CLEANUP.json in isolated scratch, verify raw SHA256. guide.py generate verifies exact MathHarness archive and manifest; guide.py select checks current logs vs previous m2_dae_guard_centered_core_20261126 native/vector archive. Resource and bounded comparisons retain actual isolated measurements. No holdout. BINARY.sha256 binds integration binary; source hashes correspond to their individual commits.
