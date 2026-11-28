# M2.39 reproduction
Base 8e7f8e94499313c498e08ebc639a013a600829bd; frozen native source bfc8b10a072e85cacc50470d0820c3c1c963924f. Missing API compile refusal 1cf652c9 (not behavioral RED). Original frozen verifier had missing newline between return and import; actual SyntaxError preserved in verify.log/exit. Correction 2aedd391aa7c1d84b77e28bc0bcef9721eca997e adds verify_fixed.py with only that newline; no native or candidate input changed. EXEC_INPUT_SHA256SUMS and CORRECTED_VERIFY_SHA256 verified.
Rust locked offline release, CARGO_TARGET_DIR=$HOME/.cargo-targets/vigilode-stage-lattice-20261124/kschain. For each test:
taskset -c 0 cargo test --locked --offline --release -p vigilode --features polynomial-events --test spatial_adjoint_contracts <name> -- --exact --nocapture --test-threads=1
embedded_goal_monitor_contract -> contract.log
native_embedded_goal_ode_portfolio -> ode.log
embedded_goal_actual_portfolio -> portfolio.log
python3 research/m2_goal_weighted_checkpoint_monitor_20261128/verify_fixed.py
python3 research/m2_goal_weighted_checkpoint_monitor_20261128/guide.py select
python3 research/m2_goal_weighted_checkpoint_monitor_20261128/posthoc.py
Related regression: same cargo command for each of bounded_checkpoint_capture_generic_ode, sparse_checkpoint_generic_ode_portfolio, online_checkpoint_generic_ode_portfolio, subset_cache_repeated_adaptive_contract, sparse_checkpoint_local_partial_contract, virtual_terminal_small_pool_contract, compact_model_fallback_partial_contract, compact_forward_only_setup_contract, partial_suffix_accounting_regression. Read actual exits and RUN_MANIFEST. Full workspace and separate native crate regression not run. Timing CPU0 four cyclic repeats of four candidates on ten mass/local offset tight strata; 160 cold +160 immediate repeat measurements; setup excluded. Logs may be gzip. No default promotion.
