# G4-S5B3 reproduction commands

```bash
set -euo pipefail
source /mnt/data/rust_1_94_1_env.sh
export CARGO_TARGET_DIR=/path/to/shared-target
export RAYON_NUM_THREADS=1
export OMP_NUM_THREADS=1
export OPENBLAS_NUM_THREADS=1
export MKL_NUM_THREADS=1
export TERM=dumb

# From repository root.
cargo test --release --offline --locked \
  -p rodas5p-integrators --test early_flow_defect_contracts

cargo test --release --offline --locked \
  -p rodas5p-integrators --test g4_s5b3_attempt_geometry_contracts

cargo test --release --offline --locked \
  -p rodas5p-cli --test cli_contracts \
  early_defect_attempt_geometry_cli_is_threshold_free_and_machine_readable \
  -- --exact

cargo run --profile measurement --offline --locked -p rodas5p-cli -- \
  generic-early-defect-attempt-geometry \
  --profile canonical \
  --output /tmp/G4_S5B3_CANONICAL_RAW.json

python research/generic_event_reentry_shadow_v23/scripts/analyze_g4_s5b3_attempt_geometry.py \
  --input /tmp/G4_S5B3_CANONICAL_RAW.json \
  --output-dir /tmp/G4_S5B3_ANALYSIS

python -m pytest -q \
  research/generic_startup_profile_preflight_v22/tests \
  research/generic_event_reentry_shadow_v23/tests

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --profile measurement --offline --locked -- -D warnings
cargo test --workspace --all-targets --release --offline --locked
cargo test --workspace --all-targets --release --offline --locked -- --test-threads=1
cargo test --workspace --doc --release --offline --locked
cargo build -p rodas5p-cli --profile measurement --offline --locked
```

The canonical raw report contains wall-time observations and is not expected to
be byte-identical across machines. Scientific attempt rows, hard gates, outcome
geometry, and work ledgers must be compared independently of wall time.
