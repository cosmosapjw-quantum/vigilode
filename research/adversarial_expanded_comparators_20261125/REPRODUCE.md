# Reproduction
Pinned repo and original evidence inputs: see receipts/MEASURED_COMMIT.txt, REFERENCE_INPUT_SHA256SUMS and source_inputs. All paths used on owner host explicitly below. Extract this archive in a new directory if rerunning; do not overwrite evidence. Full Diffsol and spatial bridge use absolute repo paths in manifests, replace only with same pinned checkout path if needed. Set CARGO_TARGET_DIR to a new dedicated target, threads to one.
- cargo +stable build --locked --offline --release --manifest-path crates/vigilode-bench/Cargo.toml
- cargo +stable test --locked --offline --release --manifest-path crates/vigilode-bench/Cargo.toml --test adapters --test expanded_diffsol_contract -- --test-threads=1
- taskset -c 2 TARGET/release/vigilode-bench wp --corpus DIR/canonical_expanded.json --out NEW/canonical_expanded.json --source-root REPO ; likewise spatial_expanded.json and each *_tsit45_*.json (timeout75)
- taskset -c 0 julia --startup-file=no --project=DIR/julia_env DIR/run_julia_canonical.jl REPO BASE/run/cross_solver_v1_rust.json implicit ; repeat explicit, redirect stdout JSONL
- taskset -c 4 julia --startup-file=no --project=DIR/julia_env DIR/run_julia_full.jl DIR/source_inputs/full_scope_original.jl BASE/run/full_scope_rust.json
- cargo +stable build --locked --offline --profile measurement --manifest-path DIR/full_diffsol/Cargo.toml ; taskset -c 2 TARGET/measurement/full-scope-suite NEW/full_diffsol.json BASE/run/full_scope_rust.json (stdout JSONL)
- cargo +stable build --locked --offline --release --manifest-path DIR/spatial_bridge/Cargo.toml ; taskset -c 0 julia --startup-file=no --project=DIR/julia_env DIR/run_julia_spatial.jl TARGET/release/libvigilode_bench_spatial_julia_bridge.so DIR/source_inputs/adversarial_spatial_battery_v3.json BASE/run/spatial_battery_v3_rust.json (stdout JSONL)
- Use actual exit records and summarize.py (reads archived raw runs, not new timing); no successful result inferred from exit code alone.
