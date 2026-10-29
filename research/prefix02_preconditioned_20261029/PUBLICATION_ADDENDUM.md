# Publication and provenance addendum

The GitHub contents connector rejected the upload of the standalone `src/bin/precond_bench.rs` with a platform safety-check error. No retry with obfuscation or alternate filename was attempted. The full benchmark source remains in the local SOURCE ZIP and its output CSV/summary is published separately.

To keep the remote research crate buildable without that binary, its remote `standalone/Cargo.toml` was changed to `autobins = false` with only the `precond_bits` bin. The same checkout-variant manifest was tested locally with Rust1.94.1: inherited9 + new16 tests PASS, strict Clippy PASS. The core `src/lib.rs`, `src/preconditioned.rs`, tests and Cargo.lock retain exact tested byte identity. The benchmark was executed before this manifest-only variant and remains reproducible from the full ZIP by replacing Cargo.toml with `evidence/Cargo_full_bench.toml` (source file `src/bin/precond_bench.rs` is included there). No native PR104 source or production path changed.

The new node has a real pre-execution PREREGISTRATION.md at Git commit `528b87f03ab1bb72fe4e158bde36fe0ff0eeb18b`. The root append-only research ledger gained only L-0190; PR105's missing preregistration and ledger remain uncorrected historical CI failures. Full authority and production remain HOLD.
