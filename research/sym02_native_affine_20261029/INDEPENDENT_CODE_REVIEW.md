# SYM02 independent scientific/code review

Reviewer: source_contract (separate from native_implementation and parent author).
Date: 2026-10-29.
Reviewed worktree: /workspace/scratch/bb4a9b3ba5de/vigilode_sym02.
Base: PR101 bc5ddbe42b24b1a6364bf653c3e2d6d938ca4087.
Result: PASS_SCOPED_STATIC_REVIEW; blocking findings: 0.

## Boundary and method

This is the single requested independent C9 source review pass. I read the actual implementation, integration tests, lib registration, FastDenseSegments adapter, THEORY.md, THEORY_DECISION.md, CONTRACT.md and the reused directed Interval operations. I compared them with the independent R5/R6 contract in MATH_REVIEW.md.

No native build, test, IVP, benchmark, scientific Python import or CAS was run in this review. A read-only git diff --check passed. Runtime C6/C8 evidence and the parent's independent exact Fraction-from-bits comparison are still required for final unit admission; this static decision is not that execution evidence. No reviewer children were used.

The claim reviewed is conditional scalar-affine correctness with caller-supplied valid lambda/b enclosures and incoming exact-IVP error. Source identifiers bind those caller assertions; they do not prove an arbitrary callback equals the affine equation. This matches the implementation documentation and avoids transferring this result to BASS30.

## Source snapshot

| File | SHA256 at review |
|---|---|
| crates/rodas5p-integrators/src/affine_event_certificate.rs | ebf34a3be111347b05fed6bcf71fb62813149e4dd8376059f77fa92cff2727ce |
| crates/rodas5p-integrators/tests/affine_event_certificate_contracts.rs | d7f2bc6071e8b6b0953047fda1b8a3e664d526c59669b919f71049c61b42c39b |
| crates/rodas5p-integrators/src/lib.rs | ddf3f3819455ae7d6facd8fd0092168378a081f4802510eefe1b594ade1d950a |
| crates/rodas5p-integrators/src/rodas5p_fast_dense.rs | 3beb035176b0ffcea42703f4f8ffc8b5111abf4ad1ea1c0552d65cea249b338c |
| research/sym02_native_affine_20261029/THEORY.md | 1358cd0918561fcaa9cfd933e15060659e10a505c3ef45b77f9e54baf63326b3 |
| research/sym02_native_affine_20261029/THEORY_DECISION.md | 86f406f4e83e604036ac4b0363be30f8a1e6cbd0bbf4516956bd6a56c639a280 |

Line references below apply to this snapshot; formatting-only changes may relocate them.

## Scientific/code findings

### Verified: exact target and physical-time residual

affine_event_certificate.rs certify(), lines 320–345:
- Exact-real stored width is enclosed by directed interval subtraction and must have positive lower endpoint.
- Exact-real H-row/stage sums are enclosed with point products and directed sums.
- All five power coefficients are retained, including a4=-d2.
- Residual is lambda*p+p_theta'/h-b, using the exact-real width enclosure. The derivative and degree-four Bernstein coefficient transform match the independent derivation.
- max mag over Bernstein intervals bounds the whole slab and constant parameter boxes. Independent coefficient/rate dependencies are relaxed only toward wider enclosures.

No sampling claim is used to establish whole-slab coverage.

### Verified: state-error bound and crossing predicates

certify(), lines 347–385:
- lambda.lo=kappa>0 supplies the valid dissipative comparison D^+|e|<=-kappa|e|+delta.
- Both rational envelopes U1 and U2 are enclosed, including the signed numerator in U2; taking max of their upper bounds is sound for both signs of E0-delta/kappa.
- Eall=max(E0,Eh) follows monotonicity of the exact comparison majorant.
- Exact endpoint cancellation p(0)=y0,p(1)=y1 is preserved by using stored endpoints.
- Whole-slab state tube derives from the polynomial Bernstein hull plus Eall.
- The derivative range -lambda*Y+b is enclosed over that tube.
- UniqueCrossing requires opposite strict endpoint signs and strict derivative sign; NoCrossing requires strict whole-slab sign or equal strict endpoint signs with monotonicity. Equality/grazing remains unresolved. I found no path producing a false no-crossing/unique-crossing result under the stated inputs and arithmetic premises.

