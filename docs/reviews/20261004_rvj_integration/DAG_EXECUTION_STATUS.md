# RVJ development DAG — execution status (2026-10-04)

Plan: `research/rvj_integration_20261004/NEXT_DEVELOPMENT_DAG.json` (21 nodes;
PP01, PP02 and SAFE-CHART were already complete when execution started at
`0153540`). That file is a hashed output of L-0063 and is not edited; this
document records what happened to each node. Every executed node was
preregistered and pushed before its code or run, has its results appended
below its preregistration, a ledger row, and passes
`tools/check-research-node.py`. Default solver paths are unchanged
everywhere; nothing here is a speed claim (timing authority stays HOLD).

## Node table

| DAG node | Research node | Ledger | Verdict | One line |
|---|---|---|---|---|
| SAFE-ENCLOSURE | `safe_enclosure_composition_20261004` | L-0064 | **FAIL** | Repaired three directed compositions; probes found real local under-coverage in all three base forms (chart decay 2/20,200, stepped time direction 676/20,000, midpoint radius 2,660/20,000) and a false floor in REV-02's `exp_interval` |
| (follow-up) | `safe_enclosure_exp_floor_20261004` | L-0065 | PASS | `exp_interval` floor `[0, 2^-1020]` was false on (-707.0234, -707); now `2^-1019`; zero violations on 40,511 points |
| SAFE-RECYCLE | `safe_recycle_policy_20261004` | L-0066 | PASS | Explicit GCRO-DR policy in the U-form driver (legacy bit-identical); refresh or cold removes the driver-level failures (2,453 and 102) on the Brusselator sets |
| PP04 | `pp04_shared_shift_policy_20261004` | L-0067 | PASS | Four methods certified by the same residual; selector right in 87/90; jet has a positive counted-flop margin in 49/90 |
| PP03 | `pp03_rhs_compression_20261004` | L-0068 | PASS | QR-compressed jet certified against the original columns; solves drop to `k(d+1)` |
| STORAGE-RUNTIME | `storage_runtime_20261004` | L-0069 | **FAIL** | Refusal made 2 allocator events (message string), registered limit 1; explicit-slot bound needs header slack at small n |
| PP16 | `pp16_complex_shift_gain_20261004` | L-0070 | PASS | Complex-shift certificate with gain `|γ|/Re γ`; gain 1 would under-bound in 16/80 random shifts |
| PP08 | `pp08_laguerre_router_20261004` | L-0071 | PASS | Router works; Laguerre chosen in 0/79 (never cheaper when both admit) |
| PP05 | `pp05_fourier_client_20261004` | L-0072 | **FAIL** | Native certified Fourier client encloses everywhere; parity margin 1.0112 > 1.01 and the alias control was accurate (accepted) |
| PP06 | `pp06_fourier_fft_candidate_20261004` | L-0073 | **FAIL** | FFT right side equals direct to 5e-16; alias control again accurate (accepted) |
| PP07 | `pp07_fourier_shared_action_20261004` | L-0074 | PASS (abstain) | The actual client performs no shifted solve; witness fails at every step start; shared action not connected |
| PP12 | `pp12_lognorm_decay_20261004` | L-0075 | **FAIL** | Verified log-norm (interval Cholesky) fixes the diffusion decay but not convection-dominated cases (3/6 F2) |
| (follow-up) | `pp12b_chain_symmetrizer_20261004` | L-0076 | **FAIL** | Chain symmetrizer: all six REV-02 F2 cases and 10/12 holdout within 1e-8 relative; the 2 failures are below the binary64 range |
| PP09 | `pp09_laguerre_envelope_20261004` | L-0077 | **FAIL** | Published file lacks component fields (G1); certified envelope gains only 4.2x; derived check suggests the loss is in the recurrence adjoint, not `e^{L'/2}` |
| PP11 | `pp11_taylor_fused_total_20261004` | L-0078 | PASS | First total-target bound for the scaled-Taylor fused φ action; unbounded for `w ~ 1e100`, useless for subnormal `w` |
| PP15 | `pp15_fourier_comparator_20261004` | L-0079 | **FAIL** | Independent reviewer failed the drafted report (matched-error and work-scaling overstatements); corrections appended |
| PP10 | `pp10_leja_candidate_20261004` | L-0080 | PASS | Leja candidate always EstimateOnly, accurate in 72/72; more products than Chebyshev in 58/72 |
| PP13 | — | — | BLOCKED | Needs a calibrated directed/plain operation cost, which is a timing measurement; timing authority is HOLD |
| PP14 | — | — | DEFERRED | General closure `q(t)` needs Taylor-model enclosure of `sqrt(1 + |a|^2 + 2|b|^2)` along the path, not in the codebase; PP07 found no shared-action use, so no dependency forces it now; not preregistered |
| ACTIVATE | — | — | BLOCKED | Requires matched timing (HOLD) and a real client with shifted solves (PP07: none) |
| (review) | `INDEPENDENT_REVIEW_DAG.md` | — | no P0 | Independent diff review of `0153540..47a86b6`: one P1 (Fourier sup bound documented in the 1-norm; the code is a modulus bound), three P2, two P3; all fixed, recorded exports byte-identical |

