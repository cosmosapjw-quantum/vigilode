# RVJ integration re-audit, 2026-10-04 — prospective registration

## Question and pinned source
Can the offline Loop11 shared-center shifted-resolvent jet become a bounded native
research API with a current-target directed residual gate, without asserting a
production solver or timing improvement? Can two newly observed chart input
boundaries be closed while preserving the current target and existing behavior?

Implementation base: `826fa05cc1fdaf7b1a0cf7ac45116596b4ddff65` on
`audit/rvj-native-followup-20261003`; its ancestor WU25 is
`5a8d7fe9ffc681bca98a98a2f9889a2a05505783`.
Archival metadata branch: `6852b4af9ecadfb8c3aa50c4d240301dd1f3a4db`.
Offline full history SHA256:
`de25f591272d1b06d5ff597a43cf73ca026959997baa87b05eceaa70d687f0ca`.
Loop11 original ZIP SHA256:
`a7d66500012f2c982470afcbd96b489c2dc7da4391f2e0f0f8aa7eaefd326fe6`.
The archive byte check covered 4550 paths, zero mismatches, no science execution.
Prior scientific results receive one source/evidence audit; they are not rerun.

## New native slice and invariants
- Dense real, fixed current J; h >= 0; gamma0 > 0; positive target gamma;
  |gamma-gamma0|/gamma0 < 1. Finite exact-binary input interpretation.
- Verify a sufficient Euclidean dissipativity condition by outward row bounds of
  (J+J^T)/2. Reject unsupported J; negative eigenvalues alone do not suffice.
- Preserve every supplied independent RHS column. No implicit rank compression,
  stale-W substitution, arbitrary-mode deletion, or causal-stage parallel claim.
- Use normalized recurrence T=R0-I=gamma0*h*R0*J, W0=R0*B,
  W(k+1)=R0*Wk-Wk, evaluated at z=(gamma-gamma0)/gamma0.
- Candidate rounding and truncation are checked by a separately computed outward
  residual of the current exact input target, B-(I-gamma*h*J)U. An L1 upper
  residual bounds Euclidean solution error under proved inverse contractivity.
  A candidate passes only if every RHS absolute error bound <= requested tolerance.
- Report resource counts (factorizations, RHS solves, recurrence depth, evaluation,
  residual work and storage) with units. A planning filter is not a speed claim.
- Add opt-in research API and a runnable example. No default solver dispatch change,
  no complex shifts, no general metric, no coupled nonlinear Fourier native port.
- Fix newly identified chart NaN/shape/time-progress boundaries only. Published
  GCRODR refresh integration debt is reviewed and scheduled, not silently enabled.

## Commands and recorded experiment inputs
Environment: supplied Rust 1.94.1, locked offline dependencies. Root records exact
commands, versions and exit codes in evidence/COMMANDS.jsonl. Intended commands:
`cargo test --offline --locked -p rodas5p-core --test shared_shift_jet_contracts`
`cargo test --offline --locked -p rodas5p-integrators --features audit2-research --test rvj_integration_boundaries`
`cargo run --offline --locked -p rodas5p-core --example shared_shift_jet_study`
`python3 research/rvj_integration_20261004/verify_exact.py`
`cargo fmt --all -- --check`, affected crate tests and affected clippy.

New diagnostic/holdout fixture families (not archived Loop11 test replay):
1. Scalars, diagonal matrices and 2x2/4x4 skew+damping matrices, including zero J,
   h=0, gamma0=2^-400 or 2^400 with reciprocal J scaling, and ordinary gamma0=1/2.
2. 33 or 65 shifts, rho<=1/5; degrees 0, 8, 17, 24 where appropriate; independent
   RHS ranks 1, 2 and 4; small n for exact Fraction oracle and cost rows.
3. Holdout: 3x3 J=[[-2,7,0],[-7,-3,5],[0,-5,-4]], h=3/16,
   gamma0=3/4, targets gamma0*(1+j/128), j=-16..16; RHS e1 and (1,-2,3).
   Not used to tune tolerance/degree after observing its result.
4. Negative controls: spectrum-stable but not dissipative [[-1,100],[0,-1]],
   positive J, invalid dimensions, nonfinite values, empty RHS/target sets,
   invalid radius/degree/resource cap, deliberately wrong candidate and too-small
   tolerance. Their expected outcome is rejection/error, never false acceptance.
5. Chart: finite input with NaN problem parameters, wrong chart output lengths,
   and t0=2^53, h=1, t_end=t0+2; errors/status must be explicit without panic or
   state evolution at an unchanged time.

## Pass, kill, holdout and stop rules
PASS for the bounded native slice requires: exact independent oracle shows every
reported finite certificate encloses actual error; accepted error <= fixed 1e-10
absolute per RHS for ordinary/holdout cases; extreme-scale cases either enclose or
explicitly reject; all negative controls fail closed; existing relevant boundary
checks remain green. Under-resolution must reject, never relax tolerance.
Report certificate rejection rate separately; no 100% coverage requirement.
Counters must reflect actual executed operations; no speed promotion by operation
counts, single-run wall time, or comparison with repeated LU alone.
One independent decision review is required for admitting the slice; production
activation and end-to-end speedup stay HOLD. Preserve first failures and repairs.
Stop after mandatory affected gates and review; no full historical rerun, no
recursive reviews. Additional probes only resolve a concrete finding.

## Results (append only after execution)

## Executed result — appended 2026-10-04

Source commit `699a7adc6dba5fc45d4da01a5456f10f25dcf210` was published before the root combined verification.
15 new cases / 591 target candidates / 1341 RHS-target rows; exact oracle enclosure
failures0, certified1309/rejected32 RHS rows,11/11 negative controls rejected.
Fixed holdout passed the original1e-10 absolute tolerance. New core6+chart9 and
related existing8 tests all passed. All8 combined commands exited0.
Chart RED six failures is preserved; resource-slot review correction is preserved.
No historical full campaign replay, no default solver activation, no timing claim.
See RESULTS.json and evidence/COMMANDS.jsonl. Independent decision and final
artifact/process validation are recorded separately; CAS returned True with its
pre-evaluation symbol warnings retained.
