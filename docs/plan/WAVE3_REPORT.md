# Wave-3 close-out report

- Plan: `docs/plan/IMPLEMENTATION_WAVE3.md`. Spec: `docs/plan/PRODUCTION_PLAN_20261020.md` (§3 P0, §5 T1–T9, §6, §7).
- Final tip: `production/wave3-integration` at `f2f20c1`. This is the base for wave 4, in the worktree `/home/claude/wt/integration3`.
- Records used: `docs/plan/EXECUTION_LOG.md` rows 19–33; `docs/plan/sdd/WAVE3/{progress.md, preflight-scan.md, final-review.md, final-fix-report.md, flake-fix-report.md, task-*-review.md}`; `research/LEDGER.jsonl` rows L-0141 and L-0142; `research/b1c_staleness_20261021/results.json`; `research/bench_wave3_20261021/{REPORT.md, results_v2.json}`; and the workstation branches `results/local-wave2-08fd052`, `results/local-wave3-4945683` and `results/local-e1b-33e60cf`.
- Claim tags:
  - **D**: a recorded fact (a measured number, a commit, a ruling text).
  - **I**: an interpretation of recorded facts. Not tested.
  - **H**: a hypothesis that a later node can test.
- No sentence in this report is a speed claim. No preregistered gate is re-scored.

## 1. Nodes, commits and verdicts

| Node | Branch | Commit | Verdict / outcome (D) |
| --- | --- | --- | --- |
| wave3-base | `production/wave3-integration` | `75a0c6c` | `08fd052` (wave-2 integration after `w2-final-fix` and `h-checker`) plus the plan docs (pre-flight B0) |
| p0-atol | `production/p0-atol` | `997fefe` | Per-component atol on the fast dense and banded routes; a uniform vector is bit-identical to the scalar. Review approved, 0C/0I/5M |
| b1c-staleness | `production/b1c-staleness` | `129aa46` | Kill gate **FAIL** (L-0141). The lagged driver stays unrouted. Review approved, 0C/0I/2M |
| p0-events | `production/p0-events` | `f65d968` | Illinois event location on the dense extension, three samples per step; terminal events truncate the solution. Approved after 1 fix round |
| ci-docs | `production/ci-docs` | `eabe19d` | `production.yml` pinned to 1.94.1 (fmt, clippy `--no-deps`, tests per package, docs, semver on PRs, bench) and four facade examples. Approved after 1 fix round |
| p0-fastdense | `production/p0-fastdense` | `35d6604` | Dense output and W-filtered interior control on the fast drivers; facade dense output routes there; `STATS_SCHEMA = "vigilode.stats.v3"`. Approved after 1 fix round |
| p0-api | `production/p0-api` | `4945683` | Typed `SolveError::Unsupported(Unsupported)` with 7 variants; `#[non_exhaustive]` on facade enums and output structs, not on `Options` or `Atol`. Approved, 5 doc minors |
| w3-final-fix | `production/w3-final-fix` | `5c6978d` | Final review FIXES NEEDED 0C/2I/6M. I1 and I2 (seam tests) and M1–M4 (docs) fixed; M5 and M6 parked |
| e1b-dense | `production/e1b-dense` | `546333f` (handoff), `3e00b94` (ingest) | Measured at `33e60cf` on the workstation; L-0142 **INCONCLUSIVE** by preregistration |
| w3-flakes | `production/w3-flakes` | `9b12bff` | Test-only isolation fixes for the two pre-existing parallel flakes |
| wave3-integration | `production/wave3-integration` | `f2f20c1` | All nodes, E1b and the flake fixes merged; `check-research-node.py` PASS (142 rows) |

Process facts (D):
- Seven conflicts (C1–C7) and six false premises (FP1–FP6) were found by the pre-flight scan and resolved by rulings R-C1…R-C7 before dispatch.
- The Global Constraint "a node report at `docs/plan/nodes/<node-id>.md`" was met by `b1c-staleness` only. The other six wave-3 nodes have task reports in `docs/plan/sdd/WAVE3/task-*-report.md` and no node report in `docs/plan/nodes/` at `f2f20c1`. This is listed in `OPEN_ITEMS.md` (O-37).

## 2. B1c kill gate: FAIL

Preregistered gate (§6 B1, unchanged from B1 and B1b). Arm: `StageSolver::Gmres` + `StalenessRule::Bounded`. Settings: rtol 1e-6; the reference is the fast driver at rtol 1e-11. Per problem, three limits must all hold: factorizations per accepted step ≤ 0.4, accepted steps ≤ 1.05× fast, and endpoint error ≤ 1.5× fast.

