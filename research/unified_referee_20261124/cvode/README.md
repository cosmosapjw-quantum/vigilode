# Same-endpoint native reference comparison

From the repository root, use the locked workspace toolchain, SUNDIALS 6.4.1 development libraries/headers, a C compiler and archiver, Julia with the checked-in component environment, and Python 3.10+. The standalone native runner uses the same Rust equations and analytic Jacobians for both native arms. A scratch row-major buffer is transposed for the SUNDIALS dense Jacobian callback. Each cell has two warmups and seven timed rotations. The Julia arm uses its own model implementation.

```bash
d=research/unified_referee_20261124/cvode
cargo build --locked --offline --release --manifest-path "$d/runner/Cargo.toml" --target-dir /tmp/vigilode-u11-native
/tmp/vigilode-u11-native/release/vigilode-cvode-holdout > "$d/raw.jsonl" 2> "$d/cvode.stderr"
JULIA_PKG_OFFLINE=true julia --startup-file=no --project=tools/benchmarks/component_julia "$d/reference.jl" > "$d/julia.jsonl"
python3 "$d/evaluate.py"
```

The linked C bridge uses CV_ONE_STEP and interpolates the terminal point when necessary. Count accepted physical steps in both arms. A CVODE negative flag or early stop produces a charged failure row with no scored endpoint. The frozen empirical uncertainty and its sensitivity to a tighter Julia solve are reported separately. Ratios require both admissible arms and overlapping achieved weighted endpoint errors. CPU congestion, memory pressure and unequal work at the final time limit interpretation of wall times.