17 ledger rows (L-0064 to L-0080): 9 PASS, 8 FAIL.

## What changed in the conclusions

- **Directed arithmetic had real defects.** REV-02's `exp_interval` floor was
  false on a small window, the stepped certificate took the wrong time
  direction for negative rates and measured its radius from the wrong
  point, and the chart decay used the rounded-up product of a decreasing
  function. No published end-to-end bound was below the truth, but the
  local compositions were wrong and are now repaired.
- **The REV-01c refresh now reaches the driver.** With refresh or cold
  GCRO-DR, the U-form driver finishes `brusselator-1d-50`, where the legacy
  recycling hits the attempt cap with 2,453 linear-solve failures.
- **The shared shift jet is worthwhile only for genuine shift families.**
  It wins counted flops on narrow and medium clusters (up to 12.6x), never
  for a common shift, and the only real client available (the Fourier
  client) has no shifted solves at all.
- **Stiff nonnormal decay is certifiable for tridiagonal convection-
  diffusion** with a verified log-norm and a symmetrizing metric; strongly
  nonnormal random and Jordan matrices remain out of reach.
- **The stiff Laguerre loss is probably not where R-NEXT-04 said.** PP09's
  derived (unregistered) check points at the recurrence-adjoint term; this
  needs the component fields exported before it can be stated.
- **The Fourier client is now native and certified** on its invariant
  leaf; its alias negative controls were mis-designed (harmonic content
  decays too fast to make aliasing harmful), which the FAILs record.

## Process notes

- Two preregistrations were amended before any run, each disclosed in
  the file: STORAGE-RUNTIME (G2 allows the error-message allocation) and
  PP05 (in two commits: charged start mismatch, internal phase witness,
  rotation enclosure by 2π reduction). The STORAGE-RUNTIME amendment was
  still too strict (the message string grows once) and its FAIL stands.
- PP08, PP09, PP10, PP11 and PP16 were implemented by delegated agents in
  separate worktrees on top of the pushed preregistrations; the integrator
  cherry-picked them, re-ran every recorded run, and obtained identical
  outputs (timing and path metadata aside).
- Corrections of my own results text after publication are appended, not
  edited in place (PP04, STORAGE-RUNTIME), except one restoration of text
  lost to shell expansion (PP07), disclosed in its commit.
- The ledger rows and preregistrations of PP08, PP09, PP10 and PP11 cite the
  agents' original commits, which were not on any pushed branch, so
  `tools/check-authority-refs.py` failed. A merge with strategy `ours`
  (tree unchanged) makes them reachable; nothing cited was rewritten.
- The PP08 and PP10 exporters panicked when their environment variable was
  unset, which stopped the workspace's ignored-test run; they now print and
  return like the other exporters.

## Validation

Full matrix on `2fe5902` (review fixes, exporter fix, `ours` merge), with
the dead `ShiftWork::add` removed afterwards (clippy `-D dead-code` after
the Hessenberg fix; the four clippy configurations and the readiness
script were re-run on the final tree):

| Step | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| clippy `-D warnings`: workspace, `audit2-research`, `audit2-bateman-authority`, `audit2-stage-certificate` | pass (after removing `ShiftWork::add`) |
| `cargo test --workspace --all-targets` and the three feature configurations | pass |
| `cargo test --workspace --profile measurement -- --ignored` | pass (failed on `47a86b6` in the PP08 exporter; fixed) |
| `tools/check-audit2-readiness.sh` | pass (after removing `ShiftWork::add`) |
| `tools/check-research-node.py --base origin/audit/rvj-native-followup-20261003` | pass |
| `tools/check-authority-refs.py` | pass after push (four agent commits unreachable before the `ours` merge was pushed) |
| `tools/check_ignored_tests_in_ci.py` | pass |
| `tools/test_*.py` | pass |
| `research/rvj_integration_20261004/validate_bundle.py` | pass |

No step was skipped.
