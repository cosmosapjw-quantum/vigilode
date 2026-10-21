# Task 6 (ci-docs) report

Branch production/ci-docs, base 997fefe, commit 93399d6 (pushed).

## Requirements
- `.github/workflows/production.yml`: triggers (pull_request, push to main, the listed paths) at the top; jobs fmt, clippy, test, docs, semver, bench. Toolchain pinned 1.94.1 via `RUST_TOOLCHAIN` env, which equals rust-toolchain.toml and workspace rust-version, so it is the MSRV check.
- semver: `if: github.event_name == 'pull_request'` (R-C7/B6a), `continue-on-error: true`, `fetch-depth: 0`, cargo-semver-checks pinned at **0.51.0** (`cargo info`: rust-version 1.93, so it installs on 1.94.1).
- test job (seam iii): `cargo test --locked -p vigilode -p rodas5p-core -p rodas5p-krylov -p vigilode-coupled` and `cargo test --locked -p rodas5p-integrators` (all targets, no `--test` list; picks up b1c_staleness_contracts, p0-events, p0-fastdense automatically). Overrides the brief's `--lib` plus list.
- Examples in crates/vigilode/examples: robertson.rs (mass drift < 1e-8, observed 1.3e-15), dense_output.rs (Prothero-Robinson, lambda=-1e3, midpoint interior error <= 2 units, observed 0.33), dae_mass.rs (constraint <= 1e-12, observed 0), banded_fd.rs (Brusselator n=100, FiniteDifference + .banded(2,2); check chosen by me for B6c: agrees with the unbanded run < 1e-4 and uses fewer FD rhs calls, observed 132 vs 2222).
- R-C7: examples use only `Options { .., ..Default::default() }`, no match on facade enums, no other struct literals; they read `is_success()`, `final_state()`, `at()`, `t`, `stats.finite_difference.rhs_calls`.
- No src/ file edited.

## Evidence
- YAML parses (jobs fmt, clippy, test, docs, semver, bench).
- `cargo build -p vigilode --examples --locked`: exit 0. Each of the 4 examples via `cargo run`: exit 0.
- Local (stable 1.97, since 1.94.1 install failed on this machine): fmt exit 0; docs `RUSTDOCFLAGS=-D warnings cargo doc` exit 0; `cargo test -p vigilode --doc` ok; clippy see deviation. Ledger checker PASS.
- No RED step: examples and CI are not unit-testable code; the examples' asserts are the checks.

## Deviation
- clippy job uses `--no-deps`. Reason: at base, `cargo clippy -p vigilode --all-targets --locked -- -D warnings` fails in rodas5p-core (`too_many_arguments` 10/7 at crates/rodas5p-core/src/damped_moments.rs:492, seen with stable 1.97; the lint is long-standing so likely also on 1.94.1). With `--no-deps` the facade crate and its examples/tests are clean (exit 0). Controller may instead prefer fixing core and dropping the flag.

## Not run locally (GitHub or handoff)
- Actions execution itself, semver job (needs PR base sha), bench job (apt gfortran/libsundials), and the full `cargo test --locked -p rodas5p-integrators` plus the light-crates test run (heavy).
- Handoff commands:
  `cargo test --locked -p vigilode -p rodas5p-core -p rodas5p-krylov -p vigilode-coupled`
  `cargo test --locked -p rodas5p-integrators`
  `cargo test --manifest-path crates/vigilode-bench/Cargo.toml --locked`
  `cargo clippy -p vigilode --all-targets --locked -- -D warnings` (expected red at base, see above)

## Open questions
- Fix rodas5p-core clippy and drop `--no-deps`? Integrators full-package test time on the GitHub runner is unmeasured (timeout 120 min set).
- Pinned toolchain 1.94.1 could not be installed locally; verification used stable 1.97.

## Fix round 1
Fixed M1-M5 and M7 (M6 no change).
- M1: Swatinem/rust-cache@v2.7.8 after each toolchain step; shared-key `root`, and `bench` with workspaces crates/vigilode-bench.
- M2: `OPENBLAS_NUM_THREADS: 1` in the bench job env.
- M3: header comment in production.yml naming vigilode-hybrid, rodas5p-fair-ab, rodas5p-cli and citing a1-inner-tolerance-parity.yml, a1-two-arm-receipt.yml, hybrid-structure-ad-20261020.yml, e4-fresh-clone-build.yml (fair-ab has no test workflow).
- M4: `cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}`.
- M5: new crates/vigilode/README.md; "Examples" section in the lib.rs crate docs (doc-only edit).
- M7: robertson.rs mass check tightened to 1e-12 (observed 1.3e-15) plus component check against reference y(40) (abs 1e-5 + rel 1e-3); observed values within.
Evidence: YAML parses; robertson exits 0; cargo doc -D warnings ok; fmt ok; ledger PASS.
