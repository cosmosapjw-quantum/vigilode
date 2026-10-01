# Preregistration: matched-accuracy transactional q1/q2 campaign (R3 HOM-06)

Written and committed before the first timed session. Results are appended below the line at the end after the run.

## Question

On a fixed corpus, is the complete certified transactional attempt faster than the serial sequential RODAS5P baseline
at the same output grid and tolerances? The attempt means an adaptive integration that includes the q=1 and q=2 paths,
the native-target certificate (witness construction included), rejected attempts and the sequential fallback. It is run
with 1, 2, 4 and 8 threads.

## Source and commands

- Source commit: `86750eb822df144590bab4b2438e9e1b71486a25` (branch `claude/jolly-wozniak-7wl15h-wu23-reaudit-r3`),
  rustc 1.94.1.
- Build: `cargo build --release --locked -p rodas5p-cli`
- Untimed verification (accuracy, work, lane fractions, peak RSS, one process per arm):
  `target/release/rodas5p r3-campaign-verify --study hom06 --output research/r3_matched_accuracy_hom06_20261001/VERIFY.json`
- Paired timing (all four candidate arms, raw session records kept next to the output):
  `target/release/rodas5p r3-campaign --study hom06 --sessions 6 --seed 20261003 --output research/r3_matched_accuracy_hom06_20261001/CAMPAIGN.json`
- Input hashes (sha256 at the source commit):
  - `crates/rodas5p-cli/src/r3_campaigns.rs` `0071d0edc6bc2d74f66be3f6326120e690be87597e092f33c397bf606ae663c7`
  - `crates/rodas5p-cli/src/main.rs` `cb06e5753ac8363c7d9048372ddad5049b4cd55bfc409ab8ef08052bb936cd5c`
  - `crates/rodas5p-core/src/polynomial_action.rs` `8ec01b70e3e923cb6d4510f95a2d7b2a32bc1309fcf7037e73dc695202bcf495`
  - `crates/rodas5p-integrators/src/transactional_q1_q2.rs` `66c3a8e085300521c05f0fc2139f002b285d2ef906ab57ec3dfc35bef9dae485`
  - `crates/rodas5p-integrators/src/integrate.rs` `aee8dfa8005c038b3b9cd0c92934522720aadfb83d07782af59f5a22e8b73313`
  - `crates/rodas5p-integrators/src/exponential.rs` `da84a475c085d76d1275fea88b8099c807754a38a93afa1e2d02848c69df7f80`
  - `crates/rodas5p-fair-ab/src/paired_timing.rs` `090b03533148a300f49b86105c71b311d29d718e9fc52ff82781edd7457d17f5`
  - `crates/rodas5p-fair-ab/src/paired_receipt.rs` `26199ea144e1b5a4d8cadba31928493c44b3d5f8721475877ea0179730299ab5`
  - `Cargo.lock` `04c7d6c147bf19ebb1a627463705d9dcf27337a3ff228dd1169f19eb73c71400`

## Corpus and arms

The corpus has five autonomous problems `f(z) = A z + q z^2`. All run on the span [0, 1] with outputs every 0.1, at
atol 1e-8 and rtol 1e-6.

| case | problem |
|---|---|
| `scalar-linear` | `A = -20`, `q = 0` |
| `scalar-quadratic` | `A = -20`, `q = -2` |
| `diagonal-quadratic-8` | `A = diag(-10^(3i/7))`, `q = -0.5` |
| `coupled-linear-6` | symmetric tridiagonal, -100 on the diagonal and 50 off it |
| `nonsymmetric-linear-2` | `[[-1000, 999], [0, -1]]` |

The reference arm `sequential-jf-gmres` is the matrix-free sequential RODAS5P with GMRES (rtol 1e-10). The candidate
arms `transactional-certified-p{1,2,4,8}` are the transactional q1/q2 path with `Q2Admission::NativeTargetCertificate`
and 1, 2, 4 or 8 threads. The thread pool is built per step, as in the current code, and its cost is part of the arm.

## Gate

**PASS** requires all four of the following. Otherwise the verdict is **FAIL**.

1. Every arm, reference included, succeeds on all 5 cases. Each run's largest tolerance ratio
   `|y - y_ref| / (atol + rtol |y_ref|)` on the grid must be at most 10, against a sequential run at 1e-13 / 1e-11.
2. There is no admission mismatch: every accepted q2-escalated step was admitted by a certificate.
3. There is no work-accuracy regression: in every case, each candidate's ratio is at most twice the reference's ratio
   plus 0.1.
4. At least one candidate arm has a verified paired decision of `Promote` against the reference. That needs a receipt
   with 6 sessions, all cases present and a passing A/A control.

The FAIL statuses are `SPEEDUP_UNPROVEN` when only item 4 fails, and `ADMISSION_EQUIVALENCE_NOT_ESTABLISHED` or an
accuracy failure otherwise.

