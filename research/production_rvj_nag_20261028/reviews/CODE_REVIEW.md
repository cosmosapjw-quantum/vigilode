# C9: one independent review of the new native RVJ implementation

Reviewer task: `/root/rvj_pr_audit`. Date: 2026-10-28. This is a single requested source review of the **new** native closure/trace implementations following parent theory admission. It is not a second audit of PR96 or a review of a previous reviewer. Runtime source remained owned by its implementation agents; this reviewer only reported concrete findings and inspected the bounded repairs.

## Decision

**The completed source is suitable for the registered bounded execution after source publication.** No mathematical false-certificate path was established within the admitted finite cyclic/trace model and stated arithmetic assumptions. Two cyclic measurement/evidence issues and one trace unnecessary-safe-refusal issue were found and repaired before scientific execution. All three targeted repairs have been read back; the JSON companion records their final disposition.

This decision does not assert that the code compiled, that tests passed, or that a numerical speedup was achieved. The reviewer has performed no native scientific run. The independent exact oracle source has only passed Python syntax compilation. Actual C6/C7 tests, campaigns and all-slab exact validation remain the parent's separately published execution gates.

## Inspected surface

- `crates/vigilode-coupled/src/cyclic_closure.rs`: complete first implementation, including six staged native tests.
- `crates/vigilode-coupled/examples/rvj_closure_campaign.rs`: complete campaign and the two targeted prepublication repairs.
- `crates/vigilode-coupled/src/trace_quotient.rs`: complete first implementation, including eight initial staged native tests, followed by the bounded degree-zero/one repair and its ninth staged test.
- `crates/vigilode-coupled/examples/rvj_trace_campaign.rs`: complete campaign, semantic controls, diagnostics and timing paths.

The source hash snapshot is in `CODE_REVIEW.json`. Later formatting changes do not establish new numerical evidence; any semantic change after this snapshot needs its own explicitly bounded assessment. The parent owns crate exports, compilation, source publication, run receipts and correction counters.

## Findings

### C9-01: private prepared-workspace destruction outside the cyclic total timer

**Classification:** measurement correctness; addressed before publication.

In the first `rvj_closure_campaign::run_once`, `total_seconds` was captured while the internal `prepared` vector was still live. Its interval/vector destruction then occurred during return, outside a timer described as complete online work. The historical arm owns 2N prepared endpoints while the direct arm owns N+1, so excluding this cost is not necessarily neutral.

The implementation owner moved `drop(prepared)` before the total snapshot and labelled the cleanup phase. The targeted readback confirms the private workspace destruction is now included. Returned states and slab receipts remain consumer-owned outputs whose subsequent lifetime is explicitly outside the producer timer. This is a defined timing boundary, not a claim to include process startup, compilation or JSON export.

### C9-02: campaign numerical errors discarded the local partial work record

**Classification:** failed-work evidence; addressed before publication.

The native API receives a caller-owned mutable `CyclicWork`, so a returned error preserves attempted arithmetic. The initial campaign wrapper instead owned this counter locally and returned `Err` through `?`; it dropped the record without writing it. Thus the API property did not carry through to the benchmark failure path.

The implementation owner moved the fallible work into an inner closure while keeping counts and phase outside. A returned error now emits structured stderr containing arm, n, grid, phase, repeat/diagnostic flag, actual partial counters, elapsed attempt time and error before propagating failure. The parent command recorder must preserve stderr. The targeted source readback confirms this behavior. It does not claim coverage for process aborts or hardware failure.

### C9-03: low-degree polynomial trace unnecessarily evaluated unused high moments

**Classification:** nonblocking robustness/performance issue for the initial finite cohort; safe refusal; addressed before publication.

The initial `quartic_polynomial_trace` validated coefficients but then called `all_moments` even for a constant polynomial. At finite a=`f64::MAX`, b=0 and coefficient[0]=1, unused a² arithmetic overflows and rejects the query although its mathematical target is exactly Tr(1)=4. The fixed `QuarticInvariant::One` already avoids that work. No incorrect finite certificate is returned.

The implementation owner added the bounded S0=4/S1=0 branch for degree-zero/one requests. Targeted readback confirms that **every supplied coefficient is validated before the shortcut**, empty input remains zero, `len<=2` returns the outward product4*c0, and larger requests retain the previous implementation. The new staged test uses finite a=`f64::MAX`, c0=1 and finite c1=`f64::MAX`, requires exact[4,4] with zero moment batches, and rejects malformed c1 even though S1=0. The unused example import was also removed. No new full review or scientific execution was performed for this readback. This fix remains within the same parent-controlled prepublication correction episode and does not alter the campaign schema or field-operation counter contracts.

## Mathematical and implementation checks

### Cyclic path, original equations and outward arithmetic

`PreparedEndpoint` is built only through validated finite states in the fixed X/Q boxes. It retains private represented state bits, dimension, model epoch, outward exact-field slopes, optional original-G enclosures and the scalar circuit denominator. The interval cyclic recurrence solves the differentiated **original** equations; it checks a positive denominator before division. The binary64 candidate uses a distinct unverified candidate path and must pass subsequent original-equation certification.

The direct path controls enclose the cubic made from **exact** original slopes at the represented endpoints. Consequently the slow residual's two endpoint coefficients are mathematically zero, and the algebraic residual has beta1=beta0 and beta5=beta6 even for a nonzero original residual. Replacing those coefficients by the corresponding proved identities is valid for this path definition; it would not be justified for arbitrary rounded-slope dense output.

