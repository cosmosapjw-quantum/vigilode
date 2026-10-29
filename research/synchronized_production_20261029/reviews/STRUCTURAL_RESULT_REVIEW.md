# Independent review of actual structural results

Outcome: **ACCEPT_SCOPED_RESULTS** for the reported structural improvements and retained adverse controls. Production remains **HOLD**. This review read retained raw outputs and performed independent postprocessing; it did not rerun a native solver.

Execution source `0a2a8eb9b9b0b775561f42f42cf36b1ef6e96f4b`, tree `5795e928f78fe3e99bfa91a4e7d10eaa7540c4db`, binary SHA-256 `9ecd8120e9ca7651fbaa525e9e6c1cb9eb582e41905e94a3648bf9b061795dbc`. Build receipt, actual executable, every raw-manifest member, command streams and pre/post native-source hashes were checked. Current native files still match the executed hashes.

## Actual validation and its scope

- Actual Rust stdout: **19 passed, 0 failed, 2 ignored**, including all11 new methods (8 kernel and3 driver). Both focused Clippy commands passed on the final source. The first published source’s probe Clippy failure remains retained and charged; it authorized no benchmark.
- The retained independent Fraction oracle reports88 exact checks on6 accepted and2 refused actual native records. This reviewer independently reconstructed original exact dyadic residuals and the rotation residual/error identity for all6 accepted records and checked output preservation for both refusals. This finite set of represented linear solves is not a generic callback roundoff proof or a whole-IVP enclosure.
- All**2460 full-IVP solves** were accounted for:60 correctness,160 warmup,2240 timed. All**35 paired ABBA/BAAB blocks** were independently reconstructed from child medians, matching every retained ratio and summary. Every timing/warmup sample has exactly the same counters, errors, accepted/rejected-step counts and proposal statistics as its qualified previous-start probe.
- Every solve succeeded and met the fixed17-grid-point diagnostic cap1e-5. All60 probe trajectories were decoded from binary64 bits and independently compared with the fixture’s analytic formula. Maximum error over all samples was**1.865174681370263e-13**. This is a binary64 analytic diagnostic, not an outward-rounded whole-path certificate.
- Scientific wall charge is**602.9004602280002/900 seconds**, including the first failed45.13034320800034-second check run and prior phase491.53919015300016seconds. Build preparation and this retained-output analysis are separate. Internal assertion counts in the review JSON count bookkeeping and individual values; they are not additional scientific tests.

## Measured structural and adverse outcomes

| Workload | Median GMRES/proposal time | All7-block range | JVP calls | MGS inner products | Proposal accepted/refused |
|---|---:|---:|---:|---:|---:|
| rotation | 3.425328 | 2.675085–3.903079 | 699→412 | 858→0 | 144/0 |
| rotation-large | 1.972945 | 1.930496–2.161135 | 702→414 | 864→0 | 144/0 |
| damped-rotation | 3.174434 | 2.930166–3.558505 | 699→412 | 858→0 | 144/0 |
| time-jvp | 1.186286 | 1.010904–1.395022 | 2718→2790 | 39168→34272 | 18/126 |
| badcoeff | 0.803978 | 0.746615–0.920267 | 699→1104 | 858→852 | 2/142 |

Ratios above1 mean a faster proposal arm. Ranges are observed block ranges, not confidence intervals. Both arms share one executable, precision, controller, starts and tolerances. Work includes refused proposals and fallback. All previous-start fixtures took18 accepted steps and0 rejections in both arms.

The three quadratic target workloads eliminate every Arnoldi column, MGS operation and GMRES cycle in the proposal arm, while lowering actual JVP work. This is a numerical structural improvement. The new plain vector loops still perform O(n) arithmetic; zero MGS does not mean zero total arithmetic.

The mixed16-rate workload is a tradeoff: JVP2718→2790 and strict unit cost3060→3132 worsen, while MGS39168→34272 and measured wall time improve. All18 zero-iteration log entries occur precisely at stage8 of each18-step solve (zero-based indices7,15,…,143). Their count equals the18 accepted proposals; all other stages retain16 GMRES columns. The414 proposal actions equal18×2 zero-start actions plus126×3 previous-start actions, consistent with actual construction/verification rather than initial-convergence shortcuts in this fixture. This concentration is an observation about the current source and fixture, not a theorem about general stage8 behavior.

The bad-coefficient control retains142 paid refusals and2 residual-valid acceptances. JVP699→1104 and strict cost987→1392 worsen; all7 blocks are slower (median time ratio0.803978, approximately24.4% more proposal time). Wrong relation coefficients can still produce an acceptable individual residual, so those2 acceptances do not invalidate the control. No generic default enablement or automatic routing is justified.

## Next algorithmic proof obligation

The current GMRES-into cycle fills its configured Arnoldi prefix before its least-squares solve except at breakdown. The time-JVP baseline uses16 columns per stage (2304 total). A subsequent research direction is earlier prefix termination with a rigorous residual-gap bound and full original-residual validation. Include every extra residual action and QR update in its cost condition. This is a new local research hypothesis, not another measured improvement in this delivery.

## Figure and report readback

The final `lane-structural-figures01/structural_performance.png` was visually inspected. All5 workloads and7 block points are present. Median diamonds, unit reference lines, axes and labels are readable, and the adverse case is retained. No legend clipping obscures a value. Counter-ratio labels agree with independently checked raw values. The footer correctly separates measured ranges from confidence intervals and instrumented operation counts from total FLOPs.

The current `REPORT_KO.md` and `LOCAL_CODEX_HANDOFF_KO.md` were read after root filled the structural results. Their numerical source/test/timing/work/error claims agree with the retained evidence; neither claims universal acceleration, completed nonlinear physical certification, native replay of PR102/103, or production readiness. They preserve time-JVP’s JVP/strict-cost regression, the bad-coefficient slowdown, and the prior failed Clippy run. They correctly distinguish absolute-residual acceptance from relative residual alone and distinguish cycles-derived QR/LS counts from hardware/function measurements. No blocking overclaim was found in those drafts. Source hashes of the read drafts are stored in the review JSON; later edits require a separate readback. Independent static reviews of external PR102/103 are separate evidence and are not recounted as this reviewer’s native tests.

## Current authority audit

Safe authority checks completed with lazy fetching disabled at the published source above. Immediately afterward, Git status/diff showed no tracked changes (only an unrelated untracked smoke temporary file); subsequent final report/handoff edits are outside that authority snapshot.

- Scoped: **FAIL**,202 citations/57 identities,6 diagnostics; missing1/unreachable3 distinct failing identities.
- Full: **FAIL**,11095 citations/1363 identities,178 diagnostics; missing59/repository-unavailable16/invalid-repository11/unreachable3 distinct failing identities.

With `GIT_NO_LAZY_FETCH=1`, missing means local object absence, not proof of remote nonexistence. The scoped failures comprise raw synthetic PR merge metadata and a historical runtime-preparation source label, but remain real checker failures. Receipts/classification are retained under `lane-structural-audit02`; the earlier interrupted attempt remains under `lane-structural-audit01`. Main-based research-node40 inherited diagnostics, physical certification gaps and inherited interactions remain open. Production approval is not supported.
