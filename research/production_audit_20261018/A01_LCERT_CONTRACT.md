# A01-LCERT-01 frozen bounded contract

Status: ready for one bounded implementation and review unit. This unit certifies a `kappa=0`, rotating-frame, represented local packet window through `RepresentedPacketCurve -> PacketCurveOutput`; it does not promote legacy `compute_certificate`, `reconstruct_outward_at_times`, A02 trajectory, damped packet behavior, generic output, speed, or overall production.

## Frozen source and convention

- Branch: `audit/production-readiness-20261018`
- Base HEAD: `d087dab9a8dacff0b195566c4a649273acdb476c`
- `omega_j = width * xi_j`; carrier remains metadata and is not added to the rotating-frame RHS or output.
- This local certificate is tolerance-independent: the existing `PacketCurveOutput` path has no physical tolerance gate. Do not add a new physical accuracy or radius threshold. Existing solver tolerance gates remain in A02 trajectory and are outside this unit.
- No changes to `collective_model.rs`, `directed.rs`, Cargo manifests/lock, prior ledger, A05 work, model equations, oracle thresholds, or arithmetic evaluation order.

## Certificate contract

Bind generated model identity, frozen represented mode coefficients, horizon, and initial mode uncertainty in a private immutable owner. A successful query returns only an immutable certified view. Preserve prior owner state and cumulative attempted work on refusal. Do not add a physical tolerance gate: this local result states solution inclusion only, not requested-accuracy satisfaction or trajectory acceptance.

For `S=sum_j w_j`, `E+` the validated initial weighted energy upper bound, `M_Y` the bound on the actual represented weighted mean, and `rho_w` the continuous residual bound of that same stored curve, retain

`lambda = S*|chi|*max(S*E+, M_Y^2)`

`B_w(t) = exp(lambda*t) * (e0,w + t*rho_w)`.

Use mode radius `B_w/sqrt(w_j)` plus interval displacement from the exact returned center; use mean radius `sqrt(S)*B_w` plus mean displacement. The full represented-polynomial residual already covers compiler/moment/candidate/reconstruction error; do not add a separate exponential tail.

Instrument setup, coefficients, certificate, output and refusal work. Check setup sizes. Measure only the warmed successful query for the local zero-allocation claim and require the same allocator's three-Vec `CollectiveModel::clone` positive control first.

## Frozen acceptance

- Apply independent exact one-mode and unequal-frequency Fraction references (4 arms, 3 modes, five times, 80 disk checks) to this new API's returned centers/radii.
- Compile-fail attempts to mutate center/radius/authority.
- Exact reference inclusion remains slack-free, with frozen epsilon40 <= 2^-120, epsilon48 <= 2^-160 and monotone epsilon. These epsilon values bound reference error and are not physical solver tolerances. Do not introduce a new tolerance threshold; a refusal preserves owner and accumulates attempted work.
- Subnormal initial value/time/uncertainty is included or typed-refused; NaN/Inf, overflow, and malformed shape are typed refusals.
- Exact success work deltas, first/intermediate failure deltas, setup/query costs, and positive allocator calibration are retained.
- Independent read-only review is required after implementation.

Independent review basis: `evidence/continuation_a01_unequal_reference/INDEPENDENT_REVIEW_KO.md` and A01 reviewer final response, which classified the existing represented-curve path as `READY_FOR_BOUNDED_REPAIR` while keeping full A01 and broader scopes on HOLD.
