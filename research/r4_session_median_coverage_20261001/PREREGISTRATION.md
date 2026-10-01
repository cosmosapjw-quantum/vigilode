# Preregistration: coverage and power of the exact session-median interval (R4-STAT-DEV-04)

Written and committed before the run. Results are appended below the line at the end after the run.

## Question

The exact simultaneous session-median interval (`rodas5p_fair_ab::exact_session_median_interval`, estimand
`SESSION_CELL_MEDIAN_ESTIMAND`) has a finite-sample coverage lower bound `1 - C q(S, k)` under iid session vectors,
complete fixed case sets and a fixed number of sessions. Does the implementation reach that bound in simulation over
the declared domain, including laws with no case spread, identical cases, strong dependence between cases, atoms and
heavy tails? How often does it promote at `theta = ln 1.15` and at `theta = ln 1.3`?

## Source and command

- Source commit: `96639dbf9b6be7c288028534570674d4decfcfcb` (branch `claude/jolly-wozniak-7wl15h-wu24-reaudit-r4`), rustc 1.94.1.
- Build: `cargo build --release --locked -p rodas5p-cli`
- Command:
  `target/release/rodas5p session-median-coverage-study --replications 10000 --seed 20261013 --threads 4 --output research/r4_session_median_coverage_20261001/COVERAGE_STUDY.json`
- Inputs: `crates/rodas5p-fair-ab/src/session_median.rs`, `crates/rodas5p-fair-ab/src/session_median_study.rs`,
  `crates/rodas5p-cli/src/main.rs`, `Cargo.lock` (hashes in the ledger row).

## Design

`session_median_study_grid()`: sessions S in {6, 8, 12, 24}, cases C in {1, 5}, `theta = ln` {1.15, 1.3}, and seven
laws: six in domain (`additive`, `case-sd-zero`, `identical-cases`, `shared-session-effect`, `atoms`, `heavy-tails`)
and one out of domain (`drift-out-of-domain`, a linear drift across sessions that breaks the iid premise). That is 96
in-domain and 16 out-of-domain scenarios. Each session cell is the median of 15 pair errors plus the session effect,
a symmetric case offset and `theta`, so the target is `theta` in every in-domain law. Alpha is 1/20 and the required
speedup is 1.15. Seeds are FNV-keyed by scenario id and replication index; data and decision use one stream per
replication (the interval has no resampling).

## Gate

Each in-domain scenario passes if its empirical coverage is at least `design_coverage_lower - tolerance`. Here
`tolerance = sqrt(ln(2 / delta) / (2 * 10000))`, with `delta = 0.01 / 96`, which is Hoeffding with Bonferroni over
the in-domain scenarios. Unbounded intervals count as covering.

One-case scenarios with a finite design and a continuous law (every in-domain law except `atoms`) must also pass an
exact check. The per-case bound is exact there, so the miss rate must equal `q(S, k)` within the same tolerance in
both directions. An over-conservative implementation therefore fails as well.

Most of this gate tests the implementation of a theorem: `1 - C q` holds for iid sessions. The 12 in-domain scenarios
with `S = 6, C = 5` are unbounded by design (`q(6, 1) = 1/32 > 1/100`), so they pass trivially and are listed as such.

**PASS** if every in-domain scenario passes. Otherwise the verdict is **FAIL**. The out-of-domain scenarios and the
promote rates (power) are reported and do not enter the verdict. A PASS does not make timing admissible: the R4
authority gate requires an independent domain review first. The registry will record the study as a hold awaiting
that review. The rule is applied as stated, with no reseeding.

## Prior information

Seed 20261013 is unused. No run of this grid exists before this commit. Only unit contracts on fixed inputs were run.

---

## Results (appended after the run at `d700572`)

Output: `COVERAGE_STUDY.json`, 112 scenarios at 10000 replications each. The run took 15 s with 4 threads. Tolerance
is 0.0222.

**Verdict: PASS.** None of the 96 in-domain scenarios failed.

- **Coverage against the design bound.** Coverage minus the design bound was at least -0.0023 in every in-domain
  scenario.
- **Exact checks.** All 40 one-case exact checks passed. The largest deviation of the miss rate from `q(S, k)` was
  0.0040.
- **By design:**

| Design | k | Coverage | False-promote rate at theta = ln 1.15 | Power at theta = ln 1.3 |
|---|---|---|---|---|
| S6 C1 | 1 | 0.9665 to 0.9727 (bound 31/32) | at most 0.017 | 0.08 to 0.86 |
| S6 C5 | unbounded | 1 (as designed) | 0 | 0 |
| S8 C5 | 1 | 0.9908 to 1.0 (bound 123/128) | at most 0.004 | 0.03 to 0.83 |
| S12 C1 | — | 0.9591 to 0.9660 | at most 0.022 | 0.16 to 0.999 |
| S24 C1 | — | 0.976 to 0.980 | at most 0.013 | 0.23 to 1.0 |
| S24 C5 | — | 0.992 to 1.0 | at most 0.005 | 0.11 to 1.0 |

- **Power by law.** Within each design, power varies with the law. Heavy tails and the strong shared session effect
  are the lowest; atoms and the additive law are the highest.

**Out of domain.** The drift law, which breaks the iid premise, failed 14 of its 16 scenarios. Its coverage fell to
between 0 and 0.92 as the session count grew. This is the expected failure of the premise and does not enter the
verdict.

**Scope.** The study checks the implementation against its theorem. It does not establish the iid-session premise on
any host. Under the R4 authority gate, the session-cell estimand is recorded in the study registry as **hold pending an
independent domain review**, not as admissible. No timing decision is admissible on this branch.
