# Validation protocol

## Preflight
```bash
git status --porcelain=v1
git branch --show-current
git rev-parse HEAD
git rev-parse 'HEAD^{tree}'
git rev-parse origin/$(git branch --show-current)
rustc --version
cargo --version
cargo metadata --locked --format-version 1
```

Expected initial identity: branch `research/switcher-v2-20261103`, commit `92d975dd4abfebb924f9a691dac1e5f87fb69943`, tree `2fd16cb90a9e75f9fde87de3584f1feab2668aca`. If advanced, record and revalidate; if diverged, stop before mutation.

## Targeted numerical/contract checks
```bash
cargo test --locked -p vigilode-coupled --test rhs_homotopy_contracts
cargo test --locked -p rodas5p-core --test leja_total_contract
cargo test --locked -p rodas5p-integrators --test conditioned_polynomial_contracts
cargo test --locked -p rodas5p-core --test nonnormal_observers_contracts
cargo test --locked -p vigilode --test production_contracts
cargo test --locked -p rodas5p-integrators --test frontier_session_contracts
cargo test --locked -p vigilode --test production_session_contracts
cargo test --locked -p vigilode --test hybrid_automaton_contracts
cargo test --locked -p rodas5p-integrators --test causal_depth_stage_residual_contracts
```

## Full workspace verification
```bash
cargo fmt --all -- --check
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo build --locked --workspace --release
python3 tools/check_ignored_tests_in_ci.py
```

Record exact argv, cwd, toolchain, source commit/tree, start/end, exit code, stdout/stderr locations and hashes. Do not omit nonzero exits.

## Spatial v2 work-precision
Run only from a clean source tree, without `--allow-dirty-source`:
```bash
cargo run --manifest-path crates/vigilode-bench/Cargo.toml --release -- wp --corpus bench/protocols/adversarial_spatial_grid_v2.json --out bench/out/adversarial_spatial_grid_v2_results.json
cargo run --manifest-path crates/vigilode-bench/Cargo.toml --release -- report --in bench/out/adversarial_spatial_grid_v2_results.json --out bench/out/ADVERSARIAL_SPATIAL_GRID_V2_REPORT.md
```

Before admitting a speedup verify endpoint-only arms have zero output clipping; curves bracket targets; only MATCHED rows form ratios; reference refinement meets preregistered uncertainty; all failures/work remain visible; timing is diagnostic-only until its timing protocol is met.

## Evidence semantics
PASS means the exact current-source command ran and the acceptance passed. FAIL means it ran and failed. CONCERN means evidence is incomplete or materially limited. NOT_RUN means no execution result. NOT_APPLICABLE needs a scope rationale. Finite tests are not class-wide theorem proofs.
