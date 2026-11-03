# Progress log — switcher v2 remediation

Append-only (the gate rejects edits to earlier lines). One entry per task, format in DESIGN.md section 13.1.

## G0 — Verify the environment and the baseline at the design commit — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: gate-validate: 0, contracts: 0, vigilode-suite: 0, clippy: 0, fmt: 0
- red: none
- mutation: none
- notes: full suite passed in 131s; skeleton helpers reproduced registered counters (rotation 892 attempts, stiff-forced 66)

## G1 — Supersede ledger row L-0206 with INVALID and keep the as-preregistered rerun — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: reproduce: 0, research-node-check: 0
- red: none
- mutation: none
- notes: reproduced preregistered rerun verdict FAIL with transition39_n2_mf relative_error 0.5015; appended L-0207 INVALID


## G2 — Re-scope ledger row L-0205 to default-path neutrality — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: research-node-check: 0
- red: none
- mutation: none
- notes: output hashes of L-0205 verified; appended row L-0208 PASS superseding L-0205 to narrow claim to default-path neutrality
## F1 — Restore adaptive_evidence without loss-margin backoff (A-4) — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: contracts: 0, lib-unit-tests: 0, vigilode-suite: 0, clippy: 0, fmt: 0
- red: f1_adaptive_evidence_factory_has_no_loss_margin_backoff (failed on base)
- mutation: caught 2, missed 0, waived 0
- notes: restored EntryProbeAdaptation::default() in adaptive_evidence; route_registry retains backoff
## F2 — Convert WorkModel V2 ceilings to callback units (A-5) — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: contracts: 0, lib-unit-tests: 0, vigilode-suite: 0, clippy: 0, fmt: 0
- red: f2_v2_entry_ceiling_stops_losing_rotation_pilots_like_v1 (failed on base)
- mutation: caught 18, missed 0, waived 0
- notes: armed V2 pilot ceilings in callback units; pilot decisions match V1 economics
## F3 — route_registry uses WorkModel V1 until E1 is re-run (A-1) — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: contracts: 0, lib-unit-tests: 0, vigilode-suite: 0, clippy: 0, fmt: 0
- red: f3_route_registry_defaults_to_work_model_v1 (failed on base)
- mutation: caught 2, missed 1, waived 1
- notes: route_registry defaults to WorkModel V1; updated admission contract assertion
## F4 — Sound progress screen for RODAS5P: c_min = 7 callbacks (A-6) — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: contracts: 0, lib-unit-tests: 0, vigilode-suite: 0, clippy: 0, fmt: 0
- red: f4_progress_reach_upper_bound_boundary_table (does not compile on base)
- mutation: caught 31, missed 0, waived 0
- notes: implemented progress_reach_upper_bound with c_min = 7 callbacks; updated economy contracts
## F5 — Keep the B7 first-batch rule; document why and pin it (A-7) — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: contracts: 0, lib-unit-tests: 0, vigilode-suite: 0, clippy: 0, fmt: 0
- red: none
- mutation: none
- notes: documented B7 first-batch rule and pinned with contract test
## F6 — Replay-safety audit: charge it, keep the batch, disable trials (A-9) — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: contracts: 0, lib-unit-tests: 0, vigilode-suite: 0, clippy: 0, fmt: 0
- red: f6_replay_mismatch_keeps_explicit_progress_and_disables_pilots (failed on base)
- mutation: caught 17, missed 0, waived 0
- notes: charged replay safety audit; preserves explicit batch and disables competitive trials on mismatch
## F7 — Stable FNV-1a epochs; window epoch from endpoints only (A-11) — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: contracts: 0, lib-unit-tests: 0, vigilode-suite: 0, clippy: 0, fmt: 0
- red: f7_tolerance_epoch_is_fnv_stable (failed on base), f7_window_epoch_ignores_checkpoint_index_and_is_fnv_stable (failed on base)
- mutation: caught 17, missed 0, waived 0
- notes: stable FNV-1a digests implemented; window epoch now independent of checkpoint index
## F8 — Refuse a declared entry_policy that the schedule does not implement (A-10) — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: contracts: 0, lib-unit-tests: 0, vigilode-suite: 0, clippy: 0, fmt: 0
- red: f8_declared_entry_policy_must_match_effective_schedule (does not compile on base)
- mutation: caught 3, missed 0, waived 0
- notes: validated declared entry_policy against effective schedule; pre-flight refusal before callbacks
## F9 — Document the switcher route set and pin the post-remediation counters (A-8) — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: contracts: 0, lib-unit-tests: 0, vigilode-suite: 0, clippy: 0, fmt: 0
- red: none
- mutation: none
- notes: documented switcher route selection on Rodas5pWindowDriver::for_problem and pinned post-remediation reference counters
## X4 — DOPRI5 secant stiffness estimate (pure function) — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: integrators-lib: 0, integrators-clippy: 0, integrators-fmt: 0
- red: none
- mutation: caught 77, missed 1 (waived: h==0.0 scales d*=0 so dd==0 returns None), waived 1, unviable 0, unwaived 0
- notes: implemented dopri5_secant_h_lambda Rayleigh quotient estimator and stability gate
## X6 — Column colouring for compressed Jacobian assembly (pure module) — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: coloring-unit: 0, clippy: 0, fmt: 0
- red: none
- mutation: caught 30, missed 0, waived 0, unviable 1, unwaived 0
- notes: implemented Curtis-Powell-Reid greedy column colouring, seed vectors, and Jacobian recovery
## X8 — Margin-shifted comparison helpers (hysteresis band, pure) — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: margin-unit: 0, clippy: 0, fmt: 0
- red: none
- mutation: caught 31, missed 0, waived 0, unviable 0, unwaived 0
- notes: implemented MarginRatio, hysteresis band check band_is_open, and decision-preserving callback caps
## X1 — Shared per-step Krylov basis module (pure, unused by solvers) — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: krylov-tests: 0, krylov-clippy: 0, krylov-fmt: 0
- red: none
- mutation: caught 253, missed 23 (waived: subspace-invariant under Householder QR least-squares projection), waived 24, timeout 1, unviable 1, unwaived 0
- notes: implemented SharedBasis for shifted stage linear systems with direction reuse and Householder QR projection
## X2 — Opt-in shared basis in the matrix-free RODAS5P stage loop — 2026-11-04
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: integrators-tests: 0, integrators-clippy: 0, integrators-fmt: 0, vigilode-suite: 0
- red: none
- mutation: caught 15, missed 0, waived 0, unviable 16, unwaived 0
- notes: wired opt-in SharedBasis into Rodas5pMfFastWorkspace stage loop with bit-identical flag-off behavior, GMRES fallback on non-convergence, and >=50% JVP reduction on 1D heat equation
## X3 — Preregistered node K1: shared basis vs per-stage GMRES — 2026-11-04
- result: done
- commit: 8800fb9a
- toolchain: pinned 1.94.1
- commands: research-node-check: 0
- red: none
- mutation: none
- notes: recorded preregistered node K1 (L-0209, verdict FAIL against preconditioned per-stage GMRES due to per-stage Householder QR cost; SharedBasis retained strictly opt-in)
## X10 — B7 first-batch length: preregistered comparison of max and min — 2026-11-04
- result: done
- commit: 68692104
- toolchain: pinned 1.94.1
- commands: research-node-check: 0
- red: none
- mutation: none
- notes: recorded preregistered node B7 (L-0210, verdict FAIL: plain min raises stiff-forced RHS calls from 400 to 1840; max(budget, k_d0_min) retained in route_registry)
## X11 — Re-run E1 WorkModel v2 calibration with measured callgrind Ir — 2026-11-04
- result: done
- commit: 157586cf
- toolchain: pinned 1.94.1
- commands: research-node-check: 0
- red: none
- mutation: none
- notes: recorded preregistered node E1R (L-0211, verdict FAIL under verbatim aafc2e0b gates and live valgrind --tool=callgrind Ir; WorkModelVersion::V1 retained in route_registry)
## X9 — Hysteresis band in route_registry after a bias measurement (node H1) — 2026-11-04
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: contracts: 0, lib-unit-tests: 0, vigilode-suite: 0, clippy: 0, fmt: 0, research-node-check: 0
- red: none
- mutation: none
- notes: wired opt-in entry_margin and return_margin on SwitchingConfig with band_is_open validation and V1/V2 callback caps; recorded preregistered node H1 (L-0212, verdict FAIL due to transition-39 prefix bias |b| >= w/2; entry_margin and return_margin remain None by default)

## X5 — Stiffness-gated entry pilots and node D1
- status: done
- files: crates/rodas5p-integrators/src/dopri5.rs, crates/rodas5p-integrators/src/lib.rs, crates/vigilode/src/switching.rs, crates/vigilode/tests/switching_v2_remediation_contracts.rs, research/d1_stiffness_gate_20261104/PREREGISTRATION.md, research/d1_stiffness_gate_20261104/eval_d1.py, research/LEDGER.jsonl
- verification: `python3 tools/agent_gate.py finish X5`
- notes: exposed batch-minimum secant h_lambda telemetry from Dopri5 and wired opt-in stiffness_gate_fraction on SwitchingConfig; recorded preregistered node D1 (L-0213, verdict PASS: 100% reduction in discarded pilot JVPs on non-stiff accuracy-limited oscillatory families while preserving identical stiff entry window; remains None by default)
