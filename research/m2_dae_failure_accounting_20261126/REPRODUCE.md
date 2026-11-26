Branch research/m2-dae-failure-accounting-20261126. EXEC_SHA.txt, INPUT_SHA256SUMS and binary hashes bind actual execution.
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_failure_accounting_contracts -- --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_failure_accounting_contracts audited_refusals -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_failure_accounting_contracts audited_training -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_failure_accounting_contracts training -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts component_replay -- --ignored --exact --nocapture --test-threads=1
Restore focused2, refusals, audited_training, training, components raw logs from gzip into scratch. Run audit.py with prior sibling m2_dae_projection_workspace_20261126 compressed logs and AUDIT.json; guide.py select. guide.py generate verifies original archive and exact ledger. Compare logical receipt text keyed by case/solver/cert/order/method, not durations. No holdout.
