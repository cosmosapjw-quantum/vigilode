# V8_SOURCE_CHEMICAL_JACOBIAN_REMAINDER_20261028

Root contract, 2026-10-28. User-authorized full physmath 1.0.0 R1–R10/C1–C10. This is a new finite algebra unit; all earlier campaigns remain CLOSED and all native/unknown/reserved owner budgets remain UNKNOWN_PRESERVED.

## Frozen scientific target

Use the actual cc602 BASS transition topology and the inherited affine chemical restriction to construct the six-variable chemical Jacobian family and its exact entrywise range. Include the electron-feedback derivative. Derive and implement the exact quadratic remainder and its fixed-coefficient bound in the existing physical norm. This supplies a concrete model-source component for the latest VigilODE integrated polyalgorithm's B6 remainder premise.

Binding: `8cb268e8de741e284cf494013f1e0748d27f03c5e77ba0c64413ae92af082d16`; BASS source `cc602e209e7deeb511ab2a1f3b2268d55206c43c`. Source provenance is in intake/actual_source_next8/SOURCE_IDENTITIES.json. Source formulas use real arithmetic applied to declared represented coefficients; actual callback floating error is a separate obligation.

## Coordinates, parameters, domain

z=(HII,HeII,HeIII,LiII,LiIII,LiIV) occupies reduced30 indices 23..28. Neutral totals are exactly the inherited binary64 values: T_H=0x1.fffffffffffffp-1, T_He=T_Li=1. Never replace T_H by 1. Neutral populations are T_H-z0, T_He-z1-z2, T_Li-z3-z4-z5.

The physical state domain is the product of three nonnegative simplexes with those totals. Its 24 vertices are the Cartesian product of 2,3,4 block vertices. All 36 Jacobian entries receive min/max and attaining state/rate witnesses over this entire declared domain.

The twelve independent nonnegative rational rate intervals are ordered: photo_H0, photo_He0, photo_He1, photo_Li0, photo_Li1, photo_Li2; recomb_H1, recomb_He1, recomb_He2, recomb_Li1, recomb_Li2, recomb_Li3. Effective rates are gamma=Gamma/H and kappa=n_b alpha/H. Their actual clock/geometry/source enclosure remains an external premise; intervals used in tests are explicitly diagnostic inputs. No native rate function is imported or called.

e=(wH,wHe,2wHe,wLi,2wLi,3wLi), with wH=sH1+sD, wHe=sHe3+sHe4, wLi=sLi6+sLi7. The API accepts positive exact frozen weights with provenance. A separate source-reconstructed inventory witness may mirror only the finite `_prepare_inventory` scalar operations under IEEE binary64 RN: parse source numeric literals, round each mass-times-ratio product, round the exact fsum to normalization, divide and round isotope inventories, then take their exact real sums for e. Save every represented hex value and the input/source identities. Label this SOURCE_RECONSTRUCTED_UNDER_RN, not an observed owner-prepared scale array. The original owner scale array and source_manifest body have not yet been recovered. Synthetic weights are allowed only in labeled controls. No source initialization/model call is permitted.

## Required algebra

F(z)=b+P z+(e^T z) Q z. P and b follow adjacent upward photo transitions with the inherited neutral substitutions. Q follows adjacent downward recombination transitions; its blocks are [-rH], [[-rHe1,rHe2],[0,-rHe2]], and [[-rLi1,rLi2,0],[0,-rLi2,rLi3],[0,0,-rLi3]]. Each first photo row uses p0*T and -p0 times every charged population in its element; intermediate photo outflow/inflow follow the source topology.

J(z)=P+(e^T z)Q+(Qz)e^T.

D2F[d,h]=(e^T d)Qh+(e^T h)Qd.

F(z+d)-F(z)-J(z)d=(e^T d)Qd, exactly for frozen coefficients.

For fixed state vertex each J entry is linear in the twelve rates. Choose each rate-box endpoint according to that coefficient's sign, separately for min and max. Optimize over the 24 state vertices. The proof uses affinity in the whole state vector at fixed rates and affinity in rates at fixed state, not a generic unsupported claim about determinants, eigenvalues, or the RHS itself. Return exact rational endpoints plus all 72 attaining witnesses and a source-coverage record. Exactness refers to the declared independent product family; shared H/T/UV correlations may make this family wider than the actual model family.

## Reuse and downstream connection

Copy unchanged next7 matrix_bounds.py and its exact dependencies to deps/prior_matrix with a byte-identity manifest. Reuse its metric(u33), generalized_upper, matrix operations and scalar sqrt enclosure. Never implement another norm or lift or replay the old test suites.

