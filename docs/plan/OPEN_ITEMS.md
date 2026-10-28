# Open items after waves 1–3

- **Purpose.** One table of every item that waves 1–3 parked, deferred or left as a standing gap.
- **Sources.**
  - `docs/plan/sdd/WAVE{1,2,3}/progress.md` (rulings and parked findings);
  - `docs/plan/sdd/WAVE3/{final-review.md, final-fix-report.md, preflight-scan.md, task-*-review.md, task-*-rereview-1.md}`;
  - `docs/plan/IMPLEMENTATION_WAVE3.md` (carry-over map);
  - ledger rows L-0141 and L-0142;
  - the workstation `SUMMARY.json` files.
- **Code state.** Checked against `production/wave3-integration` `f2f20c1` (`/home/claude/wt/integration3`) where a row names code.
- **Severity.**
  - **High:** a gap in a P0 promise, or a blocker for gate G-A.
  - **Medium:** test strength, a production feature not yet built, or measurement validity.
  - **Low:** documentation, hygiene or process.
- **Owner wave.**
  - The proposal of `docs/plan/IMPLEMENTATION_WAVE4.md`: `W4/Tn` is wave-4 Task n.
  - `W5` is the next wave.
  - `Later` means a parent-plan phase (B–E) or the module disposition of §9.
  - `Infra` is the owner's workstation or the cloud environment.
- **Status.**
  - Open: no ruling yet; work is due.
  - Parked: deferred by a recorded ruling.
  - Deferred: carried by a plan's deferral list.
  - Accepted: a ruling says no action.
  - Closed: fixed later; the row is kept for traceability.
  - Won't fix: the artefact is ledger-bound.

