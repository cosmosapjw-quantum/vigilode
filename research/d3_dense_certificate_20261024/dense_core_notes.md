# D3 dense core implementation notes

## Scope and implementation status

Implemented only source/dense.rs. Native directed.rs is unchanged; it is the source blob 89e2b3122b08db1849e4e56cc94f1f58c473b735 supplied by the parent. Existing domain::check is reused for actual returned query states. No unsafe block, platform environment provider, alternate interval layer, solver, or native production hook was added.

Status: implementation written and statically inspected; compiler/test execution NOT_RUN by this agent. Parent owns the measured 120 s execution ledger and all driver/test commands.

Public API:
- Model::new(alpha) -> Result<Model>, supported alpha 0 or 4.
- Curve::new(&Model, degree, t0, t1, &z0, &z1) -> Result<Curve>.
- Curve::certify(&incoming, budget) -> Result<Certified>.
- Certified::query(t) -> Result<Query>.
- Curve::controls()/degree()/times() return copies.
- Certified::diagnostics()/endpoint_radius()/uniform_radius()/budget() return copies.
- pub(crate) Curve::from_controls(...) admits arbitrary fixed controls through identical checks, for adversarial tests; root driver binds actual endpoints and model.

Query exposes center/radius/rounding [f64;33].
Diagnostics exposes alpha,degree,t0,t1,controls [[f64;30];6],delta [f64;2],incoming/rho/endpoint/uniform [f64;30],defect_lo/hi [[f64;11];30],gain_lo/gain_hi/response_upper [f64;30]. Inactive controls/coefficient slots are zero.

## Actual curve and clock

For n=3 or5, the stored binary64 controls define p_j(theta)=sum_i b_ij B_i^n(theta) exactly as real polynomial data. Endpoints remain the supplied original binary64 z0/z1. Construction uses h=fl(t1-t0); the polynomial derivative uses the outward enclosure of the exact represented clock difference Delta=t1-t0.

Cubic:
b1=z0+h f0/3, b2=z1-h f1/3.
Quintic:
b1=z0+h f0/5,
b2=z0+2h f0/5+h^2 Jf0/20,
b4=z1-h f1/5,
b3=z1-2h f1/5+h^2 Jf1/20.
All these operations define the actual stored controls. Their floating construction errors are included by certifying those exact represented controls.

## Domain condition

Every compact Bernstein control is nonnegative.
For each control, outward interval affine reconstruction verifies lower bounds of dependent rows:
row23=1-z23; row25=.5-z24-z25; row28=.2-z26-z27-z28.
The convex hull property gives the whole represented curve in the fixed population domain.

A negative interior Bernstein coefficient is a failure of this sufficient condition. DenseError::DomainNotCertified therefore does not assert that the actual curve leaves the domain.

Exact solution positivity and decreasing compact components follow for the fixed Riccati model with alpha>=0,lambda>0 and the given nonnegative initial state. Group sums therefore preserve their original upper bounds.

## Residual enclosure

For n=degree,m=2n:
d_i=n(b_{i+1}-b_i)/Delta, degree n-1.
Elevation from degree d to m uses weights
C(d,i) C(m-d,k-i) / C(m,k).
The product p^2 uses weights
C(n,i) C(n,k-i) / C(2n,k).
Each integer is exactly representable in binary64 at these fixed degrees; every weight division and subsequent operation uses original Interval operations.

The residual polynomial is
r_j=p'_j + lambda_j p_j + alpha p_j^2.
Every degree-m coefficient is enclosed. rho_j=max_k max(abs(lo_jk),abs(hi_jk)) bounds the continuous defect on the entire interval.

No sampling grid supplies this continuous bound.

## Propagation theorem used

Let x be the same nonnegative exact solution and e=x-p. Then
e'=-(lambda+alpha(x+p))e-r.
Since x,p>=0,
|e(s)| <= exp(-lambda s) E0 + (1-exp(-lambda s))/lambda rho.
The comparison expression is monotone toward rho/lambda, so the uniform bound on [0,Delta] is max(E0,E1).

The original exp_interval gives outward exponential endpoints at the outward exponent endpoints. The response is bounded by
min(Delta_upper, up((1-exp_lower)/lambda)).
Full33 dependent error radii are outward sums of the corresponding compact radii.

The incoming radius contract is conditional on enclosing the same nonnegative exact solution at t0. It does not assert a bound for all signed initial states in an arbitrary symmetric radius interval. Root links this condition using the initial exact represented state and inherited radius chain.

## Query enclosure and transaction

The actual returned compact center uses binary64 de Casteljau; endpoints preserve original endpoint bits.
The exact query coordinate is enclosed as (t-t0)/(t1-t0). Since t is already admitted in [t0,t1], interval intersection with [0,1] is valid.
An independent interval de Casteljau evaluation encloses the exact represented curve at that exact query time.
The physical center uses the same pure affine arithmetic order as admission::lift.
domain::check verifies the actual returned compact/full values, including exact group sums and represented signs.
The rounding radius is the outward max distance of the returned center from the interval polynomial enclosure.
Final query radius=full33 uniform solution radius+query rounding; every component must obey the supplied absolute budget.

Root must prepare all requested queries and one interior query before committing its session.
A later query is independently checked and may refuse without modifying the immutable certificate or accepted prefix.
Endpoint inheritance uses the compact endpoint error radius relative to exact stored z1, not the query center's physical affine rounding radius.

## Error classes and execution obligations

DenseError variants:
Invalid(&str), Arithmetic(CoreError),
DomainNotCertified{control,row,lower}, QueryDomain(domain::Violation),
InvalidRadius{index,value}, Budget{row,radius,budget}.

DomainNotCertified/Budget can trigger the root's bounded retry policy.
QueryDomain contains the original NonFinite/negative/group-overflow classification; the parent can retain that distinction for fatal versus recoverable routing.
Invalid model/radius/time and arithmetic failures never produce a certificate.

The runner must check the same-thread floating environment before every construction/certification/query under the existing research guard. The safe dense module contains no unsafe code and does not close the open production FP-provider gate.

## Existing source principles reused

- dense_output_v2.rs: immutable represented curve, directed derivative/residual, one-sided contraction.
- generic_verified.rs: inherited endpoint radius and complete output preparation before commit.
- packet_curve.rs/packet_curve_output.rs: Bernstein residual refinement and output recentering.
- D2 stage_safe/admission/domain/integer_gate/table/trajectory remain unchanged.

No new generic Bernstein or global-error novelty claim is implied. New deliverable scope is this fixed cubic/quintic Riccati consumer connection.

