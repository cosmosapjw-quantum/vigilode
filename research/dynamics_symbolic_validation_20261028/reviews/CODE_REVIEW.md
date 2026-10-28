# Independent C9 code and science review

Date: 2026-10-28. Reviewer: /root/attachment_theory. Baseline: 87465835d9d21298d235917f036580b0b53ab645. This is one combined source/mathematics/campaign review. No native build, test or benchmark was executed by this reviewer. The source is a working diff against the pinned baseline; native results and final source hashes must be bound by the parent execution receipt.

## Verdict

The production algorithms are mathematically sound for the declared contracts. Approve the live-support trace implementation and guarded buffer transfer for scoped native validation. Three validation/reporting findings below need closure before claiming measured matched-accuracy performance. None establishes a solver-wide speedup or closes production qualification.

Reviewed initial file SHA-256:
- trace_quotient.rs: 7a6d48dca003f42b7b150b9e7a1e2de4a46c38bf4635fd9c58b7d12fdc027e99
- switching.rs: 619faddc275f3e2787a05ae78b3676ddb4dbee502d5886d1261fe46a38c8eddd
- examples/trace_support_campaign.rs: 9ba12f5a41b8639efababa61fcf27303c40c4e527d8ae0a4febd162604d7ef0f

## Findings and required disposition

| ID | Severity/scope | Evidence and effect | Minimal correction/status |
|---|---|---|---|
| C9-01 | Validation, matched accuracy | trace campaign case() checks each interval contains the exact target; it does not assert/persist equal width or a common accuracy cap. The corpus is small dyadic and is designed to return exact singleton intervals, so this is easy to test directly. Mere containment does not specify matched precision for arbitrary future cases. | Before execution, require exact singleton endpoints for both arms in require_baseline cases, or an explicit common width gate; record the gate. Parent accepted this correction. |
| C9-02 | Regression coverage | New sparse query tests demonstrate avoiding unused S6 overflow. Existing required-overflow tests call field APIs, not the changed polynomial query. The source correctly still evaluates live S6, but this boundary should be guarded directly. | Add a nonzero S6 coefficient at huge finite a; require a refusal with requests_rejected/work charged. No tolerance relaxation. |
| C9-03 | Performance interpretation | Cargo measurement profile has LTO=false. The frozen eager function is local to the example; the changed provider is an exported function from another compilation unit. A common binary does not itself equalize inlining opportunities. | Mark the frozen eager API boundary inline(never), or explicitly disclose the compilation-boundary difference and restrict interpretation to measured compiled calls. No algorithmic correctness impact. |

The parent already corrected the controller benchmark warmup and exclusive output creation: one warmup per arm/case is excluded from rows, and OpenOptions::create_new(true) prevents overwriting an existing timing receipt. These are confirmed in the reviewed source.

## Trace mathematics and input semantics

The fixed monic quartic quotient retains rank four and algebraic multiplicity. Trace linearity gives Tr(sum c_k z^k)=sum c_k S_k, with the exact Newton sums already admitted by PR97. The new supported_moments function constructs only the dependency union of live coefficients. S0=4 and S3=-3 need no parameter arithmetic; S1=0 can be omitted identically; S2=4a and S5=-10a need no a squared; S4 and S6 share a squared only when requested.

Every supplied coefficient is passed through Interval::new before pruning. NaN, infinity and reversed intervals therefore fail even at mathematically dead S1. An interval containing zero but different from [0,0] remains live. Signed zeros are safely treated as the exact zero real. The empty or all-zero polynomial returns exact zero. Input semantics remain checked by the existing request wrapper. Uncertainty, rejected-request accounting and selected-real/real-only refusal are preserved.

Interval addition/multiplication is outward under the unchanged directed-arithmetic contract. Omitting exact-zero terms and starting the accumulator at the first live term preserves enclosure validity and can reduce inflation. No assertion is made that arbitrary cancellations between overflowing active terms will be recovered. Full-moment and field APIs continue to request their full dependency set.

The extreme references are exact binary64 numbers: 4*2^600=2^602; S3=-3; 4*(2^400)^2=2^802; and -10*2^600. These test the intended extension of the finite supported-query domain, not a changed polynomial target. Using finite f64::MAX for unused b deliberately verifies that no unrelated product is evaluated.

## Exact oracle and multiplicities

The test and campaign oracle use the integer matrix 16M, where M is multiplication by z in basis 1,z,z^2,z^3. At a=aq/4,b=bq/4, its top-right entry is 4bq-aq^2 and its row-three top-right term is 8aq; these match the quartic companion matrix. Dividing Tr((16M)^k) by 16^k recovers S_k. The selected small dyadic inputs keep these powers, coefficients, products and sums exactly representable. The oracle is a different algebraic representation from the Newton-sum implementation.

All 128 support masks at four parameter pairs cover every subset of moment dependencies, including the triple root and the double-root plus complex-pair case. The explicit cubic trace -3 at a=b=3/4 rejects the distinct-root value -13/4, protecting the multiplicity contract. These are intended executable checks; this source review does not mark them as having run.

## Trace campaign target and cost

