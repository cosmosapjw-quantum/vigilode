Locked offline release, remote x86_64, feature polynomial-events. RED intentionally fails at RED_SHA; execute final source at EXEC_SHA. Inputs are bound by exact SHA files. Original source design and previous receipts preserved.
```sh
export CARGO_TARGET_DIR="$HOME/.cargo-targets/vigilode-stage-lattice-20261124/kschain"
for t in scalar_integral_actual_one_exponential scalar_integral_actual_portfolio blocked_dopri_actual_portfolio allocator_positive_and_zero_control;do
 taskset -c 0 cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts "$t" -- --exact --nocapture --test-threads=1
done
cargo test --locked --offline --release -p rodas5p-integrators --test dopri_stage_kernel_contracts --test dopri5_allocation --test dopri5_contracts --test dopri5_dense_contracts --test dopri5_extreme_contracts -- --nocapture --test-threads=1
for t in nonlinear_schur_adjoint_actual_portfolio dopri_stage_kernel_actual_portfolio native_stage_split_scratch_resources terminal_adjoint_native_portfolio;do
 cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_resource_baseline_contracts "$t" -- --exact --nocapture --test-threads=1
done
cargo test --locked --offline --release -p vigilode --features polynomial-events --test dae_general_guard_contracts --test dae_multi_guard_contracts --test dae_pending_guard_contracts --test dae_reduced_callback_contracts --test dae_interval_scratch_contracts --test dae_scalar_affine_contracts --test dae_conserved_chain_contracts --test causal_budget_contracts --test explicit_contracts --test explicit_hardening_contracts -- --nocapture --test-threads=1
```
Capture logs green_scalar,scalar,blocked,allocator,dopri_regression,prior_nonlinear,prior_dopri,prior_stage,prior_affine,regression respectively. guide.py generate validates actual MathHarness package and freezes finite design. verify.py validates actual outputs, regression_verify.py checks prior logical/complete records, guide.py select replays measured candidates. Raw or gzip logs accepted. Timing cannot be expected to reproduce bitwise. implement.py is a historical applied patch, not a rerunnable build script.
