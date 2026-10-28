# R8–R10 theory decision and coding handoff

R8: Adopt the conditional native scalar-affine provider construction. Exact polynomial expansion, physical clock and Bernstein/Dini identities were independently checked. 324 rational test cases produced16524 exact algebra assertions and11016 Decimal90 analytic diagnostics, all passed. These diagnostics do not replace the continuum proof in THEORY.md. No new strong no-go candidate is admitted.

R9 coding contract: reuse directed::Interval, validate caller parameter boxes, retain all five quartic coefficients, and expose an opt-in FastDenseSegments adapter. Every proof-producing construction/query checks the declared FP arithmetic conditions. Query error is the outward sum of state error and the certified distance from actual returned bits to the exact represented polynomial. Per-call source/model/clock/unit/event/epoch checks and immutable owned native packet bits prevent stale reuse. Error carry requires exact endpoint/time/index adjacency.

Implementation must preserve forbid(unsafe_code); test-only FP-register negative controls live in the separate integration-test crate. Initial error and affine equation/source binding are caller premises. Typed strings bind these assertions; they do not verify an arbitrary physical callback.

R10: Theory unit complete. The native implementation and tests are the next required coding unit, not yet scientifically admitted at this checkpoint. Core7/23 and central28/88 remain fixed; SYM02 auxiliary0/1 until C10 acceptance. Reuse closed PR95/99 campaigns; no replay, performance timing, NCP or whole30 claim is introduced.