For a fixed Q, define only the named coefficient operator Qbar acting on reduced30 chemical coordinates and vanishing on other inputs/outputs. This is the polynomial remainder coefficient operator, not an assertion that missing full Jacobian blocks vanish. Let ebar be e in the same six coordinates. Obtain existing G,Ginv from the prior metric. With sQ certified by prior generalized_upper(Qbar^T G Qbar,G,Ginv), and sigma2=ebar^T Ginv ebar, return Gamma >= sqrt(sigma2*sQ). Then ||R(d)||_G <= Gamma ||d||_G^2 and ||D2F[d,h]||_G <= 2 Gamma ||d||_G ||h||_G. Restrict this implemented bound to frozen supplied rates; an interval-rate generalization may be documented as a subsequent composition.

The six-by-six chemical partial block is returned with coverage only rows/columns 23..28. Actual geometry columns -Fchem*(grad log H)^T, geometry rows, Thomson/tau rows, time variation, rate preparation bounds and callback rounding stay OPEN_NOT_ZERO. Do not pass the partial block to a full-model provider. Existing tube membership can imply the chemical simplex premise only when its actual binding/domain certificate applies. Residual/Jacobian/lognorm/solution-enclosure records retain their distinct roles.

## Required independent controls

1. Independently assemble a nine-state conservative adjacent-transition flux model and its population-dependent electron derivative; restrict it using source neutral elimination and compare F and J at exact diagnostic states/rates. A twenty-one-state equality-restriction oracle is optional if it remains within the budget.
2. Hydrogen-only oracle: gamma*(T-z)-kappa*w*z^2; J=-gamma-2*kappa*w*z. An intentionally omitted feedback term must be caught with a nonzero exact discrepancy.
3. Conservation and boundary behavior in the nine-state flux representation; neutral offset and non-unit H total must be retained.
4. Each of 72 witnesses belongs to state simplex/rate box and attains its exact endpoint by the independent flux derivative oracle.
5. A small independent exhaustive rate-corner subset checks extrema without calling the endpoint-sign algorithm; zero-width rates reproduce the fixed-rate specialization. Avoid exhaustive4096*24*36 fractions unless the registered budget allows it.
6. Interior exact points stay within bounds; the mathematical vertex proof supplies the continuum conclusion.
7. Exact remainder and Hessian polarization at signed increments; degree-two scaling R(2d)=4R(d). The polynomial identity permits signed trials; physical-domain claims still require membership.
8. Native metric remainder bound uses unchanged prior primitives; exact low-dimensional oracle and rescaled u33 reveal omission of scale/33/neutral lifting factors.
9. Invalid shapes, negative/reversed rates, invalid totals/weights/binding, nonfinite approximate inputs, scope promotion and serialization errors are rejected explicitly where applicable.
10. Plot a named H-only feedback mismatch and quadratic-remainder scaling from exact diagnostic values, with honest labels, source data, and an independent hostile review. This is a new diagnostic figure, not an actual ODE trajectory.

## Execution and admission

Only the lower-tier implementation worker writes/runs code and packages. Root/Astra freezes contract and adjudicates; a nonauthor Astra reviews theory/code and actual outputs. Source reads, metadata/hashes, and file packaging are separate from scientific invocation.

No scientific import/test/CAS/figure/model execution until root GO after preexecution review and backed-up checkpoint. New suite: one primary invocation, at most one focused repair only after a concrete failure and root direction. Local scientific ceiling total CPU60s/wall90s, suite per process CPU45s/wall60s; figure CPU10s/wall15s. Single Wolfram exact-symbolic evaluation timeConstraint20s, local tool wall recorded, remote CPU unknown. Packaging separately measured. All RHS/Jacobian/ODE/NCP/native/build calls must remain0. Preserve first source/raw regardless of outcome.

Successful admission: exact source-to-six-variable proof, all controls actually pass, 36 extrema/72 witnesses serialized, fixed-rate native remainder bridge checked, figure inspected, and independent review PASS_SCOPED. Central semantic count stays28/88 unless the canonical DAG itself changes; this unit does not close full V8.

Ceilings: query_rounding_bound:null; actual_query_rounding_bound:null; whole_domain_certificate:false; actual_ode_solution_enclosure:false; VerifiedModelProvider:null; V7fullrestore:NOT_RUN; production:HOLD; globalphysicalcompletion:null; L4:false. REI25 remains a separate binding.

Git recovery is an independent authorized operation. The previous admitted19path tree and commit have actual ACK; guarded ref publication is in progress. Every ref update uses expected_sha and force:false, with fresh HEAD/AGENTS/parallel checks and actual readback. New writes do not change prior CLOSED scientific budgets.
