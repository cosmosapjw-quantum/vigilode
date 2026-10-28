## Global Constraints

- Everything in the wave-1 Global Constraints applies (toolchain `RUSTUP_TOOLCHAIN=stable`, `--locked`,
  opt-in new behaviour, bit-identical defaults, type-distinct statuses, counted failures, commit identity and
  trailers, push only to the node's own branch, node report `docs/plan/nodes/<node-id>.md`, fmt/clippy).
- Build environment: `export RUSTUP_TOOLCHAIN=stable CARGO_TARGET_DIR=/home/claude/targets/<node-id>
  CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0`, `-j 1`. Each node has its own
  target dir (a shared one cross-links worktrees). Never `cargo clean`, never `cargo test --workspace`.
  Check `df -h /home/claude` before large builds; stop and report below 4 GB free.
- No new external crates in the root workspace. Exception: Task 5's benchmark crate is its own Cargo
  workspace (like `crates/vigilode-hybrid`) and may depend on `diffsol` with the exact version and lock
  entries already pinned in `crates/vigilode-hybrid/Cargo.lock`.
- Comparator toolchain (installed 2026-10-21): GNU Fortran 13.3.0 (`apt-get install gfortran`) and SUNDIALS 6.4.1
  (`apt-get install libsundials-dev`, headers in /usr/include, libs in /usr/lib/x86_64-linux-gnu). Pinned Fortran
  sources: Netlib DVODE + LINPACK `dgefa/dgesl/dgbfa/dgbsl` from branch `research/benchmark-results-20261019-dual`,
  `research/strict_solver_comparison_20261019/refined_runs/39261895983/sources/` (with NETLIB_SHA256SUMS); Hairer
  RADAU5/RODAS from the Assimulo 3.0 sdist (sha256 `61cdca4562745078c966cc4fab44d8bc0db11a9e6856b45c470ac65c16af9e8a`,
  `assimulo/thirdparty/hairer/{radau_decsol.f,rodas_decsol.f,LICENSE_HAIRER}`). Record every source hash.
- Wave-1 rulings bind: `wrms` stays strict for convergence callers, `step_error_wrms` carries the zero-scale
  rule for end-of-step norms; `Tier::Performance` routes no lagged driver until a B1 card passes its kill
  gate; preregistered kill gates are never re-scored or loosened.

