Rust/cargo 1.94.1, AMD Ryzen 9 5900X, fixed offline dependency lock. Worktree was reused; use an isolated worktree for reproduction, not reset a live tree.
Set CARGO_TARGET_DIR=$HOME/.cargo-targets/vigilode-stage-lattice-20261124/kschain if that cache is appropriate.
Final exact commands:
```sh
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_column_workspace_noalloc_and_transactional -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_source_defect_allocation_contract -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_defect_vector_mutations -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts allocator_positive_and_zero_control -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_defect_mutation_portfolio -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts component_replay -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_general_guard_contracts --test dae_multi_guard_contracts --test dae_pending_guard_contracts -- --nocapture --test-threads=1
cargo test --locked --offline --release -p rodas5p-integrators --features polynomial-events --lib directed_source_defect_refinement_equivalence -- --nocapture --test-threads=1
```
Component replay actually invoked the hash-bound test binary (BINARY.sha256) directly with the identical args. RED uses baseline 11cd137b, preliminary GREEN 53d41f30; first campaign d8fb02f7, final EXEC_SHA.txt. Historical input hashes checked via git show at the first exec commit. Restore raw logs listed in RAW_LOG_CLEANUP.json from their canonical compressed paths in separate scratch, checking SHA-256; guide.py generate verifies MathHarness original source. python3 guide.py select; python3 vector_guidance.py (postcampaign). Prior archived native after.log.gz and component logs belong to m2_dae_guard_workspace_20261126. No holdout.
