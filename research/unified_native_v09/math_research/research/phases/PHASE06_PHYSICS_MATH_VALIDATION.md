# Phase 6 — Physics and Mathematics Validation

## 1. BDF anchors

For \(M\dot y=f(t,y)\):

### BDF1

\[
F_1(y_{n+1})=M(y_{n+1}-y_n)-h f(t_{n+1},y_{n+1})=0,
\]

\[
J_1=M-h f_y(t_{n+1},y_{n+1}).
\]

The scalar test equation gives \(R(z)=1/(1-z)\), so \(R(z)\to0\) on the negative real stiff limit. Units: every residual term has units of \(My\).

### BDF2

\[
F_2(y_{n+1})=M(3y_{n+1}-4y_n+y_{n-1})-2h f(t_{n+1},y_{n+1})=0,
\]

\[
J_2=3M-2h f_y(t_{n+1},y_{n+1}).
\]

At \(z=0\), the characteristic roots are \(1\) and \(1/3\), so the root condition is satisfied. On the scalar test equation,

\[
(3-2z)\xi^2-4\xi+1=0,
\]

and both amplification roots tend to zero for large negative \(z\). A BDF2 run needs a startup rule; BDF1 startup must be charged and reported.

## 2. Three-stage fifth-order Radau IIA

Let \(s=\sqrt 6\). The nodes are

\[
c=\left(\frac{4-s}{10},\frac{4+s}{10},1\right).
\]

The exact tableau used in the oracle is

\[
A=\begin{pmatrix}
(88-7s)/360&(296-169s)/1800&(-2+3s)/225\\
(296+169s)/1800&(88+7s)/360&(-2-3s)/225\\
(16-s)/36&(16+s)/36&1/9
\end{pmatrix},
\]

\[
b^T=\left((16-s)/36,(16+s)/36,1/9\right).
\]

Exact checks:

- \(A\mathbf1=c\);
- \(b^T\) equals the last row (stiff accuracy);
- the stability function is
  \[
  R(z)=\frac{60+24z+3z^2}{60-36z+9z^2-z^3};
  \]
- its power series agrees with \(e^z\) through \(z^5\);
- \(R(\infty)=0\).

For stage increments \(K_i=h\dot y_i\), the mass-matrix equations are

\[
M K_i=h f\!\left(t_n+c_i h,y_n+\sum_j a_{ij}K_j\right).
\]

The Newton block Jacobian is

\[
\mathcal J_{ij}=\delta_{ij}M-h a_{ij}f_y(t_n+c_i h,Y_i).
\]

This is a \(3n\times3n\) coupled nonlinear solve and its factorization/work cannot be hidden.

## 3. Nonlinear root certificate

Let \(F(\widehat K)\) be the candidate stage residual, \(J_0=F'(\widehat K)\), and \(B\) an approximate inverse. Define

\[
Y=\|BF(\widehat K)\|,
\qquad
Z_0=\|I-BJ_0\|.
\]

Assume on \(\|u\|\le r\):

\[
\|B[F'(\widehat K+u)-J_0]\|\le L_B\|u\|.
\]

The fixed-point map \(T(K)=K-BF(K)\) maps the radius-\(r\) ball into itself and is contractive if

\[
Y+Z_0r+\frac{L_B}{2}r^2\le r,
\qquad
Z_0+L_B r<1.
\]

Equivalently a radii polynomial candidate is

\[
p(r)=Y-(1-Z_0)r+\frac{L_B}{2}r^2.
\]

A negative \(p(r)\), together with the contraction inequality, yields a root radius under the stated assumptions. Without a valid \(L_B\), this is not a proof.

For output \(G(K)=g^TK\), solve

\[
J_0^Tz=g.
\]

If \(\|K^*-\widehat K\|\le r\) and the nonlinear remainder obeys

\[
\|F(\widehat K+e)-F(\widehat K)-J_0e\|\le\frac{L}{2}\|e\|^2,
\]

then

\[
|G(K^*)-G(\widehat K)|
\le |z^TF(\widehat K)|+\frac{\|z\|_*L}{2}r^2.
\]

This supplies the missing separation between a first-order dual correction and a certified nonlinear tail.

## 4. Stage-recursive specialization

Because the RODAS stage Jacobian is block lower triangular, state-error radii can be propagated forward:

\[
r_i\le\|W^{-1}\|\left(\|R_i\|+\sum_{j<i}L_{ij}r_j+\frac12 H_i(r_{<i})^2\right).
\]

The output adjoint propagates backward through the transposed upper-triangular system. This may avoid a generic \(8n\) inverse, but the \(L_{ij}\) and Hessian bounds are compulsory and nonnormal amplification remains visible in the recursion.

## 5. Status

| Item | Status |
|---|---|
| BDF1/BDF2 definitions, dimensions, zero-stability and scalar stiff limits | PASS |
| Radau tableau, order-series and stiff accuracy | PASS |
| Radau full A-stability proof | NOT RE-DERIVED; use established theory and numerical boundary screen |
| Cheap universal nonlinear certificate | FAIL |
| Two-level operational/rigorous architecture | PASS AS DESIGN |
| Stage-recursive rigorous output bound | NOT YET TESTABLE until derivative bounds are supplied |
| Index-1 DAE support | HOLD; residual interface and consistent initialization absent |
