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

---

## Results (appended after the run)
