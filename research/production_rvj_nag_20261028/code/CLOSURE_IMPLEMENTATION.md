# Cyclic original-equation certificate implementation

## Status and ownership

Initial C1--C10 coding-lane source after the parent's `THEORY_ADMISSION.md`.
This document records implementation choices and pending validation, not
execution results. The agent has not run Cargo, unit tests, benchmarks, the
exact oracle, or any old native executable. The parent owns source publication,
crate export, compilation, preregistration, scientific execution and the
inherited N03/N05 correction ledger. No correction budget is reset here.

Owned runtime source is `crates/vigilode-coupled/src/cyclic_closure.rs`.
The bounded campaign is
`crates/vigilode-coupled/examples/rvj_closure_campaign.rs`.

## C1--C4: exact target and code mapping

The fixed original model, branch, domains and all-state whole-path cap are
unchanged from `theory/CLOSURE.md`. `CyclicModel` admits n=4,8,16 and carries
an immutable epoch. `integrate_rk4` uses the O(n) cyclic rational action at
four RHS evaluations per dyadic step. It starts x=q=0 at physical t=0 and
ends at t=1. No derivative callback, JVP, AD, Newton update or state-matrix
factorization is performed.

`PreparedEndpoint` has private state bits, dimension/epoch, interval slopes,
optional original-G residual and circuit denominator. Its exact slopes are
the rational original field evaluated with the existing finite
`rodas5p_core::directed::Interval`. The public direct preparer includes G;
the historical preparer intentionally omits unused endpoint G. Both reject
nonfinite or out-of-domain endpoints before field arithmetic.

`certify_slab` always certifies the explicitly named
`exact-original-slope-hermite-v1` path. It bounds the entire polynomial's
control hull in X cross Q. The direct kernel uses theorem C2/C3 constants
3/4 and 25/32 and the exact endpoint coefficient identities. No sampled
supremum or inferred root-iteration convergence is used.

`RadiusRecipe::Legacy` keeps the inherited valid physical recurrence.
`RadiusRecipe::Tight` implements C5's maximum of two rational expressions.
The invalid single-expression forcing shortcut is not an option. The
incoming radius is explicit and finite/nonnegative; callers own a continuous
chain from a justified initial radius.

`enclose_dense` evaluates the exact certified path by interval de Casteljau.
It returns enclosures, not rounded-slope point output with an implied physical
certificate. Its exact theta=0 and theta=1 results are the represented
endpoints. It rejects theta outside [0,1].

## C5: clocks, reuse, work and API refusal

`DyadicGrid` requires N to be a power of two with N<=2^53, so every integer
k and every t=k/N are exactly representable. This bound follows from binary64
integer representability; it is not an accuracy multiplier. `checked_slab`
checks all supplied t0,t1,h bits against the exact grid. Private `SlabClock`
fields prevent forged accepted labels. The module does not infer a clock
correction for other clocks.

Endpoint reuse requires every state bit and the model dimension/epoch to
match. The model, branch, domain and arithmetic provider are fixed in this
module. The benchmark prepares N+1 direct endpoints and verifies N-1 shared
reuse events; the API also permits an O(n)-memory streaming consumer. The
benchmark retains endpoints because the independent exact oracle needs the
entire candidate path.

`CyclicWork` counts attempted work and survives errors through a caller-owned
mutable record. It reports four candidate RHS/circuit calls per step,
interval slope/circuit calls, original-G preparations, endpoint reuse,
attempted/completed certificates, actual historical polynomial products and
transforms, direct components and explicit zero derivative/Newton/matrix
counter fields. Invalid inputs and failed arithmetic are errors; no failure
silently promotes a physical pass.

## C6--C8: comparator and preregistered campaign source

The campaign has three separately named arms:

| Arm | Residual kernel and preparation | Radius |
|---|---|---|
| `historical_power_legacy` | Native original power polynomial, convolution, power-to-Bernstein transform; 2N slope-only endpoints | Legacy |
| `direct_bernstein_legacy` | Fixed-degree direct Bernstein and exact endpoint identities; N+1 slopes and original-G endpoints | Legacy |
| `direct_bernstein_tight` | Same direct kernel and endpoint reuse | C5 tight |

Every arm uses the same native compiled RK4 candidate and the same directed
interval primitives. All endpoint bits are compared across warmups, timed
arms and diagnostic passes. This comparator does not compare a new Rust
kernel against an old compiler/language/hardware timing. The baseline is
not burdened with the direct method's otherwise unused endpoint-G work.

