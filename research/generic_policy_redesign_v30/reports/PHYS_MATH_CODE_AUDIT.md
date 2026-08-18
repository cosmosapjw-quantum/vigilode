# PHYS–MATH–CODE AUDIT — v3.0 Stage-Trajectory Geometry Discovery

## Code path

The v3.0 implementation does not add a new numerical stage. It retains values that the v2.9 audit path already computes:

1. level-1 report -> \(\rho_2\);
2. level-2 stage-3 report -> \(\rho_3\);
3. level-2 stage-4 report -> \(\rho_4\);
4. pure scalar post-processing -> \(s_{23},s_{34},\kappa_{234}\).

No new RHS, JVP, phi action, Jacobian build, factorization, Newton iteration, or runtime E-K activation is introduced by the richer telemetry.

## TDD / regression evidence

- Pure power-law shape unit test: PASS.
- Level-2 resumability parity contract: PASS.
- Early-flow-defect contracts: 4/4 PASS.
- Tolerance-scaled early-defect contracts: 3/3 PASS.
- v2.9 stage-growth CLI smoke: PASS.
- Integrator+CLI Clippy with `-D warnings`: PASS after replacing the temporary complex tuple with an internal result struct.
- `cargo fmt --check`: PASS.

## Replay evidence

Twelve family-sharded discovery replays were executed: 6 families at N=96 and the same 6 at N=256.

- 48 audit events total.
- 5 audit-E-inadmissible events, exactly matching v2.9.
- 0 budget breaches.
- 0 runtime full-E continuations.
- active switching false.
- N=192/N=384/N=2048 not executed.

A recursive v3.0-vs-v2.9 report comparison, excluding only wall-clock fields and the six newly added telemetry fields, is exact for all 12 shards. This includes R-JF trajectory/attempt information, work counters, budget decisions, prefix work, and audit-E safety labels.

## Scope locks

Unchanged SHA-256:

- `Cargo.lock`: `b6ed82be4e09045fbda4ba8b1a8782daef8840220baf88935d36a7046433afb4`
- `bdf.rs`: `68c681b58686e706ab6890d3be68bb1ca4a9398517e0e755a5c64b2cef746a04`
- `radau.rs`: `edec0453a5338fc8357a7c026aff894869641b486a3aa3df858353a315bb441b`

## Failure classification

This node fails **scientific feature promotion**, not solver correctness.

- Numerical/work parity: PASS.
- Telemetry implementation: PASS.
- Predeclared scalar safety feature family: FAIL/HOLD.
- N=192 calibration: remains BLOCKED and unopened.
- N=384 final safety holdout: remains SEALED.
- N=2048 scaling holdout: remains SEALED.
- Active switching: NOT AUTHORIZED.
