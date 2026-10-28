# A02-TXN-01 mechanical acceptance closure v02 — pure preregistration

This immutable successor preserves PREREGISTRATION.md and IMPLEMENTATION_RECORD.json
as v01 historical inputs. It does not authorize numerical execution or scientific
admission. No results are appended here. The current full A02 implementation
remains HOLD pending separate B1–B6 disposition and independent source review.

Allowed v02 changes: source-derived per-field PacketWorkCounters expectations,
assertions and before/after output for AT11 prescribed operations; exact reference
coverage for every successfully returned SUBNORMAL_TIME/SUBNORMAL_RADIUS observation;
refusal/work/input identity recording; source and command bindings. No trajectory
formula, numerical tolerance, fixture, cap, model convention or counter semantics
is changed. The sole packet_trajectory.rs change is within cfg(test) private seams.
A03/A05 and DC1 owner source are not edited by this closure.

Source-derived vectors in tests/support/a02_packet_counter_contract.rs expand all
PacketWorkCounters fields (unlisted fields are exactly zero). Expected prepare
vectors include compile_packet, generate_candidate, represented freeze, frozen
zero-tube owner, output workspace and the existing local-model constructor charge.
Initial state setup has a zero packet-counter vector. Successful/historical query
uses the frozen curve degree O+3D+1. Commit and physical tolerance rejection use
the prescribed completed-query count; stale/first test seam is a zero vector.
TXN_FAIL_SECOND completes exactly one F_FIXED interior query. Each measured delta
is asserted field-by-field without deriving expected values from observed counters.

For the first F_REJECTION h=1/4 trial, the chi=0/O2 frozen curve has the residual
term magnitude |z0|*t^2/2. Its whole-window sup is 1/128; at requested t=1/8 its
T1 residual contribution is 1/1024>the unchanged 2e-4 mean tolerance, hence the
source tally fixes one completed query before rejection. This arithmetic identifies
the expected path before execution; numerical execution must still confirm it.

Subnormal oracle inputs are decoded from binary64 bits to Fraction before
perturbation, multiplication or subtraction. SUBNORMAL_TIME uses original
M_SUBNORMAL, exact minsub initial amplitude and stationary phase. SUBNORMAL_RADIUS
uses M_INHERITED with exact minsub radii and perturbations (+eta0,0),(0,-eta1),
independent of its ordinary fixture's 1e-6/2e-6 radii. Successful observations all
receive slack-free disk checks. Typed refusals record work/inputs and do not count
as inclusion PASS. Both arms must be present in invalid.json.

Commands for a future independently reviewed and controller-pushed EXEC_SHA:

```sh
VIGILODE_A02_OUTPUT=<native.json> VIGILODE_A02_ALLOCATION_OUTPUT=<alloc.json> VIGILODE_A02_INVALID_OUTPUT=<invalid.json> cargo test --locked --offline -p rodas5p-integrators --test production_audit_verified_trajectory -- --nocapture
cargo test --locked --offline -p rodas5p-integrators --lib verified_transaction_private_tests -- --nocapture
python3 research/production_audit_20261018/evidence/continuation_a02_research/txn_01/fraction_oracle.py <native.json> <invalid.json> <oracle.json>
cargo test --locked --offline -p rodas5p-integrators --doc packet_trajectory
```

Frozen original wall limits: Rust test binary1800s, unequal Fraction oracle600s,
one-mode oracle120s. Preserve first raw stdout/stderr/exit and source/config/input
identity. Allocation claims still require the same allocator's three-Vec positive
control; failed calibration is ALLOCATION_UNAVAILABLE. No commands above have run
for this revision. A future B1–B6 repair changing any counted path requires a new
pure preregistration/source review and revised pre-execution expectations.
