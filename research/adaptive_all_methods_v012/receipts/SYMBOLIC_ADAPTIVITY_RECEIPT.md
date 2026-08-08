# Exact Algebra Receipt — Adaptive Step Research

## Nonuniform BDF2 derivative weights

Let the accepted current step be `h_n=h` and the previous accepted step be
`h_{n-1}=h/r`, where `r=h_n/h_{n-1}>0`.  Interpolating a quadratic through

\[
t_{n+1}=0,\qquad t_n=-h,\qquad t_{n-1}=-h-h/r
\]

and differentiating at zero gives

\[
w_0=\frac{1+2r}{h(1+r)},\qquad
w_1=-\frac{1+r}{h},\qquad
w_2=\frac{r^2}{h(1+r)}.
\]

Thus

\[
\frac{1+2r}{1+r}y_{n+1}-(1+r)y_n
+\frac{r^2}{1+r}y_{n-1}=h_nf_{n+1}.
\]

Substitution of `r=1` gives the constant-step BDF2 equation after multiplication
by two.  The coefficients reproduce derivatives of polynomials of degree zero,
one, and two exactly.

## Step-doubling estimator

If a one-step method has local expansion

\[
y_h-y(t+h)=C h^{p+1}+O(h^{p+2}),
\]

then two half steps satisfy

\[
y_{h/2,h/2}-y(t+h)=2\,C(h/2)^{p+1}+O(h^{p+2})
=2^{-p}C h^{p+1}+O(h^{p+2}).
\]

Therefore the fine-path local error is

\[
e_{\rm fine}
=\frac{y_{h/2,h/2}-y_h}{2^p-1}+O(h^{p+2}).
\]

The controller exponent is `-1/(p+1)` for this estimator.  For BDF2 startup,
`p=1` is used until both coarse and fine histories support order two.

## Dimensional checks

- `h`, `h_min`, `h_max`: time.
- `r=h_n/h_{n-1}` and all controller factors: dimensionless.
- `atol` has the units of each state component; `rtol` is dimensionless.
- WRMS normalized local errors are dimensionless.