### Verified: actual query discrepancy remains separate

query(), lines 411–444:
- Physical time membership precedes normalization.
- Exact normalized query is enclosed by directed arithmetic; intersection with [0,1] is justified by physical membership.
- Endpoints use stored exact values; internal Horner intervals retain all quartic terms.
- The interval difference [actual_center]-I_math followed by mag encloses the distance of the supplied finite center from the exact real polynomial, even if the center lies outside I_math.
- Its outward sum with E0/Eh/Eall encloses the original IVP at the query.
- A changed supplied endpoint center receives a nonzero discrepancy radius instead of endpoint-copy zero.

The generic query API certifies a caller-supplied center; native users are explicitly directed to the adapter that obtains the actual source output. This distinction is documented.

### Verified: native snapshot binding and carry

RepresentedAffineSegment, lines 179–225; check_spec/check_segment(), lines 394–403; carry/certify_next(), lines 447–457 and 505–517; FastDenseSegments adapter:
- Manual represented packets and native-accepted packets have distinct provenance.
- Native construction copies stored H, stages, states, times and segment index; all certificate fields are privately owned.
- Query adapter compares every represented input bit and provenance before obtaining source native output.
- Query compares full current model/source/clock/unit/event/epoch/parameter specification.
- Carry requires identical endpoint time bits, endpoint state bits, adjacent index, same origin and unchanged full specification. It forwards Eh as the next E0.
- Source/epoch change and a reset cannot use this carry while preserving its assertion identity; a reset requires a new externally justified incoming bound.

No generic RHS callback or event callback is invoked by the certificate adapter, and no controller/default-route change appears in the diff.

### Verified with stated platform premise: arithmetic guards

check_affine_event_arithmetic(), lines 39–63:
- Nonconstant black_box positive/negative 3/4-ulp sums discriminate common directed IEEE modes; half-ulp sums check ties-even.
- Positive/negative subnormal input and output products distinguish DAZ/FTZ behavior.
- Proof-producing construction/query/carry paths repeat the check; the native adapter also checks before actual query evaluation.
- Non-x86_64 hosts refuse this qualified arithmetic path.
- Production lib retains forbid(unsafe_code). Test-only MXCSR mutation occurs in the integration test crate with restoration.

These probes observe selected arithmetic properties; correct IEEE operations and a stable environment during each synchronous call remain platform premises, as THEORY.md explicitly states. I do not interpret the probes as direct control-register inspection or a proof of all hardware operations.

Overflow or nonfinite intervals propagate errors. Positive underflow may yield conservative intervals crossing zero; the width positivity guard and denominator Interval checks prevent silently using invalid divisions. I found no explicit NaN/Inf recovery path that would turn a refused computation into an admission.

## Review of planned tests

The new integration test source includes meaningful controls for:
- PR100 exact affine example and quartic final coefficient;
- exact-real width distinct from rounded width;
- threshold equality, grazing and endpoint-only evidence;
- source/model/unit/event/epoch mutation and incorrect carry adjacency;
- interior normalization underflow with physical-time membership;
- parameter boxes and E0>0;
- subnormal inputs and overflow refusal;
- actual native accepted scalar segments plus dimension/index refusal;
- nonnearest modes and FTZ/DAZ for construction/query.

Tests against exp() are marked diagnostic; they are not used as the independent exact algebraic oracle. The parent's planned Fraction-from-bits verifier should check the returned intervals for the actual emitted packets, widths, all five coefficients, residual Bernstein coefficients, query polynomial and radius, and parameter-box/carry coverage. These outcomes are not yet observed in this review.

## Nonblocking observation

The two-envelope upper can approach 2*delta/kappa on long slabs with E0=0, while max(E0,delta/kappa) is independently a sound invariant cap. Adding this cap or the simpler positive-arithmetic envelope from MATH_REVIEW.md would improve sharpness, but it is not required for correctness or this unit's scoped acceptance. No retuning is requested solely for that optional improvement.

## Disposition

Blocking findings: 0.
Scientific/static implementation match: PASS_SCOPED_STATIC_REVIEW.
Execution/independent arithmetic output verification: NOT_RUN by this reviewer, pending root C6/C8 evidence.
Core VigilODE count remains 7/23; central REI 28/88 remains separate; auxiliary SYM02 is not marked complete by this review alone.

