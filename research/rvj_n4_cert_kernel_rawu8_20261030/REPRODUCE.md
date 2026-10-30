# RVJ N4 source-locked reproduction

The *user-provided Rust1.99.1* installation prefix of the original runner was `/mnt/data/vigilode_n3_runtime/rust-1.99.1/bin`; on another machine substitute the same verified Rust1.99.1 toolchain. The offline main workspace is NOT included; the N4 research crate has no third-party crates and is independently buildable.

```bash
cd VIGILODE_RVJ_N4_KERNEL_20261030
python3 VERIFY.py .
# Add Rust1.99.1 bin directory to PATH first.
cd rust
cargo test --locked --offline --all-targets
cargo clippy --locked --offline --all-targets -- -D warnings
rustfmt --check --edition 2024 --config skip_children=true src/lib.rs src/bin/rodas8.rs tests/independent_fraction_cases.rs
cargo run --locked --offline --bin rodas8
cd ..
python3 python/certify_moments.py evidence/EXACT_DYADIC_MOMENTS_REPLAY.csv
# `certify_moments.py` default checks 240 exact-rational source interval rows.
# Prior original evidence evidence/EXACT_DYADIC_MOMENTS.csv is retained unchanged.
python3 python/rodas8_rank2_reference.py evidence/RODAS8_PY_REFERENCE_REPLAY.json
python3 python/compare_n3h_rodas8.py evidence/SOURCE_MATCHED_ACCURACY_REPLAY.json
```

Python benchmarks require installed NumPy, SciPy, mpmath (the exact versions and host are recorded in `ENVIRONMENT.json`); those are independent computational checks, not dependencies of the standalone Rust crate. `src/n3h_source_bounded_copy.rs` contains byte-identical N3H source prefix followed by a small exported wrapper; the verifier checks the original SHA-256 prefix. `fixtures/rodas5p_coefficients_snapshot.json` is the original represented 8-stage coefficient snapshot with SHA256 fixed. The compiled executable is research-only and its accepted/rejected transaction is not the full native production controller.

`native_rodas_attempt/` is a preserved incomplete integrated build **attempt**, which requires a separate exact older offline Cargo workspace. Its CI/Cargo test was NOT_RUN_TO_COMPLETION. Do not pass it off as a regression PASS.
