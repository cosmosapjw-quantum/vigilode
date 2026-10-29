# Task 6 (ci-docs) review, 997fefe..93399d6

### Spec Compliance

- ✅ Spec compliant, with the controller's three rulings applied (per-package tests, clippy `--no-deps`, informational semver) and R-C7, B6a and FP6 honoured.
- ⚠️ Cannot verify from the diff or locally:
  - Whether Ubuntu's `libsundials-dev` provides the SUNDIALS version the bench bridge expects (`crates/vigilode-bench/build.rs:124-134` links `sundials_cvode`, `sundials_nvecserial`, and others). This can only be checked on a runner (Task 8 handoff).
  - Whether `cargo install --locked --version 0.51.0 cargo-semver-checks` works on 1.94.1. The report cites `cargo info` rust-version 1.93. The job is informational, so a failure does not block.
  - Wall time of `cargo test -p rodas5p-integrators` in the debug profile. Timeout is 120 min.

### Strengths
- Toolchain: `RUST_TOOLCHAIN: 1.94.1` (production.yml:56) is installed and `rustup override set` in every job. It matches rust-toolchain.toml and the workspace `rust-version`, so each job is an MSRV check.
- Triggers: `pull_request` plus `push` to main, on the five required paths (:30-45). `permissions: contents: read` and no secrets anywhere.
- `--locked` is on clippy, test (both steps), the docs job (all four commands), and bench. The `semver` install line uses `--locked` for the install, as the brief asks. fmt takes no `--locked`, which is correct.
- Action versions: `actions/checkout@v4` only, a tag as other workflows in the repo use.
- semver: guarded with `if: github.event_name == 'pull_request'` (:132), `fetch-depth: 0`, `continue-on-error: true`, version pinned 0.51.0.
- bench builds the separate workspace via `--manifest-path crates/vigilode-bench/Cargo.toml --locked` (Cargo.lock exists there). It has the apt step `gfortran libsundials-dev` (:163-166). The build also needs gcc and `nm`, which ubuntu-latest has.
- Coverage of the root workspace: the test job names the facade, core, krylov, coupled and integrators. Those are the production crates among the 7 members.
- Examples:
  - R-C7 holds: only `Options { .., ..Default::default() }`, no match on facade enums, no other struct literals. They read `is_success()`, `final_state()`, `at()`, `sol.t`, and `stats.finite_difference.rhs_calls`.
  - Assertions are real. `dae_mass` asserts the algebraic constraint, which would fail if the mass matrix were ignored. `dense_output` measures the midpoint error of every accepted step in tolerance units against the exact `sin t`. `banded_fd` compares the banded and unbanded solutions and the FD rhs-call counts.
  - Facade `Options` is not `#[non_exhaustive]` today.
- Build: `cargo build -p vigilode --examples --locked -j 1` with stable finishes with exit 0. The only warnings come from rodas5p-integrators (not this diff).

### Issues

#### Critical (Must Fix)
None.

#### Important (Should Fix)
None.

#### Minor (Nice to Have)
1. `production.yml` (all jobs): there is no dependency cache. Each of the six jobs builds the faer-heavy dependency tree cold. The brief did not require it, but the 45/90/120-minute timeouts suggest this cost. Add `Swatinem/rust-cache@v2`, or `actions/cache` on `~/.cargo` and `target`, keyed on Cargo.lock. Pin whichever one to a tag.
2. `production.yml:154-168`: the bench job sets no `OPENBLAS_NUM_THREADS`. It only runs `cargo test` (the sources, adapters and report tests), so nothing timing-sensitive depends on it. If the job ever runs the work-precision harness (`src/main.rs`), set `OPENBLAS_NUM_THREADS: '1'`. Other workflows in the repo, such as coupled-native, do.
3. Coverage note missing: `crates/vigilode-hybrid` is its own workspace (`Cargo.toml:32`), and `rodas5p-fair-ab` and `rodas5p-cli` are root members that no job tests. No job covers them, and no comment says whether that is deliberate. Add a short comment in the test job (or the header) stating the exclusion, or add `cargo test --manifest-path crates/vigilode-hybrid/Cargo.toml --locked` (default feature `native` only).
4. `production.yml:50-52`: `cancel-in-progress: true` with group `production-${{ github.ref }}` also cancels an earlier push run on `main` when a newer push arrives. Consider `cancel-in-progress: ${{ github.event_name == 'pull_request' }}` so each main commit keeps its result.
5. Docs: neither `crates/vigilode/README.md`, `crates/vigilode/src/lib.rs`, nor the root README links the four examples. The root README mentions only the `rodas5p-integrators` `solve_stiff` example (README.md:56-83). Add a "Examples" line, such as `cargo run -p vigilode --example robertson`, to the lib.rs crate docs.
6. Latent T7 risk (not a defect now): the R-C7 pattern `Options { .., ..Default::default() }` stops compiling from an example if T7 marks `Options` (an input struct) `#[non_exhaustive]`. Preflight A15 limits T7 to output types, so this holds today. If T7's scope widens to `Options`, the examples need a builder or setter instead.
7. `robertson.rs:406-407`: total mass is a linear invariant, which Rosenbrock methods preserve to roundoff. The 1e-8 check is therefore weak (the report observed 1.3e-15) and would not catch a moderate accuracy regression. It is the brief's mandated check, so this is an observation only.

### Assessment

**Task quality:** Approved

**Reasoning:** The workflow matches the brief and rulings: pinned 1.94.1, `--locked` everywhere, the PR-only semver guard, and a correct bench job with its apt step. The four examples compile, follow R-C7, and assert real properties. Remaining items are minor: no caching, no note on the excluded `vigilode-hybrid`, `rodas5p-fair-ab` and `rodas5p-cli`, and no docs links to the examples.