| id | origin (wave / task / finding) | description | severity | owner wave (proposed) | status |
| --- | --- | --- | --- | --- | --- |
| O-01 | W2 p0-facade review minor; W3 task-3 ruling (`progress.md` l. 28); W3 final review "declined to judge" | The sequential dense route (`integrate_adaptive_dense_impl`, `dense_output_v2.rs:1954`) counts an estimate error as `unavailable` and **accepts the step** under `Enforce` and `WFilteredEnforce`. The fast dense route rejects such an attempt (D3). Mass-matrix and `InteriorError::Enforce` users get the weaker guarantee. | High | W4/T1 | Parked → Open |
| O-02 | W3 e1b-dense (L-0142); plan §2 "Global error" (F-033, owner decision not taken) | Global error accumulates beyond local interior control. Dense `error_own` is > 1 unit in 20/30 rows, 9.0 on Brusselator at rtol 1e-6, with 0 interior rejections there. The A2 "≤ 2 units" column mixes global and local. Tolerance proportionality is weak (Brusselator slope ≈ 0.78). | High | W4/T2 (study), W4/T6 (conditional controller card) | Open |
| O-03 | W3 b1c-staleness (L-0141 FAIL); node report observation I | B1 next mechanism. μ_S of the S-weighted norm blows up under component weights (HIRES μ_S ≈ 1.7e4 at y₀; 75/210 checks unbounded; fact/acc 0.588 > 0.4). Open choices: the weighted norm, or a drift test that does not need μ_S. | Medium | W4/T3 (card B1d) | Open |
| O-04 | W1 a2-wfiltered gate FAIL (2.14x steps vs Off); W3 plan decision | A2 retune parked. A κ = 2 card may open only on a preregistered witness set that excludes PR λ = −1e5. | Medium | W5 (after the O-02 study separates global from interior error) | Parked |
| O-05 | W3 final review M5(1); task-3 review M3 | `fast_dense_allocations.rs:107` passes with a margin of 1 (69 against 70 at rtol 1e-9). The budget absorbs a setup constant of 4–5 that the plan meant for growth only. | Medium | W4/T4 | Parked |
| O-06 | W3 final review M5(2–4); task-3 review M4 | Missing allocation tests: the banded fast-dense pipeline; `integrate_rodas5p_fast_dense_observed_with_atol`; the facade `with_jacobian_into` saving; the B3d zero fill (`solve.rs:778`). | Medium | W4/T4 | Parked |
| O-07 | W3 final review M6; task-6 report | CI `test` job runs `cargo test --locked -p rodas5p-integrators` (about 200 targets, debug) under `timeout-minutes: 120` on a 2-core runner. Wall time unmeasured; no ruling recorded. | Medium | W4/T7 (handoff timing + ruling) | Parked |
| O-08 | W3 task-4 review M4 | `fast_vector_atol_contracts.rs:615` ties test (b) to controller behaviour (`accepted_steps > loose.accepted_steps`). | Low | — | Accepted (note only) |
| O-09 | W3 task-5 review M5; final review "declined" | Silent paths in `events.rs`. `locate` returns no events when `extension.is_none()`. An `unwrap_or_else` hides an impossible state. An `at()` miss inside Illinois is reported as `EventFunctionNonFinite`. | Medium | W4/T5 | Parked |
| O-10 | W3 task-5 review M4 | Events suite not run on the fast route. | Low | — | Closed (T3 route assertion in `events_do_not_change_integration`) |
| O-11 | W3 task-6 ruling (`progress.md` l. 21) | rodas5p-core `too_many_arguments` (10/7) at `damped_moments.rs:492` (`query`). The clippy CI job runs with `--no-deps`, so core lints are unchecked in CI. | Low | W4/T4 | Parked |
| O-12 | W3 task-2 review M1 | B1c `results.json` writes `"verdict"` on ungated Richardson + Bounded arms (`b1c_staleness_contracts.rs:596-606`). The committed file is ledger-bound and is not edited. | Low | W4/T3 (new results name it `would_pass_gate` on context arms) | Won't fix (file); Open (generator) |
| O-13 | W3 task-2 review M2 | The zero-weight path under `StalenessRule::Bounded` (refresh, direct stages, no S-norm) has no end-to-end test. | Low | W4/T3 | Parked |
| O-14 | W2 final-fix ruling; W3 pre-flight (6 hunks) | `cargo fmt --check -p rodas5p-integrators` fails on `tests/production_audit_packet_curve.rs` (pre-existing at `4f92ec3`). CI `fmt` excludes integrators. | Low | W4/T4 | Parked |
| O-15 | W3 task-6 M3 (docs) | `rodas5p-fair-ab` (only compiled by `e4-fresh-clone-build.yml`) and `rodas5p-cli` are outside the production gate. | Low | Later (§9: Freeze → `vigilode-bench`; CLI Retire) | Accepted (documented exclusion) |
| O-16 | W1 ruling; W3 task-6 report | The Rust 1.94.1 pin cannot be installed in the cloud (rustup download blocked). Cloud builds use stable 1.97. The MSRV is checked only on the workstation and in GitHub CI. | Medium | Infra | Accepted (mitigated by workstation handoffs) |
| O-17 | W3 task-1 review M2 | The bench `RunStatus::ProtocolViolation` path is never exercised by a test. | Low | W4/T2 | Parked |
| O-18 | W3 task-1 review M4 | E1b PREREGISTRATION limitation 3 says "identical errors" without the cause. The file is bound by L-0142. | Low | — | Won't fix (cause recorded in `WAVE3_REPORT.md` §3.2) |
| O-19 | W3 task-1 review M5 | `vigilode_dense` breaks on the first `None` from `at` and leaves a partial `states`. Correct for the status logic. | Low | — | Accepted |
| O-20 | W3 task-1 review M1 | R-C5 bench rerun not an explicit handoff step. | Low | — | Closed (executed; `rc5_exit_codes.txt` MATCH) |
| O-21 | W3 plan carry-over (wave-4 deferral) | Vector atol on the sequential routes (mass, `Enforce` + dense output) is refused with `Unsupported::VectorAtolOnSequentialRoute`. Plumbing needs a vector form of `finish_step(.., atol: f64, ..)` (`sequential.rs:826`) and of the `dense_output_v2.rs` estimates (`&[atol]` at l. 1162, 1194, 1245). | Medium | W5 | Deferred |
| O-22 | W3 plan carry-over; plan §6 A3 | A3 controller hardening: van der Pol μ = 1000 rejections ≤ 20 % of steps at rtol 1e-4 (plan: 87 of 155), with no accuracy loss. | Medium | W5 | Deferred |
| O-23 | W3 plan carry-over; plan §6 A4 | A4 small-N and sparse (FD colouring) routes through the facade, with a verified declaration and typed mismatch. | Medium | W5 | Deferred |
| O-24 | W3 plan carry-over; plan §6 B2 | B2 JVP-only tier (structural or FD-coloured preconditioner, GCRO-DR, T9 forcing). Dense output on JVP-only problems is still refused. | Medium | Later (phase B) | Deferred |
| O-25 | W3 plan carry-over; p0-events rule | In-loop event stop. A terminal event truncates after the run, and work after the event stays charged. | Medium | W5 | Deferred |
| O-26 | W1 final review item 9; W3 plan carry-over | Facade-owned Stats types. `Stats` and `DriverWork` expose `AdaptiveStepConfig`, `WorkCounters`, `LinearSolverConfig`, `RoutingRecord`, `BandedWork`, `DenseErrorControl`, `InteriorSampling`, `FastInteriorWork`, `Rodas5pFastLu`, `DriverStopReason` and `AdaptiveFailureKind` (`solution.rs:5-10`). A 0.x semver baseline would freeze them. | High (G-A) | W4/T5 | Deferred → Open |
| O-27 | W3 plan carry-over; plan §7 corpus | Corpus v2 problems (E5, Pollution, Ring modulator, Akzo-Nobel, Medakzo, Gray–Scott, the interior-error set, oscillatory set). | Medium | Later (E1) | Deferred |
| O-28 | W3 plan carry-over; plan §7 rule 1 | The reference rule is still VigilODE at rtol 1e-12 with a Diffsol decisive cross-check, not "RADAU5 and CVODE agree to 1e-10". | Medium | Later (E1) | Deferred |
| O-29 | W3 local run `examples.log` | rustc `dead_code` warnings in `rodas5p-integrators/src/output.rs` (`prepared_*` fields l. 706–711, `DenseOutputPreparationWork` l. 716, `*_radii` methods). | Low | W4/T4 | Open |
| O-30 | W3 E1b `SUMMARY.json` notes; W2 seen 2026-10-20 | Workstation background load: `systemd-udevd` ~98 % and `modprobe` ~107 % CPU, 1-min load ≈ 23/24. It needs root to stop. Any timing on that host stays `diagnostic_only` until the load is gone. | Medium | Infra (owner) | Open |
| O-31 | W1 parked list; W1 a2 review I-3 | The sequential W-filtered path clones the LU/W every trial step (allocation per step on the mass/`Enforce` route). | Low | Later | Parked |
| O-32 | W1 parked list | Homotopy ledger tests cannot fail first (no RED). Frozen module. | Low | Later (§9 Freeze) | Parked |
| O-33 | W2 c1-transaction review; W1/W2 parked lists | `AdmissionRule` bins never forget successes (regime change inside a bin). | Low | Later (C4) | Parked |
| O-34 | W2 c1-transaction review | Causality check is a caller-supplied `charged_units > 0`; overshoot untested; off-by-one in `declines <= m` near integer 10τ. | Low | Later (C1/C4) | Parked |
| O-35 | W2 b1b-gmres review | GMRES: happy breakdown with `Continue` returns `Err`; a krylov error mid-stage breaks the ledger identity; explicit residual costs 3 LU + 2 W per column; no stage-level stale-P versus W⁻¹c test. | Low | Later (B1 promotion) | Parked |
| O-36 | W3 flake fix `9b12bff` | The leja and `bound_collective` flake fixes were verified in the cloud (60/60 parallel) but not by a workstation full suite on `f2f20c1`. | Medium | W4/T7 | Open |
| O-37 | W3 Global Constraint "node report at `docs/plan/nodes/<node-id>.md`" | Only `b1c-staleness.md` exists for wave 3. `p0-atol`, `p0-events`, `p0-fastdense`, `ci-docs`, `p0-api` and `e1b-dense` have task reports only in `docs/plan/sdd/WAVE3/`. | Low | W4/T0 | Open |
| O-38 | W2 p0-facade review minors | No `at()` check at an interior step end on the sequential store. JVP inside GMRES not asserted. `eprintln!` noise (none found in `crates/vigilode/src` at `f2f20c1`). | Low | Later | Parked |
| O-39 | W2 e1-bench review minors | Allocation scope misses mmap and the Fortran stack. The report test runs on a fresh small corpus. Duplicate failure blocks in `arms.rs`. | Low | Later (E1) | Parked |
| O-40 | W2 h-hygiene review minors | Thread-local allocation counters miss future spawned threads. The codegen-units 1→16 note is missing from README/REPRODUCE. `coupled-*.yml` manifest paths. | Low | Later | Parked |
| O-41 | W1 a1-api review minors | `StepSizeTooSmall` catch-all label. `Stats` is not `Deserialize`. No Fatal/Recoverable tests for the `f_t`, Jacobian and JVP callbacks. | Medium | W5 (after O-26) | Parked |
| O-42 | W1 a1-defects review minors | Residual-first stage-0 JVPs; `FixedWidth2` id literals; matrix-free zero-scale test only unstaged; `fallback.rs:110` receives atol = 0. | Low | Later | Parked |
| O-43 | W1 b1-lagged review minors | Lagged WRMS weights use `|y_n|` only. The 6th correction is not credited. `Rodas5pLaggedResult` is undocumented and not `Serialize`. The contraction ratio skips x₀. | Low | Later (B1 promotion) | Parked |
| O-44 | W1 t-verify review minors | Dead variables at `verify_theory.py:606-607`; no cross-check against `rodas5p_stage_transfer_constants`; Corollary 2a checked on a grid only. | Low | Later | Parked |
| O-45 | W1 a2-wfiltered review minors I-2, I-4 | The Lemma-3 test cannot distinguish the shift hγ from h/2. Two extra public helpers. (I-1, interior-forced rejection counter sum, is covered on the fast route by W3 test (e).) | Low | Later | Parked |
| O-46 | W3 task-3 review M3 | `FastDenseSegments::new` starts `t` at capacity 16, while `states` and `stages` start at 2 steps (inconsistent growth policy). Documented only. | Low | W4/T4 (the growth-form test makes the budget independent of it) | Parked |
| O-47 | W3 final review M4 (optional part) | No golden JSON of a dense-output `Stats` (`FastDense`/`interior` shape, events). | Low | W4/T5 | Open |
| O-48 | W3 task-6 (semver job) | `cargo-semver-checks` runs with `continue-on-error: true` until a first 0.x baseline exists. G-A makes it blocking. | Medium | W5 (after O-26) | Deferred |
| O-49 | W3 final review M1–M4; task-4 M2/M3; task-7 doc minors; task-6 new minor (Robertson y1 check) | Docs, seam tests, the asymmetric-vector test and the vector-atol allocation guard. | — | — | Closed (`5c6978d`; Robertson y1 is now relative, `examples/robertson.rs:47`) |
| O-50 | W1 rulings | Zero Richardson weight returns `InvalidInput` (atol = 0 unsupported on the lagged driver). The staged matrix-free zero-scale keeps a typed refusal. `Tier::Performance` routes no lagged driver. | — | — | Accepted (standing rulings) |

## Counts

- 50 rows. Nine need no action and are kept for traceability: O-08, O-10, O-15, O-16, O-18, O-19, O-20, O-49 and O-50. O-12 is split: its committed file is won't-fix, and its generator is open.
- Three rows are High: O-01, O-02 and O-26. All three are owned by wave 4.
- Wave 4 owns O-01, O-02, O-03, O-05, O-06, O-07, O-09, O-11, O-12, O-13, O-14, O-17, O-26, O-29, O-36, O-37, O-46 and O-47.
