In local branch worktree, verify SOURCE_SHA256SUMS; preserve existing outputs and use fresh evidence directory for repeat. Rust 1.94.1, frozen Cargo files.
CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-stage-lattice-20261124/kschain
cargo test --locked --offline --release -j6 -p vigilode --features polynomial-events --test dae_epoch_restart_contracts --test dae_evolution_contracts -- --nocapture --test-threads=1
cargo test --locked --offline --release -j6 -p vigilode --features polynomial-events --test dae_epoch_restart_contracts training -- --ignored --exact --nocapture --test-threads=1
Record stdout with CELL lines as training.log; final test stdout with RESET lines as final_tests.log. Run audit.py, audit_reset.py, guide.py generate (fresh directory only, ledger append is not replay), guide.py select. Verify actual binary path/hash from cargo output. Compare fixed 0-origin old training cells, exclude adaptive/timing as in LEGACY_REPLAY.json. No holdout.