The source specifies n=4,8,16, N=16,32,64,128, one warmup per arm and seven
interleaved timed repetitions. Arm order rotates each repetition. These are
finite resource/sampling choices; they do not enter accuracy formulas.
Candidate, preparation, certificate and complete elapsed times are reported
separately. Complete time includes actual in-run allocations. Compilation,
process startup, JSON construction and diagnostic exports are excluded and
must be reported separately if a deployment claim requires them.
The private prepared-endpoint workspace is explicitly destroyed before the
total timer stops. Returned endpoint/slab outputs remain alive for their
consumer; their later consumer-owned lifetime is outside this producer timer.
If an attempt returns an error, the fallible closure retains its work record
and emits one structured JSON record to stderr with actual partial counters,
arm, n, N, repeat, phase, elapsed time and error. Parent command logs preserve
that record and the process exit status. An operating-system abort is not
silently reclassified as a returned numerical failure.

Every slab is exported once per arm outside the timing cohort, including
exact endpoint bits, h/e0/scalar-bound bits, all path degree-3 controls,
all slow degree-3 residual coefficient intervals and all algebraic degree-6
residual coefficient intervals. Each arm propagates its own incoming radius.
The independently owned oracle substitutes these endpoints and exact original
slopes into the original equations using exact rational polynomial arithmetic.
The campaign's scalar physical-budget result is the maximum all-state
whole-path bound over every slab against the same 1e-8 cap. Coarse-grid
physical failures are preserved as results, not process errors.

Admission compares against the exact rational cap 1/100000000 through an
outward product `bound*100000000 <= 1`. The displayed binary64 `1e-8` value
is not silently allowed to enlarge the mathematical cap.

Expected deterministic counters per arm are:

| Field | Historical | Direct |
|---|---:|---:|
| candidate RHS/cyclic solve | 4N | 4N |
| interval slopes/cyclic solve | 2N | N+1 |
| endpoint original G | 0 | N+1 |
| shared endpoint reuse | 0 | N-1 |
| Bernstein transforms | 4nN | 0 |
| convolution products | 16nN | 0 |
| direct slow components | 0 | nN |
| direct algebraic components | 0 | nN |
| derivative/Newton/state-factorization counters | 0 | 0 |

These are falsifiable counter expectations, not substitutes for measured
performance or exact enclosure validation. The native result may differ
from old attachment bounds because the common directed interval provider is
different. No cross-language timing speedup is claimed.

## C9: meaningful adversarial tests staged in source

The module contains focused tests for nonfinite/domain/dimension rejection,
changed endpoint bits and epochs, each physical-clock label independently,
an interior path leaving the domain despite valid endpoints, a singular
private circuit denominator, and positive forcing with zero initial radius.
The denominator attack is deliberately outside the public Q domain; the
public path rejects it even earlier. These are staged tests, not executed
results. The independent all-slab Fraction oracle remains the decisive
coefficient/radius validation, not self-repetition of optimized formulas.

### Prepublication C9 correction episode

Independent source review found no enclosure/identity blocker and identified
two concrete cost/evidence gaps: prepared-workspace destruction was outside
the total timer, and a returned campaign error lost local partial counters.
Both were corrected before execution as described above. The exact rational
cap comparison was included in the same coherent preexecution correction
episode. Parent conservatively charges this bundle as inherited N03/N05
repair **1/3**; no scientific execution preceded it and no new task allowance
was created.

## C10: parent handoff and completion gate

1. Parent adds `pub mod cyclic_closure;` to the crate and compiles the actual
   sources centrally. Compilation/format corrections are recorded distinctly
   from corrections prompted by a scientific failure.
2. Parent publishes the actual campaign and oracle sources, records exact
   commit/environment/compiler/protocol, reads them back, then authorizes
   native tests, bounded campaign and independent all-slab oracle.
3. Preserve raw timings, every failure and raw JSON. Compare identical-path
   certificate cost first; then compare complete work at the first passing
   tested physical-cap grid. Attribute shared certificate gains to all
   candidate strategies that satisfy this path contract.
4. Any repair consumes the inherited N03/N05 budget as specified by parent;
   no renamed alternative receives a new allowance. At most the allowed
   bounded repair episode follows the first scientific cohort.
5. Even after the bounded native cohort passes, generic stiff solver
   production readiness, other closures, selected singularities, arbitrary
   clocks and wider hardware arithmetic assumptions remain open gates.

Current execution status: **NOT RUN**. Current performance status:
**unmeasured**. Current mathematical status: admitted fixed-model C1--C5
theory with implementation awaiting independent validation.
