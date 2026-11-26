Exact locked offline commands (same linked worktree, Rust 1.94.1 project toolchain, CARGO_TARGET_DIR=$HOME/.cargo-targets/vigilode-stage-lattice-20261124/kschain):
```sh
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_workspace_portfolio -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_general_guard_contracts --test dae_multi_guard_contracts --test dae_pending_guard_contracts -- --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts allocator_positive_and_zero_control -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts component_replay -- --ignored --exact --nocapture --test-threads=1
```
Baseline 13dec36a, after 2472ceb0. Use separate isolated scratch, never reset a live worktree. Decompress retained baseline.log.gz and after.log.gz into the research directory in scratch, then python3 guide.py select; generate verifies original MathHarness archive and manifest. Input hashes bind EXEC SHA, raw output hashes bind logs after decompression; baseline runner was rebuilt and not retained separately (source commit and logs retained). Component comparison uses preceding m2_dae_reduced_callback_20261126/components.log.gz. No holdout.
