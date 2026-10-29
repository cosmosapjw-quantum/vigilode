# VIGILODE-PREFIX01 standalone research crate

This is an opt-in **standalone synthetic** GMRES-prefix / original-residual certificate, **not** a patch to the VigilODE production solver. It requires Rust 1.94.1, x86_64 nearest-even and gradual underflow, a fixed represented point Jacobian and clock, and no external crates. The exact physics source/callback binding is a caller premise.

Use `THEORY_KO.md`, `REPORT_KO.md`, `NEXT_THREAD_HANDOFF_KO.md` for mathematics, tests, limitations and next steps.

```bash
cargo test --offline --all-targets -- --test-threads=1
cargo clippy --offline --all-targets -- -D warnings
cargo fmt --all -- --check
cargo run --offline --release --bin bit_probe > evidence/bit_probe.csv
python3 evidence/verify_bits.py
cargo run --offline --release --bin prefix_bench > evidence/benchmark.csv
cargo run --offline --release --bin matched_bench > evidence/matched_benchmark.csv
cargo run --offline --release --bin negative_bench > evidence/negative_benchmark.csv
python3 evidence/analyze_bench.py
python3 evidence/analyze_matched.py
```
