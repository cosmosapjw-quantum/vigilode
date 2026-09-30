# Preregistration: coverage study of the paired timing decision (R3 STAT-DEV-04)

Written and committed before the confirmatory run. Results are appended below the line
at the end after the run and are not written beforehand.

## Question

Under the declared design (`docs/TIMING_DESIGN_CONTRACT.md`) and the declared dependence
and missingness models, does the paired timing interval cover the corpus estimand at the
nominal 95% level, and does the gated decision promote at most at the nominal 2.5% rate
when the true speedup is at or below the required 1.15?

## Source and command

- Source commit: `55eeb9d086a6c58176757b95302f27670f283863` (branch
  `claude/jolly-wozniak-7wl15h-wu23-reaudit-r3`), rustc 1.94.1.
- Build: `cargo build --release --locked -p rodas5p-cli`
- Command:
  `target/release/rodas5p paired-timing-coverage-study --replications 2000 --seed 20261002 --threads 4 --output research/r3_timing_coverage_study_20261001/COVERAGE_STUDY.json`
- Input hashes (sha256 at the source commit):
  - `crates/rodas5p-fair-ab/src/timing_design.rs`
    `2cb10e6073205ef499fb75895741639f9e79ba856caf892de33bbd4e33b106bc`
  - `crates/rodas5p-fair-ab/src/paired_timing.rs`
    `090b03533148a300f49b86105c71b311d29d718e9fc52ff82781edd7457d17f5`
  - `crates/rodas5p-cli/src/main.rs`
    `aca4b4dc0bd056fe104cf9f824e91b82dfdc4494286f86237789f10ca01f2abc`
  - `Cargo.lock` `04c7d6c147bf19ebb1a627463705d9dcf27337a3ff228dd1169f19eb73c71400`

The report does not depend on `--threads`. This is a contract test.

## Design of the study

- Protocol: `PairedTimingProtocol::authoritative(seed)`. That is 30 pairs per cell,
  10000 resamples, 95% confidence, required speedup 1.15, and the Monte-Carlo gate at
  `delta = 0.01`. Each replication has its own derived seed.
- Model: `Y = theta + v_c + u_s + w_sc + e_scp`, with standard deviations
  `u 0.05, v 0.10, w 0.03, e 0.05`. The pair noise `e` is AR(1) with coefficient 0.5
  within a cell. `v_c` is median-centred, so the corpus estimand is exactly `theta`.
- Grid: 48 scenarios, the product of these factors:
  - sessions {6, 12}
  - cases {1, 5}
  - missingness {none, MCAR 0.1, MCAR 0.3, MNAR 0.3}
  - `theta = ln` {1, 1.15, 1.3}

  MNAR drops a cell whose `u_s + w_sc < 0` with probability 0.3.
- Primary scenarios: the 36 scenarios without MNAR. The 12 MNAR scenarios are the
  sensitivity analysis.
- A replication with no admissible case counts as not covered and not promoted.

## Gate

The per-scenario thresholds are 99% binomial allowances for 2000 replications:

- coverage >= `0.9374` (= 0.95 - 2.576 sqrt(0.95 * 0.05 / 2000));
- when `theta <= ln 1.15`, promote rate <= `0.0340` (= 0.025 + 2.576 sqrt(0.025 * 0.975 / 2000)).

**PASS** if every primary scenario meets both of its thresholds. Otherwise the verdict is
**FAIL** (`STATISTICAL_AUTHORITY_HOLD`). A FAIL means that no paired timing decision
under a failing design is authoritative until the design or the interval is changed and
a new preregistered study passes. The MNAR results are reported with their own pass
flags. They do not change the verdict.

Multiplicity: 36 scenarios are checked at the 99% level each. Even with exactly nominal
behaviour, a few scenarios may fail by chance. The rule is kept as stated, with no
retesting or reseeding, and the margins are reported.

## Holdout

This study is a simulation, so it has no data holdout. The confirmatory seed `20261002`
was not used before this commit.

## Disclosure of a pilot run

Before this preregistration, the harness was smoke-timed with seed 20261001 and 40
replications on two scenarios:

- `s12-c5-mcar0.1-theta1.15`: 40/40 covered, 0 promoted.
- `s6-c1-none-theta1.15`: 34/40 covered, 1 promoted, 2 decisions withheld by the gate.

Nothing in the design, the models or the thresholds was changed after the pilot. The
thresholds are the ones stated in the R3 re-audit (`research/adversarial_reaudit_20261001_r3`).

---

## Results (appended after the run)
