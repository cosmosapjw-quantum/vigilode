# Phase 6 — Physics and Mathematics Validation

## Variable-step BDF2

Let \(r=h_n/h_{n-1}\). Interpolation through \(t_{n+1},t_n,t_{n-1}\) gives

\[
\frac{1+2r}{1+r}y_{n+1}
-(1+r)y_n
+\frac{r^2}{1+r}y_{n-1}
=h_nf(t_{n+1},y_{n+1}).
\]

At \(r=1\), multiplying by two recovers

\[
3y_{n+1}-4y_n+y_{n-1}=2h_nf_{n+1}.
\]

The linear extrapolation predictor is

\[
y_{n+1}^{(0)}=(1+r)y_n-r y_{n-1}.
\]

Status: **PASS** algebraically. Numerical stability on arbitrary changing grids remains a method-level empirical gate.

## Step doubling

For a method of global order \(p\), let \(y_h\) be one coarse step and \(y_{h/2,h/2}\) two half steps. The fine-path local-error estimate is

\[
e_{\rm fine}\approx
\frac{y_{h/2,h/2}-y_h}{2^p-1}.
\]

Its normalized WRMS value is dimensionless. The controller exponent is based on local order \(p+1\).

Status: **PASS** as an asymptotic estimator; **CONCERN** near startup, order reduction, nonnormality, and nonlinear-solver error.

## Shared I/PI control

I-controller:

\[
\frac{h_{n+1}}{h_n}
=s\,E_n^{-1/k},
\]

where \(k\) is the estimator order.

PI candidate:

\[
\frac{h_{n+1}}{h_n}
=s\,E_n^{-0.7/k}E_{n-1}^{0.4/k}.
\]

Factors must be bounded; rejected trials must not use a growth factor above one.

## Family map

| Family | Primary current estimator | Estimator order used by controller | Status |
|---|---|---:|---|
| Sequential RODAS5P | embedded 5(4) difference | 5 | PASS |
| SABR5P | embedded RODAS + certified algebraic contribution | 5 | CONCERN on certificate reliability |
| Homotopy RODAS | embedded RODAS + output certificate/fallback | 5 | CONCERN on fast-path certificate |
| BDF1 | step doubling | 2 | PASS baseline |
| BDF2 | variable-step formula + step doubling | startup-dependent, then 3 | PASS baseline with startup handling |
| Radau IIA1 | step doubling | 2 | PASS baseline |
| Radau IIA3 | step doubling | 6 | PASS baseline; native RADAU5 estimator deferred |
| SDC | increment/dense-output estimator; adapt \(h\) and sweeps | method-dependent | FUTURE |
| Peer/W | variable-step local error estimator and step-ratio-aware coefficients | method-dependent | FUTURE |
| ROK/BOROK | embedded temporal estimator plus adaptive Krylov/subspace work | method-dependent | FUTURE |
| Exponential/Leja | embedded/defect estimator plus cost-aware controller | method-dependent | FUTURE |
