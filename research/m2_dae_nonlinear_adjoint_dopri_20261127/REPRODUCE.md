Locked offline release, feature polynomial-events, remote x86_64. Exact RED/GREEN/EXEC/GUIDE SHA and input SHA manifests. Raw or gzip logs accepted. Run in isolated checkout; RED intentionally fails only at RED_SHA.
```sh
export CARGO_TARGET_DIR="$HOME/.cargo-targets/vigilode-stage-lattice-20261124/kschain"
for t in nonlinear_schur_adjoint_actual_portfolio dopri_stage_kernel_actual_portfolio native_stage_split_scratch_resources allocator_positive_and_zero_control; do
 taskset -c 0 cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts "$t" -- --exact --nocapture --test-threads=1
done
for t in native_stage_goal_portfolio terminal_adjoint_native_portfolio shared_engine_portfolio reduced_source_ode_comparator; do
 cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts "$t" -- --exact --nocapture --test-threads=1
done
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_general_guard_contracts --test dae_multi_guard_contracts --test dae_pending_guard_contracts --test dae_reduced_callback_contracts --test dae_interval_scratch_contracts --test dae_scalar_affine_contracts --test dae_conserved_chain_contracts --test causal_budget_contracts --test explicit_contracts --test explicit_hardening_contracts -- --nocapture --test-threads=1
cargo test --locked --offline --release -p rodas5p-integrators --test dopri_stage_kernel_contracts --test dopri5_allocation --test dopri5_contracts --test dopri5_dense_contracts --test dopri5_extreme_contracts -- --nocapture --test-threads=1
```
Use logs adjoint,dopri,split,allocator,prior_stage,prior_adjoint,prior_engine,prior_ode,regression,dopri_regression respectively. guide.py generate validates MathHarness package; verify.py produces ODE_ACCURACY.json, INDEPENDENT_CHECKS.json, RESOURCE_COMPARE.json; regression_verify.py produces REPLAY_COMPARE.json; guide.py select produces SELECTION.json. No exact timing reproduction expected. Original failed RED compile, admission failure and estimated-size verifier failure retained separately.
