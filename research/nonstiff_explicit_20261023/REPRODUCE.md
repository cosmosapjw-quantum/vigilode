# Reproduce the nonstiff explicit implementation and local comparisons

The executable source for this study is commit
`45037fc24e8f8101605f309e1fdb3a9a4e3df56d` on
`research/nonstiff-explicit-20261023`. A later report commit may include these
instructions and evidence; it does not change the source identity used below.
The commands preserve the existing checkout, create a detached worktree, and
write every new result outside that worktree. They use the local machine only.
Do not use NCP or SSH.

`REPRODUCE.md` and `HANDOFF_KO.md` are report additions after the executable
source commit. Read or save them from the published report before entering the
pinned worktree. Their absence inside that older source worktree is expected;
the benchmark scripts and test targets used below are present there.

This document describes a fresh reproduction. A missing, failed, or incomplete
benchmark remains unmeasured; these commands do not imply a performance result.
Consult the published evidence and report for the status of a particular run.

## 1. Create an isolated checkout and record its identity

Run this in Bash from any existing checkout of `cosmosapjw-quantum/vigilode`.
The output of `mktemp` is the new run directory. Keep it, including failures.
No reset, clean, force checkout, or deletion of an existing directory is needed.

```bash
set -euo pipefail
VIGILODE_EXISTING=$(git rev-parse --show-toplevel)
VIGILODE_SOURCE=45037fc24e8f8101605f309e1fdb3a9a4e3df56d
VIGILODE_RUN_ROOT=$(mktemp -d "${TMPDIR:-/tmp}/vigilode-nonstiff-local.XXXXXXXX")
git -C "$VIGILODE_EXISTING" rev-parse HEAD > "$VIGILODE_RUN_ROOT/original-head.txt"
git -C "$VIGILODE_EXISTING" status --porcelain=v1 > "$VIGILODE_RUN_ROOT/original-status.txt"

# Fetch only if this exact published object is unavailable locally.
if ! git -C "$VIGILODE_EXISTING" cat-file -e "$VIGILODE_SOURCE^{commit}" 2>/dev/null; then
  git -C "$VIGILODE_EXISTING" fetch origin research/nonstiff-explicit-20261023
fi
git -C "$VIGILODE_EXISTING" cat-file -e "$VIGILODE_SOURCE^{commit}"
git -C "$VIGILODE_EXISTING" worktree add --detach "$VIGILODE_RUN_ROOT/checkout" "$VIGILODE_SOURCE"
cd "$VIGILODE_RUN_ROOT/checkout"
test "$(git rev-parse HEAD)" = "$VIGILODE_SOURCE"
test -z "$(git status --porcelain=v1)"
git rev-parse HEAD > "$VIGILODE_RUN_ROOT/SOURCE_COMMIT.txt"
git rev-parse 'HEAD^{tree}' > "$VIGILODE_RUN_ROOT/SOURCE_TREE.txt"
printf '%s\n' "$VIGILODE_RUN_ROOT" > "$VIGILODE_RUN_ROOT/RUN_DIRECTORY.txt"
```

If fetching is unavailable, transfer the exact source through an authorized Git
bundle or clone. Do not substitute another commit while keeping the source label.
The presence of a different branch or a dirty existing checkout is not a reason
to modify that checkout.

## 2. Prepare the pinned toolchain and dependency cache

The repository pins Rust **1.94.1** in `rust-toolchain.toml`. This work environment
has an installed copy and Cargo cache at the paths below. Those paths are a
convenience for this environment, not a claim about another host's cache.
On another host, make its already prepared Rust 1.94.1 and matching cache/vendor
directory available before continuing.

```bash
VIGILODE_RUNTIME=/workspace/scratch/0dc1e721cf82/task_runtime
if [ -x "$VIGILODE_RUNTIME/installed/bin/rustc" ]; then
  export PATH="$VIGILODE_RUNTIME/installed/bin:$PATH"
  export CARGO_HOME="$VIGILODE_RUNTIME/cargo"
fi
export CARGO_TARGET_DIR="$VIGILODE_RUN_ROOT/build"
export CARGO_BUILD_JOBS=2
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
rustc --version | tee "$VIGILODE_RUN_ROOT/rustc-version.txt"
cargo --version | tee "$VIGILODE_RUN_ROOT/cargo-version.txt"
[[ "$(rustc --version)" == "rustc 1.94.1 "* ]]
cargo metadata --locked --offline --format-version 1 \
  > "$VIGILODE_RUN_ROOT/cargo-metadata.json" \
  2> "$VIGILODE_RUN_ROOT/cargo-metadata.stderr.log"
```

If metadata fails because a locked dependency is absent, preserve the failure
and prepare the dependency cache **before** timing. With authorized network
access, `cargo fetch --locked` from this exact checkout is sufficient when the
registry and Git sources are reachable. Otherwise transfer a matching Cargo
cache or a vendor directory with its source-replacement configuration from a
prepared host. Keep `Cargo.lock` unchanged; do not remove `--locked`, silently
upgrade packages, or claim a solver failure for a missing build dependency.

