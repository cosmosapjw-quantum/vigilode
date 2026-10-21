VERDICT: ALL ADDRESSED

### Finding Verdicts

- **M1 no dependency cache** — ADDRESSED. `Swatinem/rust-cache@v2.7.8` (tag-pinned) follows the toolchain step in all six jobs (production.yml diff +47, +64, +82, +102, +122, +144). The five root-workspace jobs use `shared-key: root`. The bench job uses `shared-key: bench` plus `workspaces: crates/vigilode-bench`, so it caches that workspace's own `target/` and Cargo.lock and does not share a key with root. The action is placed after `rustup override set`, so the cache key sees the pinned rustc.
- **M2 OPENBLAS_NUM_THREADS in bench** — ADDRESSED. Job-level `env: OPENBLAS_NUM_THREADS: 1` (production.yml, bench job, diff +136-137).
- **M3 undocumented exclusions** — ADDRESSED. The header comment (production.yml, diff +25-31, below `concurrency`) names vigilode-hybrid, rodas5p-fair-ab and rodas5p-cli and says the exclusion is deliberate. The four workflow files it cites exist in `.github/workflows` (a1-inner-tolerance-parity, a1-two-arm-receipt, hybrid-structure-ad-20261020, e4-fresh-clone-build). I did not check what each one runs.
- **M4 cancel-in-progress on main** — ADDRESSED. production.yml:`cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}`. The expression is valid in a `concurrency` block, since the `github` context is available there. PyYAML loads it as a string, and the runner evaluates it. Main pushes are not cancelled. The condition is on ref, not event, which differs from the suggestion but gives the same result.
- **M5 examples not linked** — ADDRESSED. New `crates/vigilode/README.md:6-15` lists the four `cargo run -p vigilode --example ...` commands. The lib.rs crate docs gain an "Examples" section (lib.rs, diff +231-237). Its intra-doc links `Solution::at` and `Problem::banded` point at items that already appear in the docs. The report says `cargo doc -D warnings` passed. I could not rebuild here.
- **M7 weak Robertson check** — ADDRESSED. robertson.rs now asserts mass drift below 1e-12 (observed 1.3e-15, so about three orders of margin) and compares y(40) with the published reference using `1e-5 + 1e-3*|r|`.

### Requested checks
- YAML parses: yes. PyYAML loads six jobs and the concurrency map.
- rust-cache keys separate root from bench: yes, as described under M1.
- `cancel-in-progress` expression: valid.
- Robertson check across toolchains: not fragile. Mass conservation is a linear invariant and is held to roundoff. The solver tolerance is rtol 1e-6, well inside the 1e-3 relative tolerance of the reference check. No toolchain-dependent behaviour is involved.
- I could not run the example locally. The rustup download of 1.94.1 is blocked by the sandbox proxy. I relied on the report's run output for this.

### New Breakage in the Fix Diff
- Minor (robertson.rs, reference loop): for y1, the tolerance `1e-5 + 1e-3*9.2e-6` is larger than the reference value 9.19e-6. That component check therefore accepts anything from 0 to about 1.9e-5 and cannot catch an error in y1. The y0 and y2 checks do bind. This does not block the task.
- No other new defects.

### Out-of-Scope Observations
None.