| Problem | Accepted (fast / gated) | Factorizations | fact/acc | Steps ratio | Error ratio | Checks | Staleness refreshes | Unbounded checks | Problem verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| HIRES (n = 8) | 204 / 204 | 120 | **0.588** | 1.000 | 1.004 | 210 | 119 | **75** | FAIL (factorizations) |
| Brusselator 1D (n = 100) | 82 / 82 | 20 | 0.244 | 1.000 | 0.998 | 91 | 19 | 0 | PASS |

- **D.** The overall verdict is FAIL (L-0141, `supersedes: null`). It is final and is not re-scored. The ledger text reads "B1 family closed until a new mechanism, no routing change".
- **D.** On Brusselator, the corrected a-posteriori stop removed the B1b failure mode. B1b took 980 accepted steps with an error of 11.9×; B1c takes 82, the fast driver's count, with an error of 0.998×.
- **D.** On HIRES, 75 of the 210 checks returned q_pred = +∞, because h_P γ μ_S(J_P) ≥ 1. Those checks alone are 0.37 refreshes per accepted step. The other 44 staleness refreshes come from a finite q_pred > 0.5. The GMRES arm needed no contraction refresh and no budget refresh.
- **D.** The pre-flight scan found μ_S ≈ 1.7e4 at the HIRES initial state. The weight ratio s₀/s₁ ≈ 1e4 enters row 1 of the weighted log norm.
- **I.** The binding cost on HIRES is the bound's hypothesis, not the iteration. The log norm in the S-weighted norm is inflated by the weight ratios. The lemma then gives no bound at steps where GMRES would still have converged.
- **D.** The scipy-Radau proxy of the pre-flight scan predicted 0.39 against the 0.4 gate. The measured RODAS5P value is 0.588. The proxy was informational only.
- **D.** Richardson + Bounded was measured as context only: 0.789 on HIRES and 0.890 on Brusselator. The committed `results.json` carries a `"verdict"` field on these ungated arms (task-2 review M1, parked; see O-12).

## 3. E1b-dense: re-measurement with every arm interpolating

- **D.** Run at `33e60cf` on the workstation: Ryzen 9 5900X, `taskset -c 2`, `OPENBLAS_NUM_THREADS=1`, rustc 1.94.1 (pinned), SUNDIALS 6.4.1, gfortran 13.3.0, binary sha256 `31cb97ea…5736`.
- **D.** The run produced 132 rows (robertson, hires and van der Pol 27 each, Brusselator 51), with **0 protocol violations** and **0 output-clipped steps in all 30 dense rows**.
  - The dense driver ids are exactly `rodas5p-fast-dense-v1` and `rodas5p-fast-banded-dense-v1`.
  - The control that ran matched the arm in 30 of 30 rows.
  - All four references are undisputed.
  - The R-C5 bench gate passed: 16 tests passed and clippy exited 0.
- **D.** The only non-success row is DVODE mf21 on van der Pol at rtol 1e-8 (ISTATE −1), the same row as in wave 2.
- **D.** The verdict is INCONCLUSIVE, fixed in advance by the preregistration. L-0142 supersedes L-0140 for VigilODE-relative ratios only.

### 3.1 Matched-error ratios (diagnostic only)

Host disclosure (D): during the run the 1-minute load average was about 23 of 24 CPUs (23.53 before, 21.62 after). `systemd-udevd` (~98% CPU) and `modprobe` (~107% CPU) caused it, a pre-existing host condition that needs root to stop. A udev worker last ran on CPU 2, the pinned core, at the post-run snapshot. `timing_authority = diagnostic_only`. These numbers support no speed claim, and ratios across hosts (v1 against v2) are never compared.

Computed from the section "Endpoint-only VigilODE (dense output)" of `research/bench_wave3_20261021/REPORT.md`:
- `time ratio` = comparator ms / VigilODE ms, at matched `error_common`. Above 1 means the comparator spent more wall time.
- The median is over the MATCHED levels only. "k/15" counts the matched levels out of the 15 common levels.
- Band-structured comparators are compared with `vigilode-banded-dense`, all others with `vigilode-dense`.

