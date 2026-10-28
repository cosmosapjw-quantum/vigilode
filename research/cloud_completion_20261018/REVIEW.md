# Independent review of the native completion

Review date: 2026-10-18. Baseline: `c513f478f952068b91e3da368fb2cb156d942b28`. Lead reviewer: `/root/independent_review`, separate from the implementation authors. Machine-readable decisions and final source/evidence SHA-256 bindings are in `REVIEW.json`.

The new work supports **scoped promotion of the enclosed Leja action, requested-time packet trajectory, sealed PR101 continuous verifier, and outward affine-prefix kernel**. The collective structured backend is accepted as a complete numerical implementation with linear work per fixed-stage attempt. The nonlinear homotopy result is accepted as conditional theory; its native nonlinear client is not implemented. These decisions do not establish unrestricted production qualification or measured speed superiority for the repository as a whole.

## Scope decisions

| Unit | Decision | Exact admitted scope | Outside this decision |
|---|---|---|---|
| Explicit enclosed Leja API | PROMOTE_SCOPED | Finite successful fused Euclidean action bound for the stored, exactly symmetric, dense, Gershgorin-verified operator and stored inputs; existing directed-arithmetic premises | ODE trajectory error, model/input uncertainty, individual-column error, declared-only spectral evidence |
| Weighted packet chaining | PROMOTE_SCOPED | Successful requested-time trajectory for the admitted undamped, zero-carrier native model, original initial disks, exact represented window durations, and unchanged absolute output budgets | Arbitrary adaptive meshes, damping, physical-carrier extension, arbitrary callback models |
| Sealed PR101 provider and driver | PROMOTE_SCOPED | Same-curve continuous defect, inherited global error, output rounding and verify-before-commit for the immutable scalar identity-mass target on [0,1] | Generic `OdeProblem` callbacks, pure-relative tolerance, guaranteed completion for arbitrary coefficients |
| Collective structured RODAS5P | PROMOTE_SCOPED_NUMERICAL_IMPLEMENTATION | All eight native K-coordinate stages, one frozen factorization, eight O(N) solves, private reusable workspace and adaptive numerical output | Certified global trajectory error; empirical allocation or speed claims |
| Outward affine prefix | PROMOTE_SCOPED | Caller-certified nonnegative affine error maps; preallocated `evaluate` has O(N) work and O(log N) ideal fork-join span | Certification of caller-supplied gains; measured hardware speed; constructor/allocation span |
| Homotopy parallel shooting | PROMOTE_CONDITIONAL_THEORY | Closed invariant product balls, uniform contraction q<1, certified map errors and explicit cost inequality | A general parallel nonlinear driver or an unconditional speed guarantee |

All floating-point enclosure decisions retain the existing directed-primitive contracts, including their binary64/gradual-underflow assumptions. They are source-level mathematical reviews supplemented by native and exact-rational evidence, not machine-checked proofs.

## Independent reasoning

### Leja authority boundary

The lead reviewer checked the interpolation remainder, coefficient enclosures, local recurrence defect propagation, suffix indices, and complete output replay. The coefficient-times-basis cross term is included by multiplying each propagated basis defect by `abs(d_mid)+d_radius`. The final interval replay covers the represented coefficient/basis sums, column scales and returned fused binary64 vector, including cancellation and subnormal values.

The Opitz matrix representation gives the required confluent divided differences. Its shifted matrix is nonnegative; interval Taylor terms and the geometric matrix-norm tail enclose its exponential. Intersecting a lower term with zero is valid because the exact term is nonnegative. The normalized spectral support is checked after choosing the actual stored center and scale. Node optimality is not a premise.

Only `leja_phi_action_enclosed_with_workspace` sets the private certification switch. Historical candidate wrappers remain `EstimateOnly`; the reviewed report is committed only after successful finite replay. `Certified` means that the numerical bound is valid. The separate `converged` flag and requested budget still determine whether that bound satisfies a caller's accuracy request. An unmet budget therefore cannot be inferred from status alone. The certificate applies to the fused sum, not independently to every column.

The positive operation reductions are valid in their stated subsets: prefix table construction reduces 128q(q+1)/2 products to 128q; the interpolation-factor recurrence and coefficient accumulation avoid repeated prefix work. Suffix construction, dense matrix replay, other setup and output work remain charged separately. None of these counts establishes a wall-clock speed ratio.

### Packet initial uncertainty and window transfer

The lead reviewer independently rederived the weighted-ball argument; `REVIEW_PACKET.md` records a second independent review. For exact stored positive weights, the center energy and incoming centered weighted radius enclose the entire original initial set. The logarithmic-norm bound uses the actual weight sum S, rather than silently replacing it by one. The per-window continuous residual proof is reused from the baseline's frozen represented-curve contract.

The new carry radius includes displacement between the exact curve and the returned binary64 centers before becoming the next window's initial radius. This closes the induction across windows. Mean and mode radii include their own output displacements. Direct weighted-ball transfer removes the artificial conversion factor: converting radius r to component disks r/sqrt(w_j) and back gives sqrt(N)r. Repeating K such conversions can introduce N^(K/2); the new representation avoids this unnecessary inflation.

The independent Fraction oracle complexifies the real/imaginary equations, establishes an explicit Picard contraction disk, then uses a Cauchy coefficient tail. Its 120 rational containment checks include unequal frequencies, nonlinear signs, perturbed initial states and a stored weight sum slightly different from one. This finite evidence supplements the enclosure proof. The driver refuses inexact mesh differences, unsupported models and exceeded original output tolerances.

### Continuous dense verification and parallel transport

`REVIEW_DENSE_TRANSPORT.md` contains the delegated independent derivations and corrective findings. `REVIEW_DENSE.md` adds a separate author-independent cross-review and a new signed/expanding parameter test.