Both arms receive identical parameter/coefficient construction, target identity, coefficient slice and represented bits. The frozen eager body follows the pinned eager operation order, includes its single request boundary and counters, and retains old refusals separately. Large finite-domain recovery cases are outside the timed corpus; an old refusal cannot become a spurious speedup denominator.

The timing loop includes parameter/coefficient conversion, input validation, evaluation, counters and output black_box. Workload creation is shared and separately timed; serialization is outside query timing. Seven seeded ABBA/BAAB repetitions and raw blocks retain favorable and unfavorable samples. Dense degree-six and mixed controls expose support-pruning overhead instead of reporting only sparse wins. The common-accuracy gate C9-01 is still needed in the prospective record.

This campaign measures complete polynomial query calls. It does not run an ODE, certify a continuous trajectory or benchmark physical branch recovery. The original 1e-8 trajectory cap is not silently replaced by a new query-timing metric.

## Controller refinement and test language

The only production semantic change is ownership transfer of pending_state into baseline_state during successful guarded protected-reference update. prepare copied the exact accepted-start state first. Pinned adaptive.rs invokes prepare once per attempt; each observe call is followed by continue, break or the loop tail. The next prepare clears/overwrites pending_state before its next reference-promotion read.

Thus equality of baseline and all control fields after observe implies equality of both snapshots after the next prepare. Induction over legal guarded words proves unchanged reference, routing and physical numerical state. The old baseline buffer becomes scratch. The change deliberately removes the second n-coordinate copy from telemetry; callback/work scores, guards, epochs, acceptance, cooldown and precision remain untouched.

The frozen LegacySwitchState body preserves the prior observe logic. Complete-word tests compare all serialized receipt fields except the intentionally changed copy count, baseline bits, pending bits after prepare and skip state. They include dimension changes, signed zero, epoch changes, h changes, rejected attempts and terminal words. Terminal symbols stop a word. This supports the guarded control-system refinement; it does not establish a physical ODE Markov partition or topological conjugacy.

Controller timings cover prepare plus observe from empty owned buffers, with guarded protected, candidate-only and unguarded controls. Allocations and decision-word collection are inside timing for both arms. An entire n-coordinate copy is removed per promoted guarded reference. Initial allocation timing differs between arms, so including both prepare/observe is necessary and correctly implemented. The benchmark is a controller component workload with synthetic receipts and no RHS solve; wall-time results must retain that scope.

## Theory-before-code provenance

The parent supplied the actual admission markers: F1 followup_task explicitly granted the linear-trace/live-support mathematics with retained input validation before implementation; the dynamics R1–R10 proof was read and admitted before its coding assignment. Supplementary attachment report consolidation overlapped coding. The honest process claim is therefore source-connected mathematical lemmas and parent admission before implementation, followed by supplemental report finalization. Do not claim every final theory report file had been completed before the first edit.

The preregistration currently says parent admission precedes native implementation. That statement is consistent with the reported admission markers; the final workflow should record those markers and the consolidation overlap explicitly.

## Remaining acceptance boundary

Native tests, exact exported-bit Givens oracle, performance execution, final-tree identity and independent evidence replay remain parent-owned. Preserve Q-LEDGER and missing-authority history, original 23-node/34-edge DAG, 8 suites, 73 paths, 10 interactions and cumulative correction counters. This C9 review does not grant a new retry budget, close N04/N05/N06 or establish general production readiness.

## Finding closure: prospective source corrections inspected

The same reviewer inspected only the changed finding locations after parent correction, before the scientific campaign:

- C9-01 CLOSED: case() now requires support_singleton, and for each timed baseline-required input also baseline_pass and baseline_singleton. Each arm records singleton_equals_exact. The report names the exact-singleton companion-trace gate. All timed workloads reuse these validated inputs; huge recovery cases remain separate.
- C9-02 CLOSED: required_sixth_moment_overflow_is_rejected_and_charged calls the changed polynomial API with coefficient[6]=1 and a=f64::MAX. It checks attempted/rejected/completed counts, query/moment charges and positive arithmetic work. This closes the source-level test omission; test execution is still pending at this review boundary.
- C9-03 CLOSED WITH DISCLOSURE: eager_polynomial_trace and support_polynomial_trace wrappers are inline(never). The candidate wrapper keeps the public cross-crate call, and campaign metadata explicitly discloses this. The measured comparison retains complete public-call overhead; it is not claimed as a pure arithmetic instruction benchmark.

Final source-read hashes for this review: trace_quotient.rs b1f116d88d073ad22db6a9cca38e0ab44840f10604d57f5e401a72996a28ee43; switching.rs 619faddc275f3e2787a05ae78b3676ddb4dbee502d5886d1261fe46a38c8eddd; trace_support_campaign.rs ecfc8c6f27424015d37fd6a5aa121608cfb13e1661342bf417239068fe4405b8.

Final C9 decision: ACCEPT_SCOPED_FOR_NATIVE_VALIDATION. No open source/mathematics/campaign-design blocker remains within this diff. Scientific execution results must still be assessed against the recorded criteria; an unexpected native failure cannot be reclassified as passing by this review.

