# D3 saved-output independent oracle

Implementation author: separate agent `/root/backups_and_thread_links`.
Execution: NOT_RUN at handoff; root owns the measured subprocess budget.

## Scope and method

`review_dense.py` consumes only newly generated D3 saved curves and a summary of the eight frozen runs. It does not invoke the ODE solver, Rust certificate code, D2 tests, or an older campaign. Python standard library `Fraction` and `Decimal` avoid dependency recovery. The analytic reference uses 80 significant decimal digits by default; this is an independent high-precision numerical oracle, not a formal interval proof.

For controls c_i of degree n, the independent power coefficients are a_k=binom(n,k) Δ^k c_0. The residual in normalized time s is d(s)=p_s(s)/(t1-t0)+λp(s)+αp(s)^2. Exact rational differentiation and polynomial convolution produce power coefficients. They are converted to degree 2n Bernstein coefficients with b_k=sum_{j<=k} a_j binom(k,j)/binom(2n,j), then compared with each stored lower/upper interval. This route differs from direct Bernstein multiplication/elevation in the implementation. It uses exact rational t1-t0 from the saved binary64 endpoints.

Every segment is checked for exact affine-domain membership of every control, positive duration enclosed by delta, rho above the entire stored residual interval, incoming E0 bitwise equal to the previous accepted E1, time/curve endpoint continuity, nonnegative radius and full33 uniform budget. Every contraction recurrence is checked independently at Decimal80 against exp(-λΔ)E0+(1-exp(-λΔ))ρ/λ. The uniform bound must enclose max(E0,E1).

Every stored query is independently evaluated by exact rational power-basis Horner evaluation at s=(t-t0)/(t1-t0). Its full33 affine lift is compared with the actual returned center; the exact rounding displacement must fit the recorded rounding bound. The returned radius must include the exact full33 sum of uniform component radii plus that rounding bound and fit the original curve budget. Returned values and endpoints are also checked against the original Riccati solution from the recorded initial binary64 state:

z_j(t)=z_j(0) exp(-λ_j t) / [1+(α z_j(0)/λ_j)(1-exp(-λ_j t))].

The initial time/state are explicit summary inputs; initial state continuity and zero initial radius are checked. The fixed lambda pattern is (1,20,300) repeated. The complete parameter grid must equal alpha{0,4} × degree{3,5} × budget{2e-6,2e-8}. There must be exactly 37 requested outputs and one interior query per accepted segment in each run.

F1 source-review correction: requested times are independently reconstructed with binary64 operation order `tf * float(k) / 37.0`, k=1,...,37, with the last time set exactly to tf. Their bitwise multiset must equal the saved requested-time multiset, and all37 must be unique. Optional summary `requested_times` is checked against the independently reconstructed ordered schedule. Each accepted segment individually must contain exactly one `interior` query, with strict `t0 < t < t1`; matching aggregate counts alone is insufficient. Unknown query kinds are rejected. This correction was made before the first oracle execution.

## Canonical input schema

JSONL segment rows:

`{run,idx,alpha,degree,t0,t1,controls:[n+1][30],incoming:[30],rho:[30],endpoint:[30],uniform:[30],defect_lo:[30][2n+1],defect_hi:[30][2n+1],delta:[lo,hi],queries:[{t,kind,center:[33],radius:[33],rounding:[33]}]}`

`kind` is `requested` or `interior`. Radius already contains the lifted uniform radius and query rounding bound. JSON binary64 numbers must use roundtrip serialization. Optional explicit hex strings or `{bits:...}`/`{hex:...}` tokens are supported.

Summary is a list or `{runs:[...]}` of eight objects. Each object must include `run,alpha,degree,initial_state[30],tf,curve_budget`; `initial_time` defaults to0 only for this bound initial-value model. `lambda[30]` is optional and, if present, must equal the fixed pattern. Optional `accepted`, `queries`, and `query_counts` are compared against observed rows. Any numeric source JSON is interpreted as its actual parsed binary64 value, rather than an exact decimal rational.

## Command

```sh
python3 review_dense.py evidence/segments.jsonl --summary evidence/SUMMARY.json --output REVIEW.json --coefficients all --precision 80
```

`--coefficients all` is the default. The alternative `representative` checks exact residual coefficients for each run's first, last, and maximum-rho segment. All domain/carry/interval-supremum/contraction/query/reference checks still run. The selected mode, exact coefficient/segment counts, and a restricted status label are recorded; representative mode must not be presented as a full exact-coefficient audit. Cached identical component control tuples reduce duplicate exact arithmetic without changing coverage.

Outputs include input/oracle SHA-256 identities, per-run counts, maximum absolute error, maximum query and endpoint error/bound ratios, maximum exact rounding displacement/bound ratio, and elapsed wall time. Decimal comparison slack is 10^-(precision-12), independent of the solver tolerance and only attached to the numerical analytic/recurrence audit. It is reported explicitly; exact interval and rounding checks have no slack.

Each run additionally records `max_absolute_query_error` (actual returned full33 center against the analytic reference), `max_query_error_over_bound`, `max_endpoint_error_over_bound` (compact endpoint component bounds), and `max_returned_radius` (full33). The saved JSON includes `query_audit_rows` containing `run,idx,kind,t,maxabs_error,maxradius,max_error_bound_ratio` for every query already checked. These rows support C7/C8 evidence figures without another trajectory or analytic-reference pass. Stdout omits the rows and reports only their count; the complete rows remain in the output JSON.

## Remaining contract boundaries

This oracle audits accepted saved curves. Reject-before-commit state preservation, callback cost counters, stale lease/foreign model rejection, and memory behavior are properties of targeted implementation tests and source review; accepted-only output cannot establish them. The archive's historic D2 tests/campaign are reused and never executed by this file.
