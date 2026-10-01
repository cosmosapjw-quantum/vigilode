# Preregistration: corrected coverage study of the paired timing decision (R3 STAT-DEV-04, v2)

Written and committed before the run. Results are appended below the line at the end after the run.

## Why a second study

The first study (`research/r3_timing_coverage_study_20261001/`, ledger `L-0007`, FAIL) let the simulated data and
the bootstrap share one random stream. Its seeds also depended on a scenario's grid position, and its thresholds were
rounded leniently. This study repeats the same question with these defects fixed (`d198558`). It does not supersede or
relabel `L-0007`.

## Question

The question is unchanged. Under the declared design (`docs/TIMING_DESIGN_CONTRACT.md`) and the same dependence and
missingness models, does the interval cover the corpus estimand at 95%? Does the gated decision promote at most at the
nominal 2.5% rate when the true speedup is at or below 1.15?

## Source and command

- Source commit: `d198558b981a538b7f8293f8b059b9f2f515d080` (branch `claude/jolly-wozniak-7wl15h-wu23-reaudit-r3`),
  rustc 1.94.1.
- Build: `cargo build --release --locked -p rodas5p-cli`
- Command:
  `target/release/rodas5p paired-timing-coverage-study --replications 2000 --seed 20261005 --threads 4 --output research/r3_timing_coverage_study_v2_20261001/COVERAGE_STUDY.json`
- Input hashes (sha256 at the source commit):
  - `crates/rodas5p-fair-ab/src/timing_design.rs`
    `312301ef4c358631e46f658280a09e80f7350e7a9eb5de2e8b4c629aff05f041`
  - `crates/rodas5p-fair-ab/src/paired_timing.rs`
    `2edd38248c5d8bd79fa308c2b62fe4142154313d5efbaa753b7d3f3f1d0ce614`
  - `crates/rodas5p-cli/src/main.rs`
    `12d9979405c2a0058df6eb3911e97e99d3cdaf0319e21cd4b583914449f4ec3c`
  - `Cargo.lock` `04c7d6c147bf19ebb1a627463705d9dcf27337a3ff228dd1169f19eb73c71400`

## Design

The design is the same as v1:

- 48 scenarios: sessions {6, 12}, cases {1, 5}, missingness {none, MCAR 0.1, MCAR 0.3, MNAR 0.3} and
  `theta = ln` {1, 1.15, 1.3};
- the model `u 0.05, v 0.10, w 0.03, e 0.05`, with AR(1) 0.5 within a cell;
- `PairedTimingProtocol::authoritative`, with the Monte-Carlo gate at delta = 0.01.

Two things change. Each replication now draws its data and its bootstrap from separate seeds, keyed by scenario id and
replication index. The thresholds are rounded strictly.

## Gate

- Coverage must be at least `0.93745` in every primary scenario (the 36 without MNAR).
- When `theta <= ln 1.15`, the promote rate must be at most `0.03399`.

**PASS** if every primary scenario meets both thresholds. Otherwise the verdict is **FAIL**
(`STATISTICAL_AUTHORITY_HOLD`). MNAR is reported but does not affect the verdict. The rule is applied as stated, with no
retesting or reseeding.

## Holdout and prior information

This is a simulation without a data holdout. The seed 20261005 was not used before this commit. The v1 results are
known, so this study is a correction of v1, not a blind replication. Neither the design nor the gate was changed in
response to them.

---

## Results (appended after the run)
