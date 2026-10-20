#!/usr/bin/env bash
set -euo pipefail
OUT=$(realpath "$1")
NODE=crates/vigilode-coupled
RESEARCH=research/coupled_economy_20261020
PARENT=15c43ce76734d0a560fcec5909ac49f1e6f2d347
STAGE=setup
trap 'code=$?; printf "{\"stage\":\"%s\",\"exit_code\":%s}\n" "$STAGE" "$code" > "$OUT/EXIT_STATUS.json"' EXIT
rustup toolchain install 1.94.1 --profile minimal --component rustfmt --component clippy > "$OUT/rust_install.log" 2>&1
python3 -m pip install --disable-pip-version-check 'numpy==2.3.5' 'scipy==1.16.3' > "$OUT/python_install.log" 2>&1
grep -q 'test result: FAILED. 1 passed; 3 failed' "$RESEARCH/runs/39353982085/RED.log"
STAGE=publish_source
if ! grep -q 'pub fn coupled_step_residual_first' "$NODE/src/lib.rs"; then
 python3 "$RESEARCH/prepare_implementation.py" "$OUT"
fi
python3 "$RESEARCH/prepare_benchmark.py" "$OUT"
cargo +1.94.1 fmt --manifest-path "$NODE/Cargo.toml"
git config user.name 'github-actions[bot]'
git config user.email '41898282+github-actions[bot]@users.noreply.github.com'
# Never modify/stage workflows from this contents-only token.
git add "$NODE"
if ! git diff --cached --quiet; then
 git commit -m 'feat: bind residual-first derivative evaluation and capped speculation before measurement [skip ci]'
 git push origin HEAD:refs/heads/research/coupled-economy-20261020
fi
SOURCE=$(git rev-parse HEAD)
printf '%s\n' "$SOURCE" > "$OUT/EXEC_SOURCE_COMMIT.txt"
cp "$NODE/Cargo.lock" "$OUT/Cargo.lock"
git fetch --depth=1 origin "$PARENT"
git diff --exit-code "$PARENT" -- crates/rodas5p-core crates/rodas5p-integrators crates/rodas5p-krylov Cargo.toml Cargo.lock > "$OUT/UNCHANGED_PARENT_LIBRARIES.txt"
{ rustc +1.94.1 -Vv; cargo +1.94.1 -Vv; python3 -m pip freeze; } > "$OUT/VERSIONS.txt"
STAGE=tests
cargo +1.94.1 test --manifest-path "$NODE/Cargo.toml" --locked -- --nocapture --test-threads=1 > "$OUT/TESTS.log" 2>&1
tail -100 "$OUT/TESTS.log"
cargo +1.94.1 clippy --manifest-path "$NODE/Cargo.toml" --all-targets --locked -- -W clippy::all > "$OUT/CLIPPY.log" 2>&1
STAGE=build
cargo +1.94.1 build --manifest-path "$NODE/Cargo.toml" --profile measurement --example economy_bench --locked > "$OUT/BUILD.log" 2>&1
BINARY="$PWD/$NODE/target/measurement/examples/economy_bench"
sha256sum "$BINARY" > "$OUT/BINARY_SHA256SUM"
STAGE=preflight
"$BINARY" run few-mode 96 1 0.001 lazy-4 1 0 > "$OUT/PREFLIGHT.json"
python3 - "$OUT/PREFLIGHT.json" <<'PY'
import json,sys
r=json.load(open(sys.argv[1]));a=r['result']['attempt'];c=a['candidate'];w=a['work']
assert r['root_completed']and r['deterministic']and c['basis_dimension']==3
assert w['stage_jvp']==8*c['basis_dimension']*w['newton_updates']
assert w['validation_jvp']==8 and w['unused_stage_jvp_avoided']==24
PY
STAGE=accuracy_and_benchmark
python3 "$RESEARCH/benchmark.py" --binary "$BINARY" --fixture fixtures/rodas5p_coefficients_snapshot.json --source "$SOURCE" --out "$OUT" 2>&1 | tee "$OUT/benchmark.log"
STAGE=independent_readback
python3 "$RESEARCH/audit.py" "$OUT" 2>&1 | tee "$OUT/AUDIT.log"
STAGE=complete
