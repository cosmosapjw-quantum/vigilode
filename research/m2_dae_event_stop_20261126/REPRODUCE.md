Use same local worktree and verify SOURCE_SHA256SUMS. Pinned Rust 1.94.1 and Cargo files. If reconstructing, apply IMPLEMENTATION_FROM_E0121EC7.patch to the exact parent, copy EPOCH_TEST_SOURCE.rs and EVENT_TEST_SOURCE.rs to their corresponding crate test paths. No dependency change.
CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-stage-lattice-20261124/kschain
cargo test --locked --offline --release -j6 -p vigilode --features polynomial-events --test dae_event_stop_contracts --test dae_epoch_restart_contracts --test dae_evolution_contracts --test polynomial_event_contracts --test event_transaction_contracts -- --test-threads=1
cargo check --locked --offline -j6 -p vigilode --no-default-features
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_event_stop_contracts training -- --ignored --exact --nocapture --test-threads=1
Record CELL rows in training.log in fresh evidence directory; run guide.py generate (fresh ledger), audit.py, guide.py select. No holdout. Preserve original logs; compare JSON cells not test-duration lines.
