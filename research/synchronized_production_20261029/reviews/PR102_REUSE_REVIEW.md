# PR102 late-update compatibility and reuse audit

Reviewer: defect_theory. Date: 2026-10-29 Asia/Seoul.
Method: read-only immutable `git show`/`git diff`, source algebra, published
execution-log and verifier inspection. No checkout modification, merge,
benchmark, test replay or new scientific execution.

## Pins and decision

* Common PR101 baseline: `bc5ddbe42b24b1a6364bf653c3e2d6d938ca4087`.
* PR102 delivery: `8357faa5b42724a8be24609d4d2f16aec8a6eb3a`.
* PR102 actual code execution source: `0d722b4ed71413926c76bc0037f02f3a6bbdd362`.
* Current synchronized-lane execution source:
  `ef84aa3d6c028b0922bb5c1e66202b95fd88a2fc`.

**ACCEPT_SCOPED_REUSE** of the native scalar-affine accepted-segment provider.
Its existence supersedes an unqualified statement that native affine event
certification is still absent. It remains a separate source-bound lane:
the combined PR102+synchronized tree has not been built or executed here.
Actual physical full30/full33 provider qualification and production stay HOLD.

## Source identity verification

Read `REPORT_KO.md`, `ADMISSION.json`, `FINAL_SOURCE_IDENTITY.json`,
`VERIFICATION.json`, `BIT_ORACLE_RESULTS.json`, the independent bit verifier,
the complete new Rust certificate source and the dense-adapter diff.
The five changed crate files below have the same Git blobs at the code and
delivery pins; their computed SHA256 agrees with FINAL_SOURCE_IDENTITY.
Cargo.lock is unchanged from the common baseline.

| File under crates/rodas5p-integrators | Git blob | SHA256 |
|---|---|---|
| `src/affine_event_certificate.rs` | `437ed10c302ba196767ef5291661d5b65fdc476d` | `d0e716966c0dc6f30fcec1887cefdf54a71a4a3bc35fa6cca0d3a0a508c33fde` |
| `src/rodas5p_fast_dense.rs` | `70c1c437b002058a998eaa66b244da82d6e362ce` | `921cb5537d9bab52169f0af61884f8b5c7060b93c2f794d054826f47847969ef` |
| `src/lib.rs` | `3769110df1365383d7bc1bfa75d3e9f6dfe6edd2` | `ddf3f3819455ae7d6facd8fd0092168378a081f4802510eefe1b594ade1d950a` |
| `tests/affine_event_certificate_contracts.rs` | `28a56802b54221964cbfc085bca4efb59da754a5` | `287e30abf5acde1d51358f7d47d75b4ba7f877052ceaa15f504d4d1679cc7be7` |
| `examples/sym02_certificate_probe.rs` | `4102f23d3031362ae8af60cb78a4598affe71473` | `1b4989768f7ae998a71e612aca81749292a414cc23fea9e7a24641f345de3dae` |

These paths are disjoint from synchronized-lane changes in Krylov workspace,
gmres_into, facade solve, its ownership probe, and time-FD test lint. This is
a source-overlap observation, not proof that an integrated tree passes tests.

## Mathematical/code compatibility

### Represented candidate and original residual

The native adapter only issues NativeAcceptedRodas5pFast provenance for one
scalar accepted FastDenseSegments packet. It copies the actual stored stage,
H, endpoint and time bits. Declared/synthetic packets retain separate origin.
Expansion of the existing dense expression gives the exact-real quartic

`(y0, y1-y0+d0, d1-d0, d2-d1, -d2)`, `d_r=sum_j H_rj u_j`.

Thus the repaired quartic comment agrees with the implementation. Treating this
as a cubic would omit a real term; PR102 retains the final H row and tests it.
The original equation residual is `p_theta'/h + lambda p - b` with
`h=real(t1)-real(t0)`, enclosed by directed interval subtraction. Degree-four
Bernstein coefficients bound that exact residual on the entire slab. Constant
lambda/forcing boxes are included in this residual, so their declared
uncertainty must not be added a second time by a synchronized wrapper.

### Dini propagation and D-theorem specialization

For `lambda.lo>0`, the affine field is globally contractive in scalar absolute
norm. Its Dini comparison uses damping lambda.lo and the actual-path defect.
This is the Gamma=0, carrier-mismatch=0 specialization of the synchronized
composition theorem. A finite invariant-radius bootstrap is unnecessary for
this globally defined affine field; the nonlinear D4 radius test remains a
separate useful extension when source validity or quadratic remainder is local.

PR102 uses the inherited PR97/100 `max(U1,U2)` rational endpoint enclosure and
`max(E0,endpoint)` for whole-slab error. Those are valid in both signs of
`E0-delta/lambda`. Its envelope is generally weaker than the PR101/D8
second-order exponential upper used in our exact consumer. Replacing it is
not necessary for reuse and was not attempted under the exhausted correction
cap. The two formulas' coexistence is not a correctness conflict.

