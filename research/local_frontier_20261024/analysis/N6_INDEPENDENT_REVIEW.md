# Independent review of N6 cached candidate certification

Reviewer: conditioned-provider agent. Read-only review of the final
`InvariantModulePlan::certify_candidates`, its admission path, the reused
`shared_shift_jet::residual_certificate`, and their contract tests.

**Verdict:** no blocking certification defect found. Caching the symmetric-part
bound is sound for this immutable, owned matrix plan. This statement is about
shifted linear-system residual certification, not global ODE accuracy.

The constructor owns B,V,W,H, exposes no mutable matrix accessor, derives its
matrix evidence from those exact binary inputs, and rejects a positive upper
row bound for the symmetric part S=(B+B^T)/2. Gershgorin then implies S is
negative semidefinite. For alpha>=0 and real x,

    x^T(I-alpha B)x >= ||x||_2^2.

Cauchy-Schwarz gives ||(I-alpha B)x||_2 >= ||x||_2, so in finite dimension
I-alpha B is invertible with inverse 2-norm at most one. The directed
original residual r=b-(I-alpha B)u therefore yields

    ||x_exact-u||_2 <= ||r||_2 <= ||r||_1.

`certify_candidates` checks the requested epoch first, then finite nonnegative
alpha, positive finite tolerance, a nonempty RHS collection, equal candidate
count, matching dimensions, and finite entries. These checks establish the
preconditions required by the crate-private residual helper and prevent an
empty/misaligned zip from producing vacuous acceptance. Every residual uses
the full immutable B afresh; the reduced chart and approximate invariance
cannot bypass it. Rounding in candidate generation is included because the
certificate concerns the actual submitted binary candidate. Directed-arithmetic
overflow returns an error rather than an acceptance.

Candidates need not lie in the admitted subspace, as documented: an independently
computed full-space solve can use the same immutable dissipativity evidence.
Conversely an out-of-module projected candidate is rejected when its original
residual exceeds tolerance. Repeated eigenvalues/Jordan entries remain in H;
no distinct-eigenvalue reduction is introduced by the certifier.

The public standalone certifier has no work-budget argument or work counter.
Its O(number_of_RHS*n^2) residual cost should therefore be budgeted by its
caller; `evaluate` already checks its solve budget and charges this work.
This is an API/resource-accounting limitation, not a defect in the numerical
certificate. The certificate is absolute Euclidean error for the fixed linear
system; it does not establish nonlinear or time-discretization error and must
not be transferred to a different matrix/epoch or modified candidate.

Reused evidence: `evidence/N6_tests_round3.json` has exit 0 and hashes matching
all files below. Its stdout records 15 invariant-module and 6 shared-shift
contract tests passing. Relevant tests include a separately computed full-space
candidate, missing/extra/malformed columns, stale epoch, unsupported shift,
zero tolerance, an explicitly wrong candidate, out-of-module residual refusal,
and non-dissipative admission refusal. No additional run was needed solely
for this independent source/proof review.

## Reviewed source identities

- `crates/rodas5p-core/src/invariant_module.rs`: `c1f493466bfb10ee17be2134110de5fad68b438f09acd58160c562ba7bc8faa0`
- `crates/rodas5p-core/src/shared_shift_jet.rs`: `ad60ab702e347ba286628b63309d9c30285cbca37f96477e39f168abf8764be8`
- `crates/rodas5p-core/tests/invariant_module_contracts.rs`: `7e0c550fee10cbc08c935cc00095656d24f67f002f32374b505bef2e4e4c29fc`
