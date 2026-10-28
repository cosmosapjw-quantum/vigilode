# N02 independent reference protocol

`n02_independent_reference_v1.json` is a new preregistration. No published E1b
corpus or historical verdict is modified. It runs RADAU5 and CVODE at every
relative tolerance `1e-11`, `1e-12`, `1e-13`, with each problem's reference
absolute tolerance multiplied by `1`, `0.1`, `0.01`. Each level must complete the
whole grid. Both cross-family agreement and each adjacent within-family change
must be at most `1e-10` in the declared common WRMS metric (`error_rtol = 1`).
The finest RADAU5 states supply the error baseline. Failure at any level is kept
and disputes the reference, even if a later run succeeds. Agreement is empirical
on this grid; it is not an enclosure of the exact solution or of times between
sampled points. There is no automatic threshold relaxation or fallback.

Build with locked dependencies, using `CARGO_TARGET_DIR` outside the source
checkout. `build.rs` embeds the exact repository source manifest: all crates,
path dependencies within the repository, root manifest/lock/toolchain files,
fixtures, native bridge sources and Rust literal compile-time assets. Registry
dependencies remain covered by Cargo.lock checksums and Cargo's verification;
system shared-library bytes are not included in this source snapshot. Unsupported
computed includes/external path dependencies/symlink inputs fail closed.

```sh
cargo build --locked --profile measurement --manifest-path crates/vigilode-bench/Cargo.toml
"$CARGO_TARGET_DIR/measurement/vigilode-bench" wp \
  --source-root "$PWD" \
  --corpus crates/vigilode-bench/protocols/n02_independent_reference_v1.json \
  --require-independent-reference --out /absolute/artifacts/n02-results.json
```

Before and after `wp`, the executable verifies its embedded source manifest and
hashes the exact corpus and executable. A stale source, changed corpus or replaced
executable refuses publication of a result document. Git is optional: a source
archive still uses the full manifest. Git metadata, when available, describes the
build context. Runtime HEAD is never reported as the compiled commit. Dirty Git
checkouts are refused by default; explicit `--allow-dirty-source` retains hash
verification but records the exception, so promotion tools can reject it. Output
and logs should be outside the repository to keep the default clean-tree check
meaningful. Boundary checks require a quiescent source checkout; they do not
claim continuous tamper monitoring during execution.

The low-level library `harness::run_corpus` returns unattributed observations;
only the `wp` CLI attaches verified source provenance. A promotion gate must
require `provenance.source_identity.status = VERIFIED_BUILD_SOURCE` and verify
its receipt hashes. A successful process exit alone is not a production verdict.

The standalone test driver `tests/isolated_n02.py --deps <existing-rust-deps-dir>`
compiles the actual harness, corpus, report and metric code with mock solver
adapters. It is a control-flow test, not a native solver/reference experiment.
The full bench crate still requires its exact locked Diffsol dependencies,
gfortran and SUNDIALS development libraries before native integration tests.
