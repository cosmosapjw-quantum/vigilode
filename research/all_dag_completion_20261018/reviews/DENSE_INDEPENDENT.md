# A05/A11 independent mathematical and code review

Reviewer: `/root/dag_a12_independent/dense_independent`. The reviewer did not author or edit the implementation. Source hashes and evidence identity are in `DENSE_INDEPENDENT.json`.

**Decision: PASS_SCOPED for A05 and A11, with source-bound native evidence inspected.** No blocking mathematical or implementation defect was found in the reviewed source. This is not a general production or speed promotion. The existing published model-specific mathematics was inspected once at the interface where the new driver uses it; unchanged experiment campaigns were not rerun.

## Endpoint and clock ownership

The suspected endpoint-transfer problem does not occur. The represented real quartic is

\[
q(\theta)=(1-\theta)y_0+\theta\left[y_1+(1-\theta)(d_0+\theta(d_1+\theta d_2))\right].
\]

Thus its real endpoint is exactly the stored binary64 vector \(y_1\), not a fresh floating-point evaluation of a power-coefficient sum. `Rodas5pQuarticSegment::from_step` owns copies of `step.y_old`, `step.y_new` and represented dense coefficients. Both point and interval endpoint evaluators return the stored endpoint. `integrate_verified_residual` then commits that same `step.y_new` with the endpoint radius. There is no omitted recentering displacement in inheritance.

The represented clock difference must be admitted as an exact positive normal binary64. Candidate stage calculations may have used the original `h`, but the independent residual is taken against the actual represented curve and clock. Interior output normalization is enclosed by directed interval operations, and the returned ordinary binary64 center is enclosed again before the L2 displacement is added.

## Continuous tube and transaction

The provider encloses \(q\) and \(q'\) over the entire closed interval. Its residual bound is based on \(q'-f(q)\); its logarithmic norm is evaluated on the enlarged candidate-centered tube, not merely at samples or on the curve alone. Since the coordinate box enlarged by \(r\) contains the L2 radius-\(r\) ball, it contains the segment joining the candidate and every admissible solution point. Therefore

\[
D^+\|y-q\|_2\leq \mu\|y-q\|_2+\rho.
\]

The signed exponential transport uses outward bounds. For constant \(\mu,\rho\), its comparison solution is monotone toward its equilibrium, so its interval maximum is the larger of the initial and final bound. Closing that maximum inside the proposed tube establishes the usual continuous bootstrap. The endpoint is inherited in the next step without resetting accumulated error.

The generic driver constructs a candidate, verifies the continuous tube and prepares every due requested output before changing committed state, radius, output cursor or accepted segment list. Rejection retains both solver and verification work. No speculative factor/recycle cache is used in this path. Ignoring the embedded candidate `accepted` flag does not invalidate this stronger independent residual acceptance test: the candidate polynomial is allowed to be arbitrary, provided the tube and unchanged absolute output budget are verified.

## Gray–Scott full symmetric part

For each cell the reaction Jacobian is

\[
R=\begin{pmatrix}-v^2-F&-2uv\\v^2&2uv-(F+k)\end{pmatrix}.
\]

Its symmetric offdiagonal is \((v^2-2uv)/2\); this is retained in the implementation. The outward trace-plus-square-root formula bounds the largest eigenvalue of the entire 2×2 symmetric block. The periodic five-point diffusion operator is symmetric negative semidefinite, so dropping it gives a valid upper bound for the complete symmetric Jacobian. The grid-size-two duplicate periodic neighbors contribute with multiplicity and preserve this property. Taking the maximum over cellwise interval block bounds is therefore valid in the unweighted Euclidean norm and has linear cost in the cell count.

The new `BoundGrayScottProblem` owns the generated point callbacks, grid and represented constants; external callers cannot substitute an unrelated callback or arbitrary logarithmic bound. The interval RHS uses the same periodic stencil and represented scalar constants. The private `VerifiedModelProvider` implementation connects this bound to the actual inherited trajectory tube. This closes the previous distinction between a supplied-box result and a full verified trajectory capability.

## Accuracy and cost scope

The continuous error bound is for the real represented curve. A completed run checks the rounded-center radii at its requested physical output times against the original `atol`. A later standalone `segment.evaluate(t)` query returns an outward radius but does not automatically enforce the original run budget; its caller must compare the returned radius with the desired budget. A11 documentation now states this distinction.

The common driver supports immutable PR101, collective and Gray–Scott factories. It is generic over these capabilities, not arbitrary user callbacks. Pure-relative verification remains explicitly unsupported. The natural interval residual may be conservative, particularly for long or stiff trajectories; validity does not imply efficient execution on all model sizes.

The fused Gray–Scott stencil eliminates two temporary length-C Laplacian vectors per RHS callback and two per JVP callback while preserving the scalar arithmetic order. This is a source-level allocation/work reduction. Extra integer indexing means elapsed speedup still requires measurement. The shared generic verifier has no speed promotion from these tests.

## Evidence

The reviewed A05 final focused log contains 7 passed tests and no failures. The unchanged original PR101 contract has 101 outputs, 492 accepted and 292 rejected attempts, with sampled maximum absolute error 8.382183835919932e-15. Tests also exercise nonlinear collective motion, nonzero damping, large-offset affine interval cancellation, zero-span input validation, typed refusal of unbound callbacks and failed-transaction output retention.

A11 source tests cover nonzero inherited initial radius and an exact homogeneous trajectory, a nonlinear nonuniform trajectory compared with two independently coded RK4 step sizes, interval RHS enclosure, fused-callback equality, finite-difference JVP, actual homogeneous-mode offdiagonal Rayleigh behavior, linear work scaling, typed malformed/overflow refusals and failed-transaction prefixes. The initial A11 build did not reach those tests because of a concurrent unrelated `actual_k_homotopy` owned/reference type mismatch; that historical failure is retained. After correction, the actual final native log contains all 7 new A11 tests and 3 changed-helper regressions passing. The exact homogeneous trajectory used 34 accepted and 21 rejected attempts. The nonlinear run returned 11 outputs after 6 accepted and 2 rejected attempts; its maximum discrepancy against the finer independent RK4 was 8.938889858629839e-15, with two-grid RK4 spread 9.506558148265855e-15. These are discriminating numerical checks, not independent interval proof authority. The immutable-owner compile-fail test and native example also passed. No implementation edit was made by the reviewer, and no unchanged campaign was rerun.