Analysis requires Python 3, NumPy, and Matplotlib. SciPy is optional and supplies
the permitted VODE/DVODE adapter. Record the actual environment rather than
assuming the same packages are installed everywhere:

```bash
python3 - "$VIGILODE_RUN_ROOT/python-environment.json" <<'PY'
import importlib.metadata, json, platform, sys
import numpy, matplotlib
versions = {}
for name in ("numpy", "matplotlib", "scipy"):
    try:
        versions[name] = importlib.metadata.version(name)
    except importlib.metadata.PackageNotFoundError:
        versions[name] = None
with open(sys.argv[1], "x") as f:
    json.dump({"python": platform.python_version(), "packages": versions}, f, indent=2)
    f.write("\n")
PY
```

Prepare any missing Python dependencies outside the timed campaign, using an
approved pinned environment or an offline wheel cache. The benchmark runners do
not install packages or create remote resources.

## 3. Run the 25 targeted tests and build both measurement examples

The four targets contain 12 kernel contracts, one kernel allocation test,
eight public API contracts, and four ensemble contracts. They cover order and
stage time, FSAL/rejection counts, scalar/vector tolerance handling, invalid
input, recoverable/fatal/malformed callbacks, partial results, unsupported
features, actual ensemble concurrency, and deterministic member ordering.
These tests do not replace the existing RODAS5P regression suites or prove
production suitability for every problem.

```bash
cargo test --locked --offline -p rodas5p-integrators \
  --test dopri5_contracts --test dopri5_allocation \
  2>&1 | tee "$VIGILODE_RUN_ROOT/tests-kernel.log"
cargo test --locked --offline -p vigilode \
  --test explicit_contracts --test explicit_ensemble_contracts \
  2>&1 | tee "$VIGILODE_RUN_ROOT/tests-public-ensemble.log"
cargo build --locked --offline --profile measurement -p vigilode \
  --example nonstiff_bench --example nonstiff_ensemble_bench \
  2>&1 | tee "$VIGILODE_RUN_ROOT/build-measurement.log"
```

The inherited release optimization is level 3; `measurement` disables LTO and
uses 16 codegen units. Do not present those timings as an LTO release comparison.
The strict shell setting stops this sequence on a failed build or test while
leaving the logs intact. Investigate failures before starting the timed run.

## 4. Pin external comparators before the campaign

External families are limited to **VODE/DVODE, diffsol, and Julia ODE solvers**.
The Rust RODAS5P paths are the internal baselines. `--external` discovers the
installed SciPy VODE/DVODE wrapper, using Adams on nonstiff families and BDF on
the stiff controls. This is not a SciPy RK45, Radau, or LSODA comparison.
The VODE adapter records Python, NumPy, SciPy, and its method in each output.

For a reproducible external claim, preserve an `EXTERNAL_ADAPTERS.json` beside
the campaign with these fields for each available adapter:

| Field | Required evidence |
| --- | --- |
| Solver and method | Exact algorithm/options; do not label an unidentified Julia method merely “Julia” |
| Versions | Python/NumPy/SciPy for VODE; diffsol crate version and source revision; Julia version and package versions |
| Pins | Hash and copy of the Python environment lock, diffsol `Cargo.lock`, or Julia `Project.toml` and `Manifest.toml` |
| Adapter identity | Source revision and SHA256 of source, executable, and any wrapper script |
| Build/runtime | Compiler/flags, BLAS/thread configuration, and Julia warmup/JIT policy |
| Availability | `AVAILABLE` with evidence, or `NOT_RUN` with the reason |

No diffsol or Julia adapter is bundled in this implementation. Supply a pinned
local executable only after verifying its protocol against
[`BENCHMARK_PLAN.md`](BENCHMARK_PLAN.md) and `vode_adapter.py`: one JSON request
on stdin and one complete `vigilode.nonstiff-benchmark-run.v1` result on stdout.
All requested states, times, analytic references, timing scope, and honest work
counts must be present. The analyzer independently checks saved outputs. Missing
adapters stay `NOT_RUN`; an unverified version does not support an external
performance claim. Julia JIT and imports must be disclosed separately from
steady solve time, with the same policy used across all repetitions.

## 5. Execute the expanded comparison and analyze saved states

This suite spans decay, nonautonomous forcing, nonlinear logistic growth,
harmonic and rapid oscillations, damping, mixed scales, nonnormal growth,
large vectors, and moderate/stiff negative controls. `--full` uses four
tolerances and adds the 16384-component vector. Every comparison uses the same
five requested observations and analytic target. It runs one warmup and three
randomized timed rounds. The 7200-second budget applies to this campaign;
compilation and environment preparation occur first. Per-process timeouts and
budget exhaustion remain recorded, not removed from the dataset.

