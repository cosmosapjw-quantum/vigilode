# Reproduction

The frozen native crate expects the scratch arrangement `<work>/r4/homotopy/probe` and `<work>/vigilode`. To replay from a published repository folder, copy this lane into that scratch arrangement (or create a temporary probe copy and rebind only the two Cargo path dependencies to the chosen source repository). Preserve the original manifest as the preregistered byte record; log any rebinding.

1. Check source commit and source-file hashes against root's source manifest. Use an isolated target dir and the provided Rust 1.94.1 dependencies.
2. After preregistration is committed, build/run this native crate using `cargo run --offline --locked --manifest-path r4/homotopy/probe/Cargo.toml`, preserving stdout as `native.jsonl`, stderr and exit.
3. Run `python3 r4/homotopy/exact_checks.py --native r4/homotopy/native.jsonl --output r4/homotopy/RESULTS.json`.

The native executable reports observations; it does not assert the source passes. The Python checker retains sound/unsound outcomes and separately validates the proposed exact-arithmetic candidate. A process exit 0 therefore means execution completed, not that all source certificates were valid. The campaign cited in the report is inherited evidence, not a new timing run.
