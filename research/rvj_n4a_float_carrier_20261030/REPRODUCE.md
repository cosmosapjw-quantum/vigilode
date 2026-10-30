# N4A independently replayable research-only source

Restore a clean N4A archive. Set `PATH=/mnt/data/vigilode_n3_runtime/rust-1.99.1/bin:$PATH` or equivalent user-provided Rust 1.99.1. No external crates. Python `mpmath==1.3.0` needed for the independent oracle. Original frozen N4 SHA-256: `f821f90575314f03f0f83205e33bb9870579e1423520eed3258b16bf612b5721`. Never overwrite N4/N3H originals.

```bash
python verify_manifest.py .
cd rust
cargo test --locked --offline --all-targets
cargo clippy --locked --offline --all-targets -- -D warnings
rustfmt --check --edition 2024 src/n4a_carrier.rs src/bin/n4a_carrier.rs src/bin/n4a_bench.rs
cargo run --locked --offline --bin n4a_carrier > ../evidence/N4A_REPLAY.csv
cmp ../evidence/N4A_REPLAY.csv ../evidence/N4A_FROZEN_NATIVE_CARRIER.csv
cargo run --release --locked --offline --bin n4a_bench
cd ..
python exact_reference.py
```

`verify_manifest.py` checks original-frozen source-prefix identities, file SHA256 and original `RODAS5P` fixture. Test counts: Rust 33+4+1=38. Scientific N4A initial-window intervals: 12+24+65+257=358. Actual external distinct-author review: NOT_RUN. `T=0.5`, h=.25, p2, epoch0 only. First-window certification does not mutate the source-owned accepted carrier. Artifact CSV includes actual represented physical input, native candidate and interval lower/upper components and physical conditional bound.