| Problem | Comparator | Matched | Median time ratio [min–max] | Median RHS ratio |
| --- | --- | --- | --- | --- |
| robertson | diffsol-bdf-dense | 6/15 | 2.18 [2.07–2.36] | 0.82 |
| robertson | cvode-bdf-dense | 5/15 | 4.96 [4.73–5.50] | 1.02 |
| robertson | dvode-mf21 | 5/15 | 1.68 [1.52–1.72] | 1.15 |
| robertson | radau5 | 5/15 | 0.51 [0.47–0.60] | 0.96 |
| robertson | rodas | 7/15 | 0.67 [0.47–1.07] | 1.25 |
| hires | diffsol-bdf-dense | 6/15 | 1.51 [1.00–1.80] | 0.65 |
| hires | cvode-bdf-dense | 5/15 | 4.10 [3.70–4.32] | 0.70 |
| hires | dvode-mf21 | 5/15 | 1.26 [1.04–1.47] | 0.69 |
| hires | radau5 | 3/15 | 0.68 [0.55–0.81] | 0.91 |
| hires | rodas | 8/15 | 1.00 [0.57–1.59] | 1.27 |
| van-der-pol-mu1000 | diffsol-bdf-dense | 4/15 | 2.74 [2.67–2.89] | 0.75 |
| van-der-pol-mu1000 | cvode-bdf-dense | 3/15 | 5.60 [5.39–5.81] | 0.76 |
| van-der-pol-mu1000 | dvode-mf21 | 0/15 | — | — |
| van-der-pol-mu1000 | radau5 | 5/15 | 0.47 [0.46–0.51] | 0.79 |
| van-der-pol-mu1000 | rodas | 7/15 | 0.74 [0.43–0.99] | 1.19 |
| brusselator-1d-n100 | diffsol-bdf-dense | 5/15 | 1.35 [1.03–2.06] | 0.48 |
| brusselator-1d-n100 | diffsol-bdf-sparse (vs banded) | 5/15 | 0.72 [0.61–0.98] | 0.48 |
| brusselator-1d-n100 | cvode-bdf-dense | 5/15 | 3.00 [2.92–3.97] | 0.35 |
| brusselator-1d-n100 | cvode-bdf-band (vs banded) | 5/15 | 1.46 [1.36–1.74] | 0.35 |
| brusselator-1d-n100 | dvode-mf21 | 5/15 | 0.52 [0.51–0.72] | 0.35 |
| brusselator-1d-n100 | dvode-mf24 (vs banded) | 5/15 | 0.49 [0.46–0.62] | 0.35 |
| brusselator-1d-n100 | radau5 | 6/15 | 1.09 [1.05–1.21] | 0.53 |
| brusselator-1d-n100 | radau5-band (vs banded) | 6/15 | 0.49 [0.47–0.54] | 0.53 |
| brusselator-1d-n100 | rodas | 7/15 | 0.92 [0.85–1.03] | 0.76 |
| brusselator-1d-n100 | rodas-band (vs banded) | 7/15 | 0.64 [0.60–0.71] | 0.76 |

- **D.** For van der Pol against DVODE, no level is bracketed by both arms. The DVODE failure at rtol 1e-8 removes the high-accuracy end of its curve.
- **I.** The ratios are consistent in direction with the wave-2 E1 row and with plan §2. VigilODE dense needed fewer comparator-relative RHS calls on the small dense problems. On Brusselator, the banded BDF and Radau codes needed fewer RHS calls than VigilODE banded dense. Under the disclosed load this is a pattern in diagnostic data, not a ranking.

### 3.2 Global-error observation

`error_own` is the maximum over the 101-point grid of the WRMS error against the reference, with the run's own weights rtol·(atol_scale + |y_ref|).

- **D.** VigilODE dense rows have `error_own` above 1 tolerance unit in **20 of 30 rows**:
  - Brusselator: all four dense arms, at 1.134 (rtol 1e-4), **9.000** (1e-6) and 8.775 (1e-8).
  - van der Pol: `vigilode-dense` 1.634, 2.041 and 2.723; `vigilode-dense-off` 1.345, 2.073 and 2.748.
  - HIRES: `vigilode-dense-off` 3.620 (1e-4) and 7.446 (1e-6).
- **D.** In the A2 section (`WFilteredEnforce` against `Off`, 15 entries), `error_own(enforce)` is within 2 units in **9 of 15** entries. The 6 entries above 2 units are van der Pol at 1e-6 and 1e-8, and Brusselator dense and banded at 1e-6 and 1e-8. Their error ratios Off/Enforce are 1.016, 1.009 and 1.000. On the Brusselator entries the interior rejections are 0, and the maximum interior estimate is 0.64 and 0.39.
- **D.** Where the interior control acted, it lowered the grid error. On HIRES the error ratio Off/Enforce is 3.70 (rtol 1e-4) and 11.8 (1e-6), at step ratios 1.15 and 1.045.
- **I.** The six failed "≤ 2 units" entries are not interior-control failures. Interior control is local and its estimates were below one unit. The grid metric is a *global* error, which accumulates over the run. This matches the propagation finding of plan §2 ("Global error", F-033). The `≤ 2 tol units` column of the A2 table therefore mixes a global measurement with a local criterion. It is evidence for A2 only on rows where the global error is not dominated by accumulation.
- **H1.** On Brusselator and van der Pol, each accepted step's true local error is ≤ 1 unit, and the global error is their accumulation through the flow. This is the case where an error-per-unit-step criterion (T4 with μ ≤ 0) would bound the global error.
- **H2.** Alternatively, the embedded estimate under-reports the local error, or the flow amplifies it (μ > 0 regions of van der Pol). H1 and H2 are distinguishable by a per-step local-error measurement, which wave 4 preregisters (`IMPLEMENTATION_WAVE4.md`, Task 2).
- **D.** Tolerance proportionality is weak on Brusselator. The absolute grid errors are 1.1e-4, 9.0e-6 and 8.8e-8 at rtol 1e-4, 1e-6 and 1e-8, a log–log slope of about 0.78 over 4 decades. This is computed from the table above, not by the report generator.

