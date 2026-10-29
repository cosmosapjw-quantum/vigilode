# VIGILODE-PREFIX02: source-guarded preconditioned GMRES original-residual certificate

This is a self-contained research crate stacked on PR105. It is NOT a native solver integration.

`src/preconditioned.rs` implements both left and right diagonal preconditioned GMRES with two-pass MGS, incremental Givens projected triggers, original-equation outward residual, stage inverse bound, source/clock/norm/epoch guards, warm starts, restart, and transactional caller output.

Use Rust 1.94.1, `cargo test --locked --offline --all-targets`, `cargo clippy --locked --offline --all-targets -- -D warnings`. Exact bit-level oracle and 7-block synthetic paired benchmark are in `evidence/`.

See THEORY_KO.md and REPORT_KO.md for precise scientific scope and limitations. Original VigilODE 7/23 and central REI 28/88 unchanged, production HOLD.