### Query rounding and representation jump

The certificate is first for the exact-real polynomial, not its rounded
query implementation. At an actual native returned binary64 center q, it
encloses the exact polynomial at the same represented time in I and forms
`rho=max(abs(q-I.lo),abs(q-I.hi))` outward. Hence
`abs(q-x_exact)<=rho+E` follows by triangle inequality.
Endpoint copies use the stored exact endpoint, avoiding unnecessary polynomial
dependency loss. Interior normalization is separately enclosed and intersected
with the proven exact theta domain [0,1].

This is a **pointwise representation displacement** in the synchronized
transfer terminology. It must not be accumulated as a physical jump at every
dense query. Carry uses the polynomial's endpoint error, and the query rho is
added only to the returned observation. A global uniform query-rounding
constant has not been inferred from the42 sampled queries.

### Chronological binding and event logic

The carry object requires exact adjacent time/state bits, next segment index,
origin and unchanged source/model/clock/unit/event/epoch/parameter specification.
It therefore implements the identity transfer kappa=1, eta=0 for an unreset
scalar representation. Changed norm, state reset or source epoch correctly
requires an externally justified new incoming bound; this provider does not
silently authenticate such a transformation.

Whole-tube event sign uses the certified state interval. Opposite endpoint
signs imply a crossing; a strictly signed original affine flow over the tube
adds uniqueness. Strict whole-slab separation, or equal strict endpoint signs
plus monotonicity, gives no crossing. Equality/grazing remains unresolved.
These conditions match the D5 event proof and PR100 hypotheses.

The source IDs remain caller assertions. NativeAccepted provenance proves where
the segment came from, not that an arbitrary RHS callback equals the declared
affine formula or that the supplied incoming global error is justified.

### Arithmetic premise

The production crate keeps forbid(unsafe_code). Safe black-box arithmetic
probes check nearest/ties-even behavior and gradual underflow at proof calls;
the implementation is scoped to x86_64 and assumes the floating environment
does not change during a call. The published mode tests are evidence for that
scope; they do not qualify other targets or arbitrary compiler arithmetic modes.

## Evidence actually reused

The published `evidence/C8_POST_LINT_FIX.log` was read and records13 new
contract tests plus8 existing fast-dense tests PASS, no ignored methods.
The immutable verifier source reconstructs quartic/residual Bernstein
coefficients using Fraction from emitted binary64 bits, then checks query
displacement and carry. Its published result records:

* six records: four declared/synthetic and two actual native accepted segments;
*42 query records, of which14 are actual native queries;
*381 total checks, comprising367 exact rational comparisons and14 Decimal90
  analytic-solution diagnostics;
* actual native IVP `y'=-y`, span[0,0.125].

This audit did not rerun those tests or the bit verifier. The result is retained
as PR102 evidence at its original code pin, not counted as another execution
of the synchronized lane or its nonlinear exact consumer. Published abstract
theory totals324 rational cases/16524 assertions/11016 Decimal diagnostics are
additional PR102 records, not newly independently replayed here.

## Integration obligations and useful next step

1. Reuse the native affine provider and its tests rather than rebuild a second
   accepted-segment certificate. Merge/cherry-pick into a new isolated local
   integration branch only after the current branch and PR102 pins are fetched;
   preserve both execution identities and all budgets.
2. Rebuild and test the *actual combined tree*: the13 affine and8 fast-dense
   contracts, synchronized facade ownership and Krylov workspace contracts, and
   source-bound Python oracles. Test unchanged numerical outputs and error carry
   after the facade ownership transformation, even though crate paths differ.
3. For a real application supply an accepted owner capsule binding equation,
   source/rates, units, physical clock, initial global error and scalar event.
   Its source contract is the prerequisite; ID strings alone do not provide it.
4. The adapter skew-product theory has a numerical clock component x0. Its
   forcing `delta_j+G delta0+L Phi(B)(C+s delta0)` and the physical-coordinate
   recovery term cannot be replaced by PR102's single requested-time affine
   contract. Reuse the Bernstein/query/carry mechanisms after proving that
   actual equation and source-offset embedding, or retain the existing
   clock/fiber bound as the conditional path.
5. Full nonlinear Gamma/deltaJ/domain-tube transplantation still needs actual
   providers and outward arithmetic. A directly certified original dense-path
   residual already contains stage errors: do not additionally charge the
   triangular stage-reconstruction bound for the same error.
6. General DOPRI/piecewise dense output and multi-component physical metrics
   require their actual polynomial/partition and norm-transfer contracts.
   These are distinct from this native RODAS scalar quartic.

No new native run, production gate promotion or correction-budget reset follows
from this late source audit. Original DAG delta remains0, and the integrated
physical qualification remains a local follow-up obligation.
