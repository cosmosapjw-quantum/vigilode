Locked offline release Rust/cargo on authorized remote x86_64. Isolated worktree. RED_SHA.txt, GREEN_SHA.txt, EXEC_SHA.txt, GUIDE_SHA.txt bind exact revisions. RED and GREEN use identical focused test.
```sh
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts integrated_certificate_baseline_profile -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts certificate_workspace_actual_slab_noalloc -- --exact --nocapture --test-threads=1
taskset -c 0 cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts integrated_certificate_workspace_portfolio -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts allocator_positive_and_zero_control -- --exact --nocapture --test-threads=1
RUST_MIN_STACK=524288 cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts certificate_workspace_stack_smoke -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts component_replay -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_affine_graph_native_portfolio -- --ignored --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts guard_affine_graph_vector_portfolio -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts bounded_sturm_portfolio -- --exact --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_general_guard_contracts --test dae_multi_guard_contracts --test dae_pending_guard_contracts --test dae_reduced_callback_contracts --test dae_interval_scratch_contracts --test dae_scalar_affine_contracts -- --nocapture --test-threads=1
```
Also stack smoke at RUST_MIN_STACK=1048576 and2097152. TIMING_AFFINITY.txt binds actual CPU. guide.py generate verifies original MathHarness archive and 154 files, guide.py select/verify.py accept compressed or raw logs. inventory.py lexical scan. RAW_LOG_CLEANUP.json binds raw byte identities. Finite exact checks, not general qualification.