## 4. Workstation test results

| Run | SHA | Toolchain | Workspace pass / fail / ignored | Wall (s) | Other suites | Matrix |
| --- | --- | --- | --- | --- | --- | --- |
| local-wave2 (`1842daa`) | `08fd052` | 1.94.1 pinned | 1580 / 0 / 110 | 2765 (warm cache) | hybrid 19/0, bench 11/0 | all MATCH |
| local-wave3 (`32ff334`) | `4945683` | 1.94.1 pinned | 1634 / **2** / 111 | 3072 | hybrid 19/0, bench 11/0, checker unittest 16 OK, checker gate PASS (141 rows), 4 examples exit 0 | 2 DIFF (the failures) |
| local-e1b | `33e60cf` | 1.94.1 pinned | — (bench R-C5: 16 / 0) | 20.6 (wp) | — | all MATCH |

- **D.** Every new wave-3 test target passed on the workstation:
  - `b1c_staleness_contracts`: 6 passed, 1 ignored;
  - `fast_vector_atol_contracts`: 4;
  - `fast_dense_contracts`: 8;
  - `fast_dense_allocations`: 1;
  - `events_contracts`: 15;
  - `p0_api_contracts`: 9.
- **D.** The two failures were the following.
  - `rodas5p-core::leja_setup_work_contract::counted_action_records_cold_coefficient_table_without_changing_authority` fails the assertion `receipt.subtotal().scalar_proxy_units() > 0`. In the cloud on `5c6978d` it failed in 30 of 40 parallel runs and in 0 of 10 serial runs. Neither `rodas5p-core` nor the test changed since `08fd052`, and the same test also failed in the wave-1 integration.
  - `rodas5p-integrators::production_audit_bound_collective::warmed_interval_rhs_and_log_norm_make_no_heap_allocations` fails `(1, 0) == (0, 0)`. It failed in 3 of 40 parallel runs and in 0 of 10 serial runs. Its counter is a global `ALLOCATIONS` counter, the pattern of the krylov flake fixed in `08fd052`.
- **D.** Ruling (`progress.md`): both are pre-existing order-dependence flakes, not wave-3 regressions.
- **D.** The fix `9b12bff` changes tests only:
  - the leja assertion is now a biconditional on `InitializedByThisCall`, which is stronger, not weaker;
  - `bound_collective` uses a thread-local gate.

  It passed 60/60 in parallel and 10/10 serially in the cloud, and was merged at `f2f20c1`.
- **D.** The fix has not yet been confirmed by a workstation full-suite run on `f2f20c1`. The wave-4 local handoff carries that row (O-36).
- **D.** `examples.log` contains rustc `dead_code` warnings from `crates/rodas5p-integrators/src/output.rs`: the `OutputCollector` `prepared_*` fields, `DenseOutputPreparationWork` and the `*_radii` methods (O-29).

## 5. What was deferred

Each item has a row in `docs/plan/OPEN_ITEMS.md`.

- **From the wave-3 plan's carry-over map (wave 4):**
  - A3 controller hardening;
  - A4 small-N and sparse colouring routes;
  - B2 JVP-only tier;
  - vector atol on the sequential route;
  - in-loop event stop;
  - facade-owned Stats types;
  - corpus v2 problems;
  - the §7 rule-1 reference (RADAU5 and CVODE agreeing to 1e-10).
- **Parked by wave-3 rulings:**
  - the sequential route accepting a step whose interior estimate is unavailable under `Enforce` or `WFilteredEnforce` (task-3 ruling);
  - final-review M5 (allocation-test margin and missing allocation tests) and M6 (CI timing);
  - the B1c minors M1 (the `verdict` field on ungated arms) and M2 (unpinned zero-weight path);
  - p0-events M5 (silent no-op paths) and the `at()` miss reported as `EventFunctionNonFinite`;
  - e1b minors M2 (`ProtocolViolation` untested), M4 and M5;
  - clippy `--no-deps` (`damped_moments.rs:492`);
  - the `production_audit_packet_curve.rs` fmt failure.
- **A2:** the retune stays parked. The E1b evidence above shows that the A2 acceptance cannot be read off a global-error column (§3.2 I).
- **B1:** the family stays closed until a card with a new mechanism opens. Wave 4 proposes one (B1d). It is a new card with its own preregistration and not a re-score of B1c.
