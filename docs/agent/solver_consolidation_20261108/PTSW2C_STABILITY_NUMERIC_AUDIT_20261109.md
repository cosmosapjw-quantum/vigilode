# PTSW2C constant-step stability: independent root-locus cross-check

**Date:** 2026-11-09  
**Method:** PTSW2C, two-stage variable-step parallel two-step W method, constant-step stability slice  
**Status:** amplification polynomial derived exactly; stiff-limit spectrum and A(alpha) boundary independently reproduced numerically; no interval-certified sector proof.  
**Execution policy:** CPU only; no GPU code or dependency.

This note strengthens the PTSW2C stability evidence without overstating it. The 2002 source reports A(81.1 degree)-stability and rho_infinity=0.6796. We independently derive the exact constant-step amplification matrix and characteristic polynomial. A numerical unit-circle root-locus scan recovers a limiting angle of 81.16974941 degrees, and the stiff-limit spectral radius is 0.6795805635. This corroborates the source result but is not an interval proof of the whole sector.

## 1. Amplification matrix derivation

Take the scalar test equation
\[
y'=\lambda y,\qquad z=h\lambda,
\]
with constant step h and exact W-matrix Jacobian \(T=\lambda\). For PTSW2C,
\[
\gamma=\frac{12}{25},\quad v=0,\quad
b^T=(1/28,27/28),
\]
and at \(\sigma=1\),
\[
A(1)=\begin{pmatrix}-1/28&-27/28\\-65/252&205/252\end{pmatrix},
\quad
\Gamma(1)=\begin{pmatrix}-6/35&-54/175\\54/175&-138/175\end{pmatrix}.
\]
Define the scaled old and new peer derivatives by \(q_{m-1}=hK_{m-1}\), \(q_m=hK_m\). The stage equations are
\[
(1-\gamma z)q_m
=z\,\mathbf1\,u_m+z(A(1)+\Gamma(1))q_{m-1}.
\]
With \(r(z)=z/(1-\gamma z)\),
\[
q_m=r(z)\mathbf1\,u_m+r(z)(A(1)+\Gamma(1))q_{m-1}.
\]
Since \(v=0\), the advancing output is \(u_{m+1}=u_m+b^Tq_m\). Thus
\[
\begin{pmatrix}u_{m+1}\\q_m\end{pmatrix}
=M(z)\begin{pmatrix}u_m\\q_{m-1}\end{pmatrix},
\]
where
\[
M(z)=
\begin{pmatrix}
1+r(z)b^T\mathbf1&r(z)b^T(A(1)+\Gamma(1))\\
r(z)\mathbf1&r(z)(A(1)+\Gamma(1))
\end{pmatrix}.
\tag{1}
\]
The dimension is \(1+s=3\): one advancing solution value and two old stage derivatives.

## 2. Exact characteristic polynomial

Exact rational simplification of det(w I-M(z)) gives
\[
\boxed{
\chi(w,z)=
\frac{
\begin{aligned}
&2592w^3z^2-10800w^3z+11250w^3\\
&+1824w^2z^2+1600w^2z-11250w^2\\
&-624wz^2-2050wz-667z^2
\end{aligned}}
{18(12z-25)^2}.
}
\tag{2}
\]
At \(z=0\), \(\chi(w,0)=w^2(w-1)\), as required for consistency and zero-stability.

The stiff limit of (1) is
\[
M_\infty=
\begin{pmatrix}
-13/12&-29/336&5/112\\
-25/12&145/336&297/112\\
-25/12&-319/3024&-157/3024
\end{pmatrix}.
\tag{3}
\]
Its eigenvalues are
\[
0.5571972165127336,\qquad
-0.6304504601082186\pm0.2536965897514941\,i.
\]
Therefore
\[
\boxed{\rho_\infty=0.6795805635,}
\tag{4}
\]
independently reproducing the source's rounded value 0.6796. PTSW2C is not L-stable.

## 3. Unit-circle root locus

