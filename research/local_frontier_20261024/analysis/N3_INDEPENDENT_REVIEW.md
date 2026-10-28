# Independent review of N3 cost-reference routing

Reviewer: conditioned-provider agent, separate from the N3 implementation owner.
Scope: `vigilode-coupled` switching controller, its adaptive-driver insertion,
unit/integration tests and `N3_REGIME_COST.md`. This is a read-only source and
mathematical review. No solver code was changed, and unchanged tests were not
rerun solely for this review.

**Verdict:** no blocking correctness defect found in the reviewed changes.
This supports an opt-in routing experiment; it does not establish general
production readiness or a wall-time improvement.

## Admission and commit behavior

The new `prepare` call receives the actual clipped trial step and the current
accepted starting state before either candidate or protected solver callbacks.
It discards a stale cost reference on model-epoch change and, when enabled,
on invalid step ratio, accepted-step age, dimension or scaled state distance.
Nonfinite distance/scale arithmetic fails conservatively. A declined reference
forces protected calibration rather than supplying an accuracy exemption.

Only a physically accepted protected attempt with a positive finite score
creates a reference. Rejected local-error attempts, fatal callbacks, failed
output collection and an epoch change before endpoint commit all keep
`record.accepted=false`; they cannot create a successful calibration. The
original full residual/error acceptance and final epoch check remain in the
adaptive driver. The guard adds no RHS/JVP callback and never reads an exact
solution. The strict-state integration test verifies equality with the direct
protected trajectory and equal RHS/JVP counts.

The reference state is the **starting** state of the protected calibration
step. The reference clock is recorded **after** that accepted step, so age zero
at the immediately following attempt means no further accepted step has
occurred; the state comparison nevertheless includes the displacement over
the calibration step. This convention is consistent with the code and should
remain explicit in diagnostic interpretation. Coordinate-copy receipts include
both the pending-state copy and, on successful calibration, the baseline copy.

## Compatibility and limitations

With `reference_guard=None` and unchanged model epoch, the former h/state
transfer, exponential averaging and routing thresholds remain in force. The
intentional exception is epoch invalidation, including Some-to-None and
None-to-Some transitions. A provider that always returns None cannot advertise
an unannounced model mutation; the existing immutable-model callback contract
still applies. The new optional public policy field is behaviorally defaulted,
but external Rust code using exhaustive struct literals must add the field or
use `..Default::default()`; this is not universal source compatibility.

The locality region bounds neither the Jacobian nor cost variation. The
absolute state scale has client units and needs an application-appropriate
choice. The extra O(n) comparisons and copies can outweigh callback savings
for cheap problems and must be included in whole-solve timings. Epoch checks
are observations, not synchronization against arbitrary concurrent mutation.

## Cadence mathematics

For stationary representative candidate cost C and positive protected cost P,
with one accepted protected fallback and L subsequent accepted protected
steps per failed probe, L >= ceil(C/(epsilon P))-1 gives
C/((L+1)P) <= epsilon in exact arithmetic. The implemented lower bound L>=1
is conservative for this statement. A finite maximum can violate the target,
and the receipt exposes the clipping and predicted ratio.

This is not a global runtime bound. Rejected fallbacks, changed event costs,
unpriced events, floating-point rounding at a ceiling boundary, and renewed
calibration require additional accounting. In particular a locality failure
resets the old cooldown; it can shorten the gap between probes. An empirical
mean cost is not a verified multiplicative enclosure. `N3_REGIME_COST.md`
correctly states the additional uniform error-bound assumption needed to
transfer a proxy inequality to actual costs and gives the correction factor
(1+delta)/(1-delta). No unconditional 25% overhead claim is justified.

## Reused validation

The current three reviewed source hashes exactly match
`evidence/runs/n3-round2-tests.receipt.json` (exit 0). Its stdout records 88
passing tests across the coupled library, integration targets and benchmark
example, including reference invalidation, rejected calibration, legacy
same-epoch behavior, callback failure, work counting, epoch changes and strict
guarded/raw trajectory equality. The later benchmark-model test receipt also
contains the same controller hashes. Review does not independently establish
benchmark speedups; measured route selection remains conditional on the
paired local campaign and stronger applicable comparators.

## Reviewed source identities

- `crates/vigilode-coupled/src/switching.rs`: `7f7d372cb0970099db393df68328d610451e744d02b892409e9491717b3de8b4`
- `crates/vigilode-coupled/src/adaptive.rs`: `fbc28b3d1a7b9416ea439500a1356c2e063db0c984997888f09eb6ffab331ed8`
- `crates/vigilode-coupled/tests/switching_contracts.rs`: `f6c86966f4f2f2ebd8d8251e0d60f288d78168b853d9f0f3616e8bdb19351c2d`
