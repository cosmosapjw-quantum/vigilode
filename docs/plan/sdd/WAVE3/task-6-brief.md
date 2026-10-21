## Task 6: CI-docs — production CI gate, MSRV, examples (node id `ci-docs`)

Plan §1 G-A (`cargo semver-checks` clean, docs build), §6 E2; wave-1 ruling (MSRV 1.94.1 unverified in the
cloud). The repository has research workflows only; no workflow builds or tests the production crates.

**Files:**
- Create: `.github/workflows/production.yml`.
- Create: `crates/vigilode/examples/{robertson.rs, dense_output.rs, dae_mass.rs, banded_fd.rs}`.
- No edit to any `src/` file (other nodes own them).

**Exact content:**
- Triggers: `pull_request` and `push` to `main`, on paths `crates/**`, `Cargo.toml`, `Cargo.lock`,
  `rust-toolchain.toml` and the workflow file. The toolchain is the pinned 1.94.1 from
  `rust-toolchain.toml`, which makes this the MSRV check.
- Jobs:
  1. `fmt`: `cargo fmt --check -p vigilode -p rodas5p-core -p rodas5p-krylov`. Integrators is excluded:
     `tests/production_audit_packet_curve.rs` is unformatted from before.
  2. `clippy`: `cargo clippy -p vigilode --all-targets --locked -- -D warnings`.
  3. `test`: `cargo test --locked -p vigilode -p rodas5p-core -p rodas5p-krylov -p vigilode-coupled`, and
     `cargo test --locked -p rodas5p-integrators --lib` plus the `--test` list of production contract tests
     named in Tasks 2–4 of waves 1–3.
  4. `docs`: `RUSTDOCFLAGS="-D warnings" cargo doc -p vigilode --no-deps --locked`,
     `cargo test -p vigilode --doc --locked`, `cargo build -p vigilode --examples --locked`, and each example
     run with `cargo run --example`.
  5. `semver`: `cargo-semver-checks` installed at an exact pinned release (`cargo install --locked --version <X>`;
     the implementer records X), run with `--baseline-rev ${{ github.event.pull_request.base.sha }} -p vigilode`.
     It has `continue-on-error: true` until the first published 0.x baseline; G-A makes it blocking.
  6. `bench`: `apt-get install gfortran libsundials-dev`, then
     `cargo test --manifest-path crates/vigilode-bench/Cargo.toml --locked`.
- Examples, each asserting one check and exiting 0:
  - Robertson with an analytic dense Jacobian; mass conserved to 1e-8.
  - Prothero–Robinson with dense output and `at(t)`; interior ≤ 2 tolerance units.
  - Robertson as an index-1 DAE with mass diag(1, 1, 0); constraint ≤ 1e-12.
  - Brusselator 1D n = 100 with `Jacobian::FiniteDifference` and `.banded(2, 2)`.

- [ ] **Step 1:** Write the examples. Run `cargo build -p vigilode --examples --locked -j 1` and
  `cargo run -p vigilode --example <each> --locked`; all exit 0 on the base.
- [ ] **Step 2:** Write the workflow. Validate it with
  `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/production.yml'))"`. Run jobs 1–4
  locally with the stable toolchain and record their exit codes; job 6 runs in Task 8's handoff.
- [ ] **Step 3:** Node report listing which jobs were run locally and which can only run on GitHub.

