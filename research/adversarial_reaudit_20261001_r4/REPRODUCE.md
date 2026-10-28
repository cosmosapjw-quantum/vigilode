# R4 reproduction

Published findings refer to production source `1c54194123ee6abc6daa512e8574922f510b4e2c`, with preregistered inputs at `b2914f3e3c03eda60a8547619db6284aac8250a2`. Report-only HEAD changes do not change this source identity. No new branch is required. Keep the published audit untouched; use a fresh replay directory.

## Environment

Rust/Cargo 1.94.1, Python 3.12 standard library, Linux `flock`. Production Cargo.lock registry dependencies are matched in `evidence/runtime/dependency_match.json`. Numerical scripts do not need NumPy/SciPy, SymPy or mpmath. Original setup receipts include the supplied Rust/vendor archive identities and preflight outputs. `setup_runtime.py` is a forensic record of this workspace preparation; it expects the exact attachment layout and an already extracted inner vendor archive, so it is not a universal installer.

Use an installed Rust1.94.1 and normal populated registry cache, or the provided vendor directory with an isolated Cargo home. For an offline vendor, its configuration must replace crates-io with the absolute vendor path. Never change the repository Cargo.lock to accommodate a missing package. Shared Cargo target/lock avoids competing builds; `CARGO_TARGET_DIR` must be absolute.

Example layout is `<work>/vigilode`, `<work>/r4`, `<work>/runtime_r4`. Clone the existing branch into `vigilode`, copy the report directory into sibling `r4`, then optionally detach the clone at the audited source. The clone retains the preregistration commit object required by the runtime runner. Compare every file in SOURCE_MANIFEST before executing; a report-only checkout is acceptable only if those hashes match.

## Frozen inputs

Run the published `validate_bundle.py --repo <repo>` to check the registered-input and source hashes plus package structure. It does not perform the scientific probes. A supplied final `MANIFEST.sha256` can be checked by `sha256sum -c MANIFEST.sha256` from the published node.

Polynomial/homotopy/statistics manifests expect `../../../vigilode/crates` from `r4/<lane>/probe`. Time's registered runner copies its probe to a temporary location and rebinds only `../../../../crates/...` dependencies to `--repo`. Preserve original manifest bytes; path rebinding in a temporary copy is an environment operation. Time's `RESOLVED_Cargo.lock` records its dependency graph. The final polynomial Cargo.lock is the first-run resolution, recorded separately from the preregistered inputs.

## Native and oracle commands

From `<work>`, after setting Cargo/toolchain environment:

```bash
export CARGO_TARGET_DIR="$PWD/runtime_r4/replay-target"
export CARGO_INCREMENTAL=0
export CARGO_BUILD_JOBS=1
export CARGO_PROFILE_DEV_CODEGEN_UNITS=1
export CARGO_PROFILE_TEST_CODEGEN_UNITS=1
mkdir -p runtime_r4
python3 r4/time/run_probe.py --repo vigilode --output r4/time
python3 r4/time/exact_oracle.py --input r4/time/native_raw.jsonl --output r4/time/EXACT_ORACLE.json
flock runtime_r4/cargo-build.lock cargo run --offline --locked --manifest-path r4/polynomial/probe/Cargo.toml > r4/polynomial/native.jsonl 2> r4/polynomial/native.stderr
python3 r4/polynomial/oracle.py
flock runtime_r4/cargo-build.lock cargo run --offline --locked --manifest-path r4/homotopy/probe/Cargo.toml > r4/homotopy/native.jsonl 2> r4/homotopy/native.stderr
python3 r4/homotopy/exact_checks.py --native r4/homotopy/native.jsonl --output r4/homotopy/RESULTS.json
flock runtime_r4/cargo-build.lock cargo run --offline --locked --manifest-path r4/statistics/probe/Cargo.toml -- "$PWD/vigilode" > r4/statistics/native.stdout.jsonl 2> r4/statistics/native.stderr
python3 r4/statistics/exact_session_interval.py --output r4/statistics/EXACT_SESSION_INTERVAL.json
python3 r4/decision/independent_median_check.py --candidate r4/statistics/EXACT_SESSION_INTERVAL.json --output r4/decision/independent_median_check.json
```

Capture each command's exit status and stop on failure; do not continue after a failed build with stale output. These commands run in the fresh replay copy, never overwrite the evidence in the original report. Native probes emit observations including failures, so exit0 is not a scientific PASS. Do not reinterpret malformed witness controls as failures of an unmodified constructor.

Full bounded regression (the same fixed plan; absolute paths avoid the recorded path incident):

```bash
python3 r4/evidence/runtime/run_bounded_suite.py \
  --repo "$PWD/vigilode" \
  --target-dir "$CARGO_TARGET_DIR" \
  --evidence-dir "$PWD/r4/evidence/runtime/replay" \
  --build-lock "$PWD/runtime_r4/cargo-build.lock" \
  --plan "$PWD/r4/evidence/runtime/FROZEN_PLAN.json" \
  --prereg-commit b2914f3e3c03eda60a8547619db6284aac8250a2
```

The recorded campaign builds149harnesses and lists735tests. The300s native budget intentionally yields named omissions on this host. To close only those omissions use LOCAL_REMAINDER_PLAN; do not pretend the original campaign was complete. The future normalized-path runner is explicitly NOT_EXECUTED in this report.

## Stored first failures and auxiliary checks

The initial tar ownership-error stderr is stored as gzip encoded in base64 plus BINARY_LOG_ENCODING.json; decode the base64, verify the gzip hash, then decompress if needed. No executable binaries are published. The reviewer's first exact-check script failed only while serializing a rational exceeding Python's4300-digit string limit; first script/stderr and the corrected serialization-only version remain in decision/. This reviewer check was chosen after candidate evidence under the registered independent-review plan, not a preregistered candidate experiment or a new performance trial.

Machine validation and repository process checks are recorded in VALIDATION.json. `check-research-node.py --base 1c54194123ee6abc6daa512e8574922f510b4e2c` verifies the append-only result ledger; `check-authority-refs.py` verifies remote source citation reachability. These are process gates, distinct from the source invariants that failed.