```bash
VIGILODE_ADAPTER_ARGS=()
# Optional: set these variables to verified, pinned local executables.
if [ -n "${VIGILODE_DIFFSOL_ADAPTER:-}" ]; then
  test -x "$VIGILODE_DIFFSOL_ADAPTER"
  VIGILODE_ADAPTER_ARGS+=(--diffsol-adapter "$VIGILODE_DIFFSOL_ADAPTER")
fi
if [ -n "${VIGILODE_JULIA_ADAPTER:-}" ]; then
  test -x "$VIGILODE_JULIA_ADAPTER"
  VIGILODE_ADAPTER_ARGS+=(--julia-adapter "$VIGILODE_JULIA_ADAPTER")
fi
python3 research/nonstiff_explicit_20261023/run_bench.py \
  --binary "$CARGO_TARGET_DIR/measurement/examples/nonstiff_bench" \
  --out "$VIGILODE_RUN_ROOT/full-comparison" \
  --source-commit "$VIGILODE_SOURCE" \
  --external --full --budget-seconds 7200 \
  "${VIGILODE_ADAPTER_ARGS[@]}" \
  2>&1 | tee "$VIGILODE_RUN_ROOT/full-comparison-runner.log"
python3 research/nonstiff_explicit_20261023/analyze_bench.py \
  "$VIGILODE_RUN_ROOT/full-comparison" \
  2>&1 | tee "$VIGILODE_RUN_ROOT/full-comparison-analysis.log"
```

`PLAN.json` records the binary hash, source, dimensions, tolerances, CPU
affinity, and available CPU quota. `RECEIPTS.json` keeps process outcomes;
`SUMMARY.json` and `REPLAY.json` separate numerical validity from process
success. The analyzer produces work–precision figures and the raw-output
archive/manifest. Cost ratios use only successful, replay-verified runs meeting
the same achieved-error threshold. Error at five sampled times is not a
uniform-in-time certificate. Observed min/max ranges are not confidence
intervals. VODE's Python callback costs and interpolated output policy are
explicit comparison limitations.

Each runner requires a previously nonexistent output directory. If a run is
interrupted or fails, retain it and choose a new directory for the next attempt.
Do not overwrite it or mix trials from different source/binary identities.

## 6. Measure independent-trajectory throughput separately

```bash
python3 research/nonstiff_explicit_20261023/run_ensemble_bench.py \
  --binary "$CARGO_TARGET_DIR/measurement/examples/nonstiff_ensemble_bench" \
  --out "$VIGILODE_RUN_ROOT/ensemble-comparison" \
  --source-commit "$VIGILODE_SOURCE" --budget-seconds 180 \
  2>&1 | tee "$VIGILODE_RUN_ROOT/ensemble-comparison-runner.log"
```

This additional, at-most-180-second campaign compares one and two workers on
16 independent 4096-component decays at two tolerances. It emits its own
summary, replay, and archive. Pool construction is recorded separately; member
setup and integration are included in solve time. Verify identical saved states
across worker counts before interpreting throughput. This does not measure
parallel Runge–Kutta stages or a single-trajectory homotopy acceleration.

Finally record `git status --porcelain=v1` and verify the checkout still has the
pinned HEAD. Preserve all logs and source/binary/environment identities with
the new results. A rerun is new evidence; it must not overwrite historical
failures or change the meaning of previously published measurements.

## Public API and supported scope

This complete example can be used as `src/main.rs` in a client crate depending
on the pinned `vigilode` package. The RHS closure returns `Ok(())` after filling
every output component:

```rust
use vigilode::{Atol, Dopri5, Options, Problem};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // y' = 2t, y(0) = 0; exact solution y(t) = t^2.
    let problem = Problem::new(1, |t, _y, dy| {
        dy[0] = 2.0 * t;
        Ok(())
    });
    let options = Options {
        rtol: 1e-8,
        atol: Atol::Scalar(1e-10),
        h0: 0.01,
        ..Options::default()
    };
    let solution = Dopri5::solve_observed(
        &problem,
        (0.0, 1.0),
        &[0.0],
        &options,
        &[0.0, 0.25, 0.5, 0.75, 1.0],
    )?
    .ensure_success()?;
    println!("y(1) = {:?}", solution.final_state);
    println!("RHS calls = {}", solution.stats.counters.rhs_calls);
    Ok(())
}
```

`Dopri5` is an explicitly selected Dormand–Prince 5(4) path for forward,
identity-mass ODEs, including nonautonomous RHS functions and scalar/vector
absolute tolerances. It does not call Jacobians, JVPs, partial time derivatives,
matrix factorizations, or Newton solvers. Declared mass matrices, events, and
dense output are refused before RHS callbacks; observation times use actual
step clipping. There is no automatic stiffness detector or method switching.
The shared RODAS5P `Options::tier` does not turn this into a Rosenbrock solve.

Every RHS call must implement the same mathematical function throughout a
solve so that FSAL reuse is valid. `Recoverable` callback errors may retry with
a smaller step; fatal errors and incomplete/nonfinite successful callback
outputs stop with the last accepted state. Runtime stops return a partial
solution with `success == false`; `ensure_success()` makes this a typed error
without losing the partial data. Explicit stability restrictions and global
accuracy still require application-specific validation. The ensemble API runs
independent solves and retains each member's error and status separately.

For a combined two-hour benchmark allocation after environment preparation, use `--budget-seconds 6900` for the main campaign and `--budget-seconds 180` for the ensemble, leaving 120 seconds for analysis. Compilation and dependency preparation are separate; a strict overall wall deadline must include those explicitly.