For PR101, the interval power representation describes the same stored quartic used by the candidate. The verifier forms the residual polynomial before taking coefficient magnitudes, retaining stiff cancellation. Rigorous sine/cosine polynomial tails cover the mathematical right-hand side. A signed logarithmic-norm comparison closes the continuous tube, and endpoint ownership carries the resulting radius to the next segment. Requested-output evaluation adds the represented polynomial's output-rounding displacement. Candidate rejection and verification failure cannot commit an unverified segment.

The symmetric square convolution is also a proved concrete cost reduction: 13 diagonal contributions and 78 off-diagonal contributions require 351 primitive calls instead of 507. It encloses the same nonlinear polynomial and saves 156 calls per successful verifier invocation. The recorded 784 attempts save 122,304 calls, lowering structural verification work from 696,398 to 574,094 with unchanged accepted/rejected counts and output discrepancy. This is an operation-count guarantee, not a timing claim.

The budget scheduler reserves future headroom without enlarging the original tolerance. It is a completion heuristic, not part of the error theorem. A reviewer-proposed nonzero expanding Riccati orbit exposed starvation in the first schedule; the corrected schedule completes that case. The stiff witness now requires more steps than the earlier stiff-only run, which is recorded as a tradeoff rather than a speed improvement.

For the affine kernel, nonnegative monotonicity justifies replacing each exact subtree map by its outward majorant; floating-point associativity is not assumed. The corrected preallocated recursive implementation has 3N−2 compositions and N bound evaluations for N>0. Allocation and initialization remain separate O(N) serial setup. The conditional homotopy theorem follows from a product-norm contraction and includes the q=0 case, continuation bound, finite iteration bound, numerical-error floor and a full ideal-cost inequality. It establishes a constructive route when its premises are verified; it does not establish those premises for an arbitrary nonlinear solver.

### Structured stages and honest measurement

`REVIEW_STRUCTURED.md` verifies that every stage matches the native autonomous K-coordinate recurrence, including its frozen Jacobian coupling. Refactor failure invalidates solves, residual witnesses and determinant metadata. Output is transactional; adaptive rejection retains the accepted state. The fixed eight-stage method has O(N) workspace and O(N) setup/attempt work, with all stage combinations and factorization checks included.

The allocation counter's library-allocation positive control failed. Consequently, raw allocator counts are not accepted as allocation evidence. The final runner emits null authoritative allocation values when calibration fails and retains raw values only as diagnostics. Passing allocation assertions in other test targets likewise remain raw outcomes until the relevant library-allocation positive control passes. No zero-allocation or measured-speed promotion is made from this environment. This is an evidence limitation, not a failure of the algebraic linear-work proof.

## Findings and closure

| ID | Finding | Resolution / present status |
|---|---|---|
| CR-P01 | Dyadic mesh wording did not imply exactly representable differences | Corrected to suitably scaled power-of-two meshes with explicitly checked differences; native guard retained |
| CR-L01 | Legacy Leja comments/reason implied every total remained estimate-only or awaited review | Qualified legacy API wording; explicit enclosed authority remains separate |
| DTR01 | Initial affine implementation did not justify its claimed native logarithmic span | Replaced with preallocated recursive disjoint sweeps; constructor span separated |
| DTR02 | Homotopy iteration formula omitted q=0 and executable residual uncertainty | Explicit boundary branch and numerical-error term added |
| DTR03 | Extra initial bound evaluation contradicted the exact operation count | Direct validation preserves the claimed N evaluations |
| DTR04 | Expanding nonlinear orbit exhausted future global budget | Preserved failure record; corrected headroom schedule and same-tolerance passing regression |
| DTR05 | Initial oracle label lacked a separate exact-rational oracle | Separate Fraction enclosure script and whole-interval fixture added |
| DR01 | Native candidate disposition could be counted again by the outer verified driver | Restore only disposition counters, retain all work, record one outer decision |
| SR01 | Failed mutable refactor left stale determinant metadata readable | Validity-checked accessor and failure-invalidation regression |
| SR02 | Allocator instrumentation missed known library allocations | Closed by scope correction: authoritative fields are null when uncalibrated; measurement remains unestablished; raw diagnostics retained |

## Evidence and remaining development

Final integration edits were inspected separately: moving the packet degree tests to the end of their module, retaining explicit proof arguments with a targeted lint annotation, iterating the identical stored weight slice in the same order, and marking unconnected generic output staging as intentionally unused. These changes do not alter the reviewed formulas or authority boundaries; this review binds the final bytes. The final two test-only cleanups were also inspected: a local lint allowance retains the independent indexed 4×4 Jacobi oracle and its arithmetic order, and replacing `matches!(call, Ok(_))` with `call.is_ok()` preserves the same success assertion. No numeric assertion or refusal check was removed. Earlier cross-review receipts may bind their pre-format bytes.

The lead review reran no historical native campaign. It independently checked the new mathematics and API boundaries, inspected source-bound execution/oracle evidence, and coordinated reviews separated from each implementation author. The dense/transport reviewer independently recomputed 214 rational affine prefixes plus six initials. Native test counts and final source bindings are recorded in `REVIEW.json`; failures that motivated repairs are retained alongside passing receipts.

The most useful next implementation steps are: preserve the explicit Leja budget/status distinction at any future integrator admission boundary; extend packet transport only after deriving and implementing the new damped/carrier premises; recover stable-regime PR101 efficiency with a certified future nonexpansive envelope; attach real model-specific continuous providers to the generic driver; and implement the nonlinear homotopy client only after certifying its invariant balls and contraction constants. A release benchmark should compare full costs at matched achieved accuracy and use calibrated allocation instrumentation. These are concrete remaining development units, not conditions retroactively imposed on the scoped approvals above.
