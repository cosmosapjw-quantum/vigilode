# Preregistration: matched-accuracy polynomial phi campaign (R3 POLY-03)

Written and committed before the first timed session. Results are appended below the line at the end after the run.

## Question

On a fixed corpus of symmetric nonpositive operators, is a joint Chebyshev or Laguerre phi action faster than the
Arnoldi (Krylov) fused phi action? All arms see identical inputs and produce verified outputs. The study separates
coefficient setup (cold) from frozen-operator reuse (warm).

## Source and commands

- Source commit: `86750eb822df144590bab4b2438e9e1b71486a25` (branch `claude/jolly-wozniak-7wl15h-wu23-reaudit-r3`),
  rustc 1.94.1.
- Build: `cargo build --release --locked -p rodas5p-cli`
- Untimed verification:
  `target/release/rodas5p r3-campaign-verify --study poly03 --output research/r3_matched_accuracy_poly03_20261001/VERIFY.json`
- Paired timing:
  `target/release/rodas5p r3-campaign --study poly03 --sessions 6 --seed 20261004 --output research/r3_matched_accuracy_poly03_20261001/CAMPAIGN.json`
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

Every case uses `h = 1`, so `w_k = b_k` exactly and every arm receives the same five vectors. The action is
`sum_k phi_k(A) w_k`.

| case | operator | enclosure |
|---|---|---|
| `diagonal24-rho10` | diagonal, spectrum from -0.1 to -10 | Gershgorin |
| `diagonal24-rho100` | diagonal, spectrum from -0.1 to -100 | Gershgorin |
| `laplacian64-rho100` | Laplacian, n = 64, rho = 100 | Gershgorin |
| `laplacian128-rho400` | Laplacian, n = 128, rho = 400 | Gershgorin |
| `gram32` | `-(M M^T)` with n = 32 | declared `[-||M||_F^2, 0]` |

The reference arm `arnoldi-fused-phi` is `fused_phi_action` on the dense operator (relative tolerance 1e-12, dimension
up to 64). The candidate arms `chebyshev-cold`, `chebyshev-warm`, `laguerre-cold` and `laguerre-warm` use
`joint_phi_action_unbounded` with a truncation budget of 1e-12. A cold arm computes its coefficients on every call. A warm
arm reuses them through a coefficient cache keyed to the operator. The timed arms compute no rounding enclosures. The
certified bounds are separate POLY-02 results.

## Gate

**PASS** requires both of the following. Otherwise the verdict is **FAIL**.

1. Every arm succeeds on all 5 cases. The error relative to the dense augmented exponential, with the norm floored
   at 1, must be at most 1e-9. There may be no unsupported or failed input, and such inputs stay in the denominator.
2. At least one polynomial arm has a verified paired decision of `Promote` against the Arnoldi reference.

The FAIL status is `SPEEDUP_UNPROVEN`. A ratio of polynomial degrees, such as 245/56, is never reported as a speedup.

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

The untimed verification ran once at the source commit, into a scratch directory:

- Every arm passed the accuracy gate on every case. The relative errors were 1.1e-15 to 2.2e-13.
- The peak RSS was 9.6 to 9.9 MiB per arm process.

No timing was measured. Nothing was changed after the pilot. The seed 20261004 was not used before this commit.

## Addendum before the rerun (committed before any rerun output)

### What happened at `86750eb` / `380f565`

The first execution of the commands above completed the untimed verification. The timed campaign then aborted in the
shared session harness, before any receipt was written:

- The first arm, `chebyshev-cold`, aborted with `INVALID_RAW_TIMING_PROTOCOL` on `diagonal24-rho10#aa` (batch 4
  against a calibration of 5).
- No decision was computed or seen.

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

---

## Results (appended after the run)

Rerun on 2026-10-01 (KST), 00:19–00:35 UTC, at source commit `0799a05e6c616084af4d58671ee4c6c93e2c5a1c` (code `d198558`). The outputs are `VERIFY.json` (with `VERIFY.arms/`), `CAMPAIGN.json` and `CAMPAIGN.<arm>.sessions/`.

**Verdict: PASS** for the preregistered gate. Every arm is verified on every case, and `chebyshev-warm` has a verified decision of `Promote` against the Arnoldi reference.

| arm | verified decision | speedup point | 95% interval | sessions | failures |
|---|---|---:|---|---:|---:|
| `chebyshev-cold` | Inconclusive | 0.565 | [0.018, 1.989] | 6 | 0/0 |
| `chebyshev-warm` | Promote | 3.166 | [1.167, 13.414] | 6 | 0/0 |
| `laguerre-cold` | Block | 0.122 | [0.001, 0.317] | 6 | 0/0 |
| `laguerre-warm` | Inconclusive | 1.180 | [0.196, 3.119] | 6 | 0/0 |

Untimed verification, as the largest relative error over the 5 cases, and peak RSS:

- `arnoldi-fused-phi`: 2.16e-13, 10324 KiB, all within 1e-9: True
- `chebyshev-cold`: 1.96e-14, 10724 KiB, all within 1e-9: True
- `chebyshev-warm`: 1.96e-14, 9980 KiB, all within 1e-9: True
- `laguerre-cold`: 1.22e-14, 10256 KiB, all within 1e-9: True
- `laguerre-warm`: 1.22e-14, 10632 KiB, all within 1e-9: True

Only the warm Chebyshev arm promotes. It reuses the coefficient table across repeated calls with the same operator,
enclosure and `h`. With coefficient setup on every call (cold), Chebyshev is Inconclusive and Laguerre is Blocked.
The intervals are wide, for example [1.17, 13.4] for the warm arm, because the five cases differ strongly.

This PASS is limited to the measured population: these five dense symmetric operators, `h = 1`, a frozen operator
and this host. It is under `STATISTICAL_AUTHORITY_HOLD` from the R3 coverage studies (`L-0007`, `L-0010`). It is not
a production speed claim. No ratio of polynomial degrees is reported as a speedup.
