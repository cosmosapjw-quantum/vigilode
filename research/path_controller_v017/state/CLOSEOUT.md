# Path Controller v0.17 durable closeout

- Base tag: `vectorized-jf-rhs-telemetry-v0.15.0-alpha1`
- Functional head: `a2e8834dfc429cd0e78a7654a41fd0bd075eecf2`
- Release tag: `vectorized-jf-path-controller-v0.17.0-alpha1`
- Successor branch: `vectorized-jf-endpoint-correction-v0.18`

## Durable result

- 6 cases, 22 schedules, 132 rows
- false accepts: 0
- non-hostile numerical failures: 0
- strict sub-eight-batch full-coverage survivors: 0
- depth-parity correctness survivor: `direct-decoupled-q2-c2`
- speculative six-batch lane: `direct-decoupled-q1-c2`

## Claim boundary

This tag is a direct/common-W correctness-reference cycle. It does not establish strict
Jacobian-free wall-time speedup, global fifth order, or production online certification.

BDF/Radau remain frozen comparators and were not modified.
