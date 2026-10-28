# Independent structural enclosure and charged-failure review

Reviewer: `/root/production_homotopy`, a separate native agent with shared
parent context. This is a complementary analytic derivation and source review,
not a blinded review. No native build or benchmark was repeated by this reviewer.
The exact reviewed source SHA256 is in STRUCTURAL_REVIEW.json.

**No correctness finding is open in the reviewed mathematical/code scope.**
The structural agent completed the final correction3 native/oracle run. Its
recorded source hash matches this reviewed source. The reviewer read the
actual receipts and oracle code, and reused that execution evidence.

## Residual identity and full-dimensional coverage

I independently used `v=x-mu*1`, so
`Jv=-d*v+kappa*(mean(x)-mu)*1` and `J1=-(d-kappa)*1`.
Substituting `U_i=t_i*v+c_i*mu*1+Delta_i` into the original raw-U stage
equation reproduces both scalar defects, the positive RHS-evaluation defect
`h*gamma*(x-f(y))`, and the mean term
`h*gamma*kappa*(t_i+sum_j a_ij*t_j)*(mu-mean(x))*1`.
The lower-block lift coefficient is
`(h*gamma*d*a_ij-gamma*c_ij)*Q +
 (h*gamma*(d-kappa)*a_ij-gamma*c_ij)*P`.
The signs and all full-dimensional lift terms agree with the implementation.

The norm `|t+(c-t)/n|+(n-1)|(c-t)/n|` is the row absolute sum of
`tQ+cP`, so it is the exact induced infinity norm. Applying it to each defect
block, then triangle summation, gives the stated safe upper bound. The native
plan encloses the real products `h*gamma` and real difference `d-kappa` of
represented inputs; it does not substitute the rounded setup product/rate into
the exact target. Rounded scalar transfers are legitimate arbitrary inputs
to the identity, and their scalar equation defects are explicitly enclosed.

## Mean, roundoff and lift analysis

The magnitude-ordered expression `(a-fl(a+b))+b` is exactly Algorithm1.1
FastTwoSum in the original-author source of Ogita--Rump--Oishi, *Accurate Sum
and Dot Product*, DOI10.1137/030601818. I retrieved and read Algorithm1.1 and
Sections2--3 at https://www.tuhh.de/ti3/paper/rump/OgRuOi05.pdf . Its binary
round-to-nearest, finite-arithmetic conditions match the code; gradual
underflow is allowed for sum transformations. Product errors in this code
are separately enclosed, so an underflow-free product transformation is not
assumed.

For `s=fl(1/n)` and `v_i=fl(s*x_i)`, telescoping exact sum residuals leaves
only correction-accumulation and final-addition rounding. Also
`sum|s*x_i|<=sum|v_i|+E_product`, so the scale contribution is bounded by
`[r(s)/s]*(sum|v_i|+E_product)`. This independently yields the stated
compensated-mean envelope. The plan requires an exactly represented positive
dimension no larger than 2^53.

For normal biased exponent e, the bit implementation gives
`r(z)=2^(e-1076)`. At the lower exponent boundary it replaces an unrepresentable
half-spacing by the least subnormal. The greater-magnitude spacing is a safe
choice at powers of two, where the opposite side of the rounding cell is
smaller. At the largest finite binade it is the finite half-spacing. The
function is nondecreasing in |z|; consequently a maximum rounded magnitude
bounds each component's local radius. Expanding the actual RHS and lift
operations gives exactly the source's radius sums and the `|t_i|` multiplier
on the rounded subtraction defect. Nonfinite intermediate values are refused.

## Correction3: exact scalar degeneration

When n=1, `P=I`, `Q=0`, and the represented mean is exactly f0 as a real number.
The actual retained stage expression reduces to one rounded collective product
plus exact signed-zero operations. Its defect is bounded by
`2^-53*|c_i*f0|+eta`, with `eta=2^-1074` enclosing the smaller half-subnormal
constant upward. Precomputing the weighted factor multiplying |f0| and the
weighted subnormal offset is therefore valid. The residual also retains the
collective scalar-equation defect and `h*gamma*rhs_error`. This specialization
uses the exact algebraic dimension 1 identity, not a measured size threshold.

## Scheduled failures and compatibility

I read `ParallelExecution::map_ordered` and every revised closure. Numerical
failure is nested inside outer `Ok`, so sequential and Rayon collections join
all scheduled results. The direct screen always performs i*n state
accumulations and attempts one RHS per scheduled row before a returned numerical
failure. Its scalar counters recover mix/residual work at the actual failure
position. The quotient lift constructs complete rows before finite checking;
the enclosed lift retains per-row entered/completed component counts. The
first stage-ordered error is returned only after all row work is accumulated.

`MeanFieldPlan::step` and `MeanFieldAffine::sequential_step` intentionally
discard the work payload when preserving their old CoreResult signatures.
Both point users to `step_charged`/`sequential_step_charged` for retries.
`MeanFieldEnclosurePlan::step` itself returns StructuralResult. The change
therefore supplies the required charged contract without silently changing
legacy error types. Setup construction remains outside the per-step ledger,
as the source explicitly documents.

## Evidence reuse and precise claim boundary

The independent Python Fraction oracle reads actual returned stages and
evaluates every original equation component with exact rationals. It uses
neither the factorized screen identity nor its rounding-envelope construction.
The prior receipt covers 21 cells, 168 stage bounds and 728 components, including
tiny/huge magnitudes, subnormals, d=kappa, zero rates and non-binary rates/steps.
Its radius check covers 8,184 normal exponent/mantissa combinations. These
finite checks corroborate implementation; the general statement rests on the
derivations above. Correction3 changes the dimension 1 envelope and is followed
by its own final oracle execution by the structural agent.

The result is an original-stage residual certificate. The existing ordinary
f64 `AlgebraicResidualBudget` evaluation, temporal truncation and endpoint
rounding remain separate obligations for a complete physical trajectory
certificate. Individual block norms are sharp; the composed rounding bound
is generally conservative. No global solver or novelty claim is inferred.

## Final execution closure

`structural/CORRECTION3_EXECUTION.json` matches source
`ebc43ae0ef3faada53764f0895c4b6587645d0ee51497a39bfe77d7fcd347330`.
All seven recorded commands exited 0: the 11 native contracts, build, exact
oracle fixture generation and four timing cells. The final independent oracle
passes all 728 original components and 168 bounds over 21 fixtures, with input
SHA256 `56603072497979e3b065a4b72146455fa60821bf84c126126e058139fdb64a5e`.
Its 8,184 exponent/mantissa radius checks are explicitly reused from the prior
unchanged rounding-radius implementation. No new execution was requested by
this reviewer, and no fourth implementation correction was introduced.
