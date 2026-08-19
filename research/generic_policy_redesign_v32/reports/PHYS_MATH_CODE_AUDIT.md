# v3.2 PHYS–MATH–CODE AUDIT

## Verdict

PASS for implementation/replay neutrality; DISCOVERY SURVIVOR for `zeta34`; activation remains forbidden.

## Equation-to-code map

- Pure drift helper: `pexprb54s4_quadratic_remainder_drift` in `crates/rodas5p-integrators/src/exponential.rs`.
- Retained level-2 report: `Pexprb54s4Level2PrefixReport::quadratic_remainder_drift`.
- Research row serialization: v2.9/v3.x stage-growth research row optional fields.
- Endpoint-free replay command: `generic-policy-redesign-level2-prefix --profile discovery96|discovery256`.
- Offline analysis: `research/generic_policy_redesign_v32/scripts/analyze_quadratic_remainder_drift.py`.

## Work neutrality

The helper consumes already-existing `y_n,U2,U3,U4,D2,D3,D4`; it adds no RHS, JVP, phi action, Jacobian build, or Newton iteration. Only vector arithmetic, tolerance scaling and WRMS-like reductions are added.

## Replay parity

All 12 endpoint-free v3.2 shards completed. Compared with the same-event v3.1 authority data:

- attempt rows: exact after excluding wall-time fields,
- accepted rows: exact after excluding wall-time fields,
- trajectory summaries: exact after excluding wall-time fields,
- event keys: exact,
- rho2/rho3/rho4 and chi34 authority diagnostics: exact,
- full E runtime continuation: 0.

The v3.1 audit-E local-admissibility label is joined only after event-key and stage-authority parity is proven.

## TDD / regression evidence

- pure quadratic cancellation / scaling / permutation / clock-tail / nonfinite contracts: PASS,
- report propagation contract: PASS,
- level-2 resumability parity: PASS before discovery,
- early-flow and tolerance-scaled telemetry regressions: PASS before discovery,
- Clippy `-D warnings`: PASS before precampaign freeze,
- BDF/Radau/Cargo.lock hashes remain frozen.

## Discovery gate

Predeclared authority witnesses were only `zeta34` and `relative_drift`.

`zeta34` passes all numeric gates:

- pooled AUC >= 0.70: 0.7674,
- min dimension AUC >= 0.70: 0.7105,
- min LOFO AUC >= 0.60: 0.6875,
- N=96/N=256 orientation agrees: higher-is-unsafe.

`relative_drift` fails. No discovery threshold was selected.

## Risk ledger

P0: treating discovery AUC as a safety certificate or selecting a threshold on N=96/N=256.
P0: using N=384/N=2048 before the independent calibration/holdout DAG permits it.
P1: N=96 has only one unsafe event, so its AUC=1.0 is weak evidence by itself.
P1: nonautonomous safe events overlap the unsafe zeta34 range, so monotone separation is not present.
P1: scalar-atol coordinate scope is not arbitrary-component invariant.
P2: debug wall-time is not performance authority.

## Final code verdict

The v3.2 mechanism is ready to advance exactly one predeclared feature (`zeta34`) to a separately frozen independent-calibration node. It is not ready for runtime E-K activation or switching.
