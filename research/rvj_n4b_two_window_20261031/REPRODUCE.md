# N4B reproduce, Rust 1.99.1

```bash
bash REPLAY.sh
# Optional, expensive full 180/220-digit 358-mode oracle:
N4B_FULL_180_220=1 bash REPLAY.sh
```

No network access; no third-party Rust crates. `quick_fraction_oracle.py` uses Python standard library and computes 716 exact-dyadic analytical bound tests. `independent_second_window.py` requires mpmath 1.3.0. `independent_physical_bdf.py` requires NumPy/SciPy for a NON_RIGOROUS cross-check. The no-network replay does **not** execute the PR131 original native RODAS5P. `N4B_RUST_BIN` can point to the user's installed Rust 1.99.1 bin directory. The script verifies the complete source archive before execution and fails closed for missing/wrong identities.

Prior N4A source is preserved byte-for-byte in the first 42,406 bytes of `rust/src/n3h_source_bounded_copy.rs`; `PARENT_N4A_MANIFEST.json` is historical, not an active manifest. `MANIFEST.json` is the new N4B identity root. The additional cost benchmark is `cargo run --release --locked --offline --bin n4b_cost`. Benchmark timing claims use 3 repeats and are descriptive only.

All archive source and output files are a local research source-owned model; numerical state outputs and original RODAS stage success are different kinds of claims. No production admission.