The inspected degree elevation and square-product weights match the admitted exact formulas. All non-dyadic fractions, including1/3,2/3,1/5,1/10,1/20,6/7 and8/7, are formed by outward interval division or equivalent enclosed operations. The factors3/4,7/32 and25/32 are exactly represented dyadics. The original slow and algebraic residual bounds are distinct; delta=Df+Dg/7 is propagated in the original physical norm.

The tight radius uses **both** rational expressions and their maximum. It does not use the invalid forcing shortcut `(E+h*delta)/(1+lambda*h)` alone. Whole-slab and endpoint radii are separate, and the algebraic/root transfer includes the actually returned whole bound. Nonfinite incoming radius, denominator failure, arithmetic overflow or unsupported input returns an error.

Both historical and direct kernels check the whole polynomial control hull in X/Q. This suffices together with the admitted invariant-domain/unique-root theorem for the original exact trajectory; it is not merely an endpoint-membership test. `enclose_dense` uses interval de Casteljau on the same exact-field controls and returns exact represented endpoints at theta0/1. It does not silently produce a different rounded-slope interpolant.

### Identity and represented clocks

The model is fixed and autonomous. Prepared slopes need not be keyed by h, but state bits, dimension and epoch must match. `reuse_endpoint` checks exact bits and epoch, and the certificate validates both prepared endpoints against the immutable model. The fixed model/branch/norm identity is not inferred from approximate equality.

`DyadicGrid` requires a power-of-two N≤2^53, so k, k/N and its endpoint labels are exactly representable. Private `SlabClock` fields prevent public forging; `checked_slab` compares t0, t1 and h separately. The module explicitly leaves the incoming error premise and continuous chaining to the caller. A mutable public receipt is not used as an unforgeable admission capability in this implementation.

### Fair cyclic cost comparison and work

All arms generate the same compiled RK4 candidate, and the campaign checks all endpoint bits across warmups, timed runs and the untimed diagnostic pass. Baseline preparation is slopes-only with2N endpoints; it does not pay the direct arm's otherwise unused endpoint-G preparation. The direct arm prepares N+1 endpoints and checks N−1 reuses. Historical convolution/transform counters and direct component counters correspond to the inspected loops.

The same interval provider serves both arms. This is a same-language, same-source comparison, not a comparison against attachment timings from another environment. Rotated arm order and seven retained samples permit a bounded descriptive comparison; they do not establish a universal speedup. Coarse-grid cap failures are emitted as data. Exact-cap admission uses an outward multiplication by100000000, so the rounded `1e-8` literal does not enlarge the requested mathematical cap.

### Trace semantics, overflow and polynomial field

`QuarticParameters` revalidates incoming intervals even though the underlying interval type has public fields. The public query boundaries require explicit semantics; selected-real and real-only requests are rejected before query arithmetic. The trace includes algebraic multiplicity, with S0=4 at the triple-root case. Invariant formulas are consequences of the original quotient equations, not a recovery of a selected branch.

The shared T12 field exactly expands to the original S2/S4/S6 combination. All fixed coefficients in the field are dyadic. Interval operations enclose parameter boxes; binary64 values are explicitly candidate evaluations. Overflow returns an error with caller-owned partial counters. The request dispatcher uses checked counter increments; its failure semantics are distinct from arithmetic success. No root enumeration, Jacobian, JVP/AD or Newton path is present.

The direct field comparator evaluates only S2/S4/S6 rather than deliberately computing unused S5. The shared field has8 multiplications/7 additions versus direct13 multiplications/9 additions in the declared scalar provider model. These are source-operation counts, not hardware FLOPs or a timing prediction. Both candidate trajectory routes use the same direct-moment forcing, whose work is charged on every RHS call.

### Trace path and measurement scope

The finite dyadic grid makes the prescribed p and p' candidate control values exactly representable before the subsequent rounded field evaluation. The candidate integrator checks nonfinite final states; physical dense-path/domain/accuracy admission remains the external exact original-field oracle's responsibility. Native node diagnostics and point interval field widths are correctly labelled as such.

The trace campaign retains explicit failed trajectories and failed timing blocks with partial work. Its four phases use the same retained represented field dataset for both routes; candidate-trajectory timing also includes allocation/destruction of each temporary trajectory. It does not contain a root-enumeration timing comparator and does not imply otherwise.

The exact oracle constructs the entire original controlled Hermite residual and a separate quintic reference difference for every exported slab. This is currently an **external rational whole-path certificate**, and its actual cost must be included whenever an integration-plus-certification total is claimed. The native API itself currently certifies point/box observables and fields. This scope boundary is accurately present in source and schema.

## Validation pending after this source review

The registered execution should run the staged native tests, the finite two campaigns and the independent original-equation oracle. The oracle checks720 cyclic paths/2160 arm records/383040 interval coefficients and992 trace slabs, exact clocks, cap outcomes, domain and recurrence conditions, and explicit coefficient/multiplicity negative controls. These expected counts are not executed results at review time.

Any scientific failure must retain its original source, input and receipt. The implementation owners report the two cyclic fixes and exact-cap clarification as a single parent-controlled prepublication correction episode; the parent records the inherited N03/N05 use. This review creates no new allowance and does not reset exhausted older tasks.