A stability boundary occurs when a root w of (2) lies on the unit circle. Set
\[
w=e^{i\phi},\qquad 0\le\phi\le\pi.
\]
After removing the nonzero denominator, the boundary points z are the roots of the quadratic
\[
a_2(w)z^2+a_1(w)z+a_0(w)=0,
\]
with
\[
a_2(w)=2592w^3+1824w^2-624w-667,
\]
\[
a_1(w)=-10800w^3+1600w^2-2050w,
\]
\[
a_0(w)=11250w^3-11250w^2.
\tag{5}
\]
For each \(\phi\), both roots of this quadratic were inspected; only left-half-plane points in the upper half-plane are relevant for the upper boundary. The lower boundary follows by conjugation.

A 30,001-point scan over \(\phi\in(0,\pi)\), followed by bounded scalar minimization around the smallest sampled angle, gives
\[
\phi_*=1.7885465625,\qquad
z_*=-0.4373064246+2.8149962319\,i,
\]
and
\[
w_*=e^{i\phi_*}.
\]
The angle measured from the negative real axis is
\[
\boxed{
\alpha_* = |\arg(-z_*)| = 81.16974941^\circ.
}
\tag{6}
\]
The characteristic residual at the computed pair is \(2.196\times10^{-11}\) in floating-point evaluation, and the amplification-matrix spectral radius is 1 to numerical precision.

This agrees with the source classification A(81.1 degree)-stable. A ray scan over 163 angles in the 81.1-degree sector and 600 logarithmically spaced radii from \(10^{-6}\) to \(10^8\) found maximum sampled spectral radius
\[
\max_{\rm sampled}\rho(M(z))=0.999999845290.
\]
A negative control at 82 degrees gives
\[
z=-0.39487634+2.80969117\,i,\qquad
\rho(M(z))=1.0105069736>1.
\]
The latter is an explicit witness that the sector cannot be widened to 82 degrees.

## 4. Evidence boundary

**Derived exactly**
- the constant-step amplification matrix (1);
- the characteristic polynomial (2);
- the stiff-limit matrix (3).

**Numerically checked**
- stiff-limit eigenvalues and \(\rho_\infty\);
- unit-circle root-locus sector boundary (6);
- sampled stability inside 81.1 degrees;
- an unstable witness at 82 degrees.

**Still unresolved**
- a rigorous proof that no other root-locus branch or singular boundary limits the sector before 81.169749 degrees;
- interval-certified positivity/root-count verification over the full unbounded sector;
- W-perturbation robustness when \(T\ne J\);
- variable-step stability for arbitrary admissible step sequences. The present calculation is only the constant-step linear-stability slice.

Therefore the source-reported A(81.1 degree) result is independently corroborated numerically, but not yet promoted to an interval-certified theorem. Do not change production status or remove the stability-proof gate.

## 5. Reproduction

The accompanying verify_ptsw2c_stability.py builds (1) in exact rational arithmetic, checks (2), computes (3), samples both branches of the unit-circle root locus, and checks the sector and outside-sector witnesses. It requires Python, NumPy, SciPy and SymPy.

The output from the research environment was:

~~~text
PASS: exact amplification characteristic polynomial constructed.
PASS: stiff-limit eigenvalues=[(0.5571972165127336+0j), (-0.6304504601082186+0.25369658975149406j), (-0.6304504601082186-0.25369658975149406j)]; rho_inf=0.6795805635.
PASS: unit-circle root-locus minimum sector angle=81.16974941 deg.
PASS: boundary z=-0.4373064246+2.8149962319i, w=exp(i*1.7885465625).
PASS: boundary characteristic residual=2.196e-11, rho(M(z*))=1.000000000000.
PASS: coarse polar scan max rho inside 81.1-deg sector=0.999999845290.
PASS: 82-deg outside-sector witness z=-0.39487634+2.80969117i has rho=1.0105069736>1.
LIMITATION: this is floating-point root-locus and polar sampling, not an interval-certified sector proof.
~~~

## 6. Primary source

H. Podhaisky, Parallele Zweischritt-W-Methoden, PhD thesis, Martin-Luther-Universität Halle-Wittenberg (2002), https://doi.org/10.25673/3017. The institutional dissertation PDF is https://opendata.uni-halle.de/bitstream/1981185920/9802/1/prom.pdf.
