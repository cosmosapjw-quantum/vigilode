# Final validation receipt

The final tagged tree is required to pass the following exact commands with Rust 1.94.1 and the
locked offline vendor directory:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --release --offline --locked -- -D warnings
cargo test --workspace --release --offline --locked -- --test-threads=1
cargo build --workspace --release --offline --locked
```

The delivery gate also requires:

- `rodas5p validate` machine-readable status `pass`;
- two byte-identical canonical screens equal to the tracked canonical artifact;
- canonical SHA-256
  `a665562e3088950e3a7d64e985f220708cac6cad7356f096baf378e4ee27026e`;
- 41 workspace/doc tests with zero failures in both the working tree and a clean Git archive;
- independent canonical assertions for 57 cases, 114 controls, 8,208 candidates, 3,510 fast
  accepts, 4,698 fallbacks, zero numerical failures, zero one-step false accepts, and 401 low-depth
  fast rows;
- clean `git diff --check`, clean worktree, and `git fsck --full`;
- no dynamic BLAS/LAPACK/OpenBLAS/MKL dependency;
- all Phase 0–10 and closeout files present and nonempty.

The raw command transcript, including exact HEAD, timestamps, and clean-archive output, is shipped
as `validation/FINAL_VALIDATION_RAW.log` in the delivery package. Failed validation-script attempts
are documented in `harness/FAILURE_LOG.md`; none were treated as completion evidence.