The report includes these quantities, which feed no gate:

- the q1 acceptance fraction p1 and the fallback fraction pf;
- the certificate fraction and the certificate operations;
- W batches and critical-path depth;
- vector work (WorkCounters) and peak RSS per arm;
- the paired speedup intervals.

## Host and authority

The host is a shared cloud container with 4 vCPUs (Intel Xeon @ 2.80GHz). It is not an authoritative timing host.
The paired protocol is `PairedTimingProtocol::authoritative(seed)`:

- 30 pairs per session and case, in the seeded ABBA order;
- 10000 resamples, a 95% interval and a required speedup of 1.15;
- the Monte-Carlo gate at delta = 0.01.

There are 6 independent session processes, each with an A/A control. The protocol is fixed and is not extended after the
data are seen. The corpus has 5 cases, which is in the region where the R3 coverage study (`L-0007`) found conservative
coverage. That study's preregistered gate failed, however (`STATISTICAL_AUTHORITY_HOLD`), so no decision here is a
production speed claim, whatever its value. A numeric PASS or FAIL below refers only to the gate stated in this file.

## Disclosure of a pilot run

The untimed verification ran once at the source commit, into a scratch directory, to check the pipeline:

- Every arm passed the accuracy gate on every case, with tolerance ratios 7.9e-6 to 5.9e-2.
- There were 0 admission mismatches.
- `coupled-linear-6` fell back on every attempt, so its certificate fraction was 0.
- `diagonal-quadratic-8` was admitted by a certificate on every attempt, with p1 = 0.

No timing was measured. Nothing was changed after the pilot. The seed 20261003 was not used before this commit.

## Addendum before the rerun (committed before any rerun output)

### What happened at `86750eb` / `380f565`

The first execution of the commands above completed the untimed verification. The timed campaign then aborted in the
shared session harness, before any receipt was written:

- `transactional-certified-p1` finished. Its unverified assessment decision, `Block`, was printed to the log. This is
  disclosed, and it is not a result.
- `transactional-certified-p2` aborted with `INVALID_RAW_TIMING_PROTOCOL` on `scalar-linear#aa` (batch 2 against a
  calibration of 3).

The cause was a STAT-DEV-02 harness defect: the A/A control was given the candidate case's batch, while it records its
own warmups. It is fixed in `c8aa0dc` and has a regression test. All outputs of that execution are kept unchanged in
`aborted-run-380f565/`, including the verification and the raw session records of the sessions that ran.

### Changes made before the rerun

An independent review of the code then found defects, which are fixed in `d198558`:

- HOM-05: the certificate model must also reproduce the ODE at every candidate stage state.
- HOM-06 gate item 2 is now counted per step (`certificate_mismatches`). The earlier count difference could not fail.
- POLY-03: the Arnoldi operator is built outside the timed call, like the candidate's.
- `r3-campaign` headlines the verified decision and records a failing arm instead of aborting.

The corpora, arms, tolerances, accuracy gates, speed gate and seeds above are unchanged. The rerun repeats both
commands at source commit `d198558b981a538b7f8293f8b059b9f2f515d080`, where the input hashes are:
  - `crates/rodas5p-cli/src/r3_campaigns.rs` `8dd1c5e3a06aaad5c4e812823682caf45571d66689ed2db1f97d8d718a185b95`
  - `crates/rodas5p-cli/src/main.rs` `12d9979405c2a0058df6eb3911e97e99d3cdaf0319e21cd4b583914449f4ec3c`
  - `crates/rodas5p-core/src/polynomial_action.rs` `5e79dc83462ab668ad61df33f6ece1202857afd21dbf80af165be7e321691f54`
  - `crates/rodas5p-integrators/src/transactional_q1_q2.rs` `ccd8d6b7dd4cbbb12e34c5d9d19f59297bd11785cfd5c7654796a8e3d744acaa`
  - `crates/rodas5p-integrators/src/integrate.rs` `aee8dfa8005c038b3b9cd0c92934522720aadfb83d07782af59f5a22e8b73313`
  - `crates/rodas5p-integrators/src/exponential.rs` `da84a475c085d76d1275fea88b8099c807754a38a93afa1e2d02848c69df7f80`
  - `crates/rodas5p-fair-ab/src/paired_timing.rs` `2edd38248c5d8bd79fa308c2b62fe4142154313d5efbaa753b7d3f3f1d0ce614`
  - `crates/rodas5p-fair-ab/src/paired_receipt.rs` `65466446e4c2b3f39a7f9272ac63e186ae0c80f51a7ea5560510a1532f43f106`
  - `Cargo.lock` `04c7d6c147bf19ebb1a627463705d9dcf27337a3ff228dd1169f19eb73c71400`

The label "certified" in the HOM-06 arm names refers to the q=2 admission only. q=1 fast accepts still pass the
operational gate, and the report gives their fraction as p1.

---

## Results (appended after the run)
