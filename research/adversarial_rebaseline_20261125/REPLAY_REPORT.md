# Adversarial rebaseline result, 2026-11-25 KST

**Measured commit:** 872a9598e76f2166b3b1e6aa837439a20d735ed7, tree 917a477ecc700b1a7e2c3de77dc1253f4e9b5d37. Solver sources identical to published RETRO-WARP 14a910fd9e9f9579859a00021ec18d133e301bc2; benchmark standalone lock synchronized in measured commit. Original benchmark report identifies the older 933922495 baseline. Old report and all eight source datasets were hash-verified and preserved, not overwritten.

## Executed replay

All eleven build/run invocations recorded in receipts/exit.tsv exited 0 (three builds, eight data generation commands). Exact original cross_solver_v1_valid.json, spatial v3 and full-scope arms; no new opt-in arms inserted into original fixed-arm matrix. Original Rust 1.99.0 exact compiler identity (stable alias), Julia 1.12.7 and original Julia manifest, CPU0 with single-thread environment, unpinned ensemble as separate cohort. Opt battery uses pinned workspace rustc 1.94.1; old opt compiler not independently recorded. Timing diagnostic, not controlled release qualification.

| Dataset | Rows | Solver statuses | Exact finite recorded error values |
|---|---:|---|---:|
| cross_rust | 126 | 123 success / 3 solver-failure | 123 |
| cross_julia | 45 | 44 Success / 1 Unstable | 44 |
| spatial_v3 | 378 | 378 success | 378 |
| opt_battery | 96 | status not recorded | 96 |
| full_rust | 242 | 234 success / 8 failed | 191 |
| full_ens | 3 | 3 success | not comparable via error_common |
| full_julia | 112 | 109 success / 3 failed | 103 |
| full_scipy | 30 | 30 success | 30 |
| Total | 1032 | 921 recorded success / 15 failure / 96 unrecorded | 965 |

All 1032 row keys and recorded status categories match the original. All 965 pairs of finite recorded common/own errors are numerically identical. Eighteen regenerated reference arrays are exactly equal to the old values, not an independent proof of accuracy. Canonical/spatial source guard VERIFIED_BUILD_SOURCE; zero protocol violations. The 96 opt rows cannot be classified as successes; in particular B04 hits the cap and B02 has large error. A terminal/tangent raw success is not an event semantic pass. Original fixed-arm replay does not evaluate the new 29-route portfolio as a whole.

## Diagnostic findings

At rtol=1e-6: Robertson RHS-only auto 9.767871 -> 0.276977 ms, RHS 214333 -> 6269, actual endpoint error changes 2.95e-7 -> 5.34e-7. This is not a matched-error speedup or an isolated RETRO attribution. Brusselator n400 auto JVP 1057.601537 ms vs sparse JVP 7.474738 ms, JVP counts 67949 vs 368, Krylov iterations 65480 vs 0, same accepted/rejected 92/12, RHS820, nearly identical endpoint errors. Periodic advdiff auto 42.809739 ms, 4368 JVP, 4160 iterations. Analytic ft PR 0.052849 ms, Enforce 0.855665 ms; different achieved errors. DAE Robertson 0.379397 ms vs RADAU5 0.027581 ms; analytic index-1 0.287216 vs 0.014817 ms. Inconsistent IC and pendulum-index3 native failures persist. Terminal run still full [0,8.5] then truncation; native RHS340 vs CVODE43; tangent roots_found=0. These are output-contract differences, not merely performance gaps.

Unpinned ensemble 1t 6.343395 ms, 8t 1.548770 ms; same observed max error 6.257362e-8, approximately 4.10x on this diagnostic run. step_into remains zero allocation; unified n256 full solve allocations 130 -> 209, bytes 214018 -> 249577: investigate separately from workspace guarantee. Five typed capability refusals preserved. Nominal spatial comparator/native ratios at 1e-6: DVODE band .45776, RADAU band .70063, RODAS band .70290, diffsol sparse 1.62460, CVODE band 1.86392 (comparator time divided by VigilODE banded dense). No achieved-error ranking inferred.

## Reference adjudication

Canonical 5/5 disputed, spatial 7/9 disputed: 12/14 remain disputed. All 900 canonical matched rows blocked. Spatial 2340 matched records: 1820 disputed, 262 neither brackets, 148 baseline not bracket, 22 comparator not bracket, 88 MATCHED on the two undisputed sphere-photon problems. Matched records are diagnostic and not the same as original solver-run rows. Do not relabel the disputed records or claim overall external superiority. M0 must repair reference evidence before M6 full same-output matched-error campaign. See RESEARCH_PLAN.md.

## Provenance and bounds

The initial exact named toolchain alias was incomplete, but stable is the same original compiler build. Initial --locked build refused outdated standalone lock. Offline Cargo resolved only local package version synchronization (harness) and required spectral dependencies rustfft/primal-check/strength_reduce/transpose (benchmark and harness). Existing external registry version set unchanged apart from added spectral dependencies. Original pre-build lock hash in receipts/HARNESS_SHA256SUMS is intentionally historical; final inputs in receipts/HARNESS_FINAL_SHA256SUMS. Build logs retain deprecated-API warnings. No native Superpowers/subagent execution exposed here. Existing external libraries are not newly qualified by this replay. New full offline dictionary intake, event/DAE, aperiodic and cached-RHS implementation not performed in this baseline stage. Release remains HOLD_NOT_RELEASE_QUALIFIED.
