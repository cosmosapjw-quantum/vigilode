# F5: exact regular range, a cubic wall, and a missing horizontal chart

## Status and provenance

The vertical family has an exact geometric range and a newly computed self-transversality wall. A smooth, geometrically regular point on that wall has local quotient length **3**, not 1. Global work additionally finds a **missing horizontal same-abscissa branch** in the source's claimed chart exhaustion. Its baseline points satisfy the original incidence equations and have nonzero 12-by-12 Jacobian determinant. Thus this work does **not** propagate the source's 116/58 global count to any parameter family, or certify that count for its original stated full carrier.

Read-only input: `../baseline/RESEARCH_NOTE_KO.md` §3, `../baseline/SOURCE_REVIEW_KO.md`, `../baseline/evidence/INPUT_MANIFEST.json`. Original v13 PDF was materialized with SHA-256 `d9dc4abb2c15c1e2a242c0e0d4a1e3cf87a6332ef98511c62a7d7deab3288d81`; relevant text is §§11.1,14.1–14.5. Stage5 ZIP SHA-256 `ca3e6c6303e31b429faf5f7ccb513d074d074e979d6fb1a0350173e4b9977019` was materialized, and its manuscript §7 and original-incidence generator were read. The latter makes stationary-leg deletion explicit; we retain that stricter nonstationary carrier below. Historical pass counts and global conclusions are not inherited as new verification.

## 1. Dimensionless moduli

For `y²=x³+px+q`, `G=diag(gx,gy)`, `eta=gx/gy>0`, put

\[
 x=\eta X,\qquad y=\eta^{3/2}Y,\qquad
 \alpha=p/\eta^2,\quad\beta=q/\eta^3.
\]

The curve becomes `Y²=X³+alpha X+beta`; the metric becomes the scalar multiple `gy eta³ diag(1,1)`. Hence `(alpha,beta)` are the two dimensionless metric-curve moduli. Multiplying a metric by a positive scalar changes normal representatives and coefficients, not the incidence geometry. Smoothness is `4 alpha³+27 beta² != 0`. In the polynomial derivations use the scaled normal

\[
 n(x,y)=(-f'(x),2\eta y),
\]

which is `gx` times the original metric normal. Coefficients below are correspondingly rescaled.

## 2. Vertical packet: exact geometry and transversality

Let

\[
 m=\eta/3,\quad H=-p-\eta^2/3,\quad
 c=4q-8\eta p/3-32\eta^3/27.
\]

For either sign of `h=sqrt(H)` and `b=sqrt(c)`, the two ordered witnesses are

\[
 ((m+h,-b/2),(m+h,b/2)),\quad
 ((m-h,-b/2),(m-h,b/2)).
\]

Both have coefficients `lambda=1/(2eta), mu=-1/(2eta)`. Their source and return are `(x-f'(x)/(2eta),-b)` and `(x-f'(x)/(2eta),b)`. Equality and the curve equations reduce exactly to the displayed `m,H,c` formulas. Four-distinct feet and nonstationary legs hold precisely when `Hc != 0`. Nonparallel normals additionally require

\[
 f'(m+h)f'(m-h)=\frac49(\eta^2+3p)(4\eta^2+3p)\ne0.
\]

Therefore the **real regular vertical range**, assuming smoothness, is exactly

\[
 \alpha<-1/3,\qquad \alpha\ne-4/3,\qquad
 \beta>\frac23\alpha+\frac8{27}.
\]

There are four ordered real collisions in this packet. No other vertical collision exists: `b != 0` forces each chord's starting ordinate to `-b/2`; equality of its sources for distinct abscissae forces the equations above.

Geometric regularity is weaker than self-transversality. Define

\[
 D=4\eta^3p+27\eta^2q+27pq
   =\eta^5[4\alpha+27(1+\alpha)\beta].
\]

On the regular vertical range, the collision is self-transverse **iff `D != 0`**. Here is a direct Jacobian certificate, without genericity inference. For common chord `(a,b)`, `c=b² != 0`, set

\[
\begin{aligned}
 T(z)&=a(3z^2+3az+a^2+p)-c,\\
 S(z)&=3z^2+p,\qquad U(z)=3(z+a)^2+p,\\
 K(z)&=U(z)T(z)-S(z)(T(z)+2c),\\
 R(z)&=-\eta a(T(z)+2c)-cU(z),\\
 E(z)&=T(z)^2-4c f(z).
\end{aligned}
\]

The starting ordinate is `T/(2b)`, normal determinant `eta K/b`, and

\[
 P_x=z-\frac{SR}{\eta K},\qquad
 2bP_y=\frac{T(K+2R)}K.
\]

Take the even part and divided odd part of `E(m+h)`, replace `h²=H`, and take the divided difference of these two source coordinates after clearing their displayed denominators. These four polynomial equations `E0,E1,Gx,Gy` are generated in `derive.py`. Division by `2h` is legitimate on `H != 0`. No division by `a` is made. Their determinant with variables `(a,m,H,c)` at the vertical point is

\[
 \frac{2^{23}}{3^{21}}
 (\eta^2+3p)(4\eta^2+3p)
 (8\eta^3+18\eta p-27q)^6 D.
\]

All factors preceding `D` are units on this chart. Reconstruction from `(c,H)` is etale on `cH != 0`, and solving the normal equations is an isomorphism on `K(x)K(u) != 0`; hence the quotient Jacobian criterion is exactly the original self-transversality criterion.

The real wall is `beta=-4alpha/[27(1+alpha)]`. It lies strictly in the vertical real range for `alpha<-4/3` or `-1<alpha<-1/3`. Its cubic discriminant is

\[
 4\alpha^3+27\beta^2=
 \frac{4\alpha^2(3\alpha+1)^2(3\alpha+4)}{27(\alpha+1)^2}.
\]

Thus these intervals do contain smooth regular wall points.

## 3. Exact length-three wall and local splitting

At `(eta,p,q)=(1,-1/2,4/27)`, one has `H=1/6`, `c=20/27`, normal product `-5/9`, and smoothness expression `5/54`, but `D=0`.

The minor of `(E0,E1,Gx)` in `(m,H,c)` has determinant `12800000/1594323 != 0`. The implicit equations therefore give analytic functions of `a` with

\[
\begin{aligned}
 m&=1/3-a/2+(3/40)a^2+O(a^3),\\
 H&=1/6-(9/20)a^2+O(a^3),\\
 c&=20/27-(7/4)a^2+O(a^3).
\end{aligned}
\]

Substitution into the last equation gives

\[
 G_y=\frac{2080}{729}a^3+O(a^4).
\]

Third-order corrections in the eliminated coordinates cannot change this leading coefficient because all their first derivatives in `Gy` vanish at the base point. Thus the completed local quotient algebra is `C[[a]]/(a³ unit)`, of length exactly **3**. Each of its four ordered etale lifts also has length 3. This is a scheme-length statement, not three distinct points at the wall.

Varying `q` at fixed `eta=1,p=-1/2`, the coefficient of `a` along the persistent vertical solution has derivative `6400/243`. Reversing both ordered witnesses induces `(a,m)->(-a,m+a)` and leaves centered `m+a/2,H,c` unchanged. After multiplication by an analytic unit, the reduced equation is `a` times an even analytic function of `a`. Its leading unfolding gives

\[
 a^2=-\frac{120}{13}(q-4/27)+O((q-4/27)^2).
\]

Consequently two additional real local quotient branches exist for sufficiently small negative `q-4/27`, and disappear into a complex-conjugate pair for positive perturbations. The vertical branch persists. This exact local bifurcation shows why real cardinality cannot be assumed constant over the geometric regular range.

## 4. Missing horizontal same-abscissa collisions

The source's horizontal argument compares the **two different** possible starting abscissae. It overlooks two distinct starting feet at the **same** abscissa and opposite ordinates. The formula `y=T/(2b)` is unavailable when `b=0`, so `h=0` is not equivalent to equal witnesses there.

Let `a²=4H != 0`, put `x=eta/3-a/2`, `u=x+a`, and choose `y²=c/4 != 0`. Consider

\[
 ((x,y),(u,y)),\qquad ((x,-y),(u,-y)).
\]

All four feet lie on the curve, and both witnesses have coefficients

\[
 \lambda=-1/(2\eta),\qquad\mu=1/(2\eta).
\]

The common source and return are

\[
 P=(x+f'(x)/(2\eta),0),\qquad
 Q=(u+f'(u)/(2\eta),0).
\]

The two normal determinants are opposite, with magnitude represented algebraically by `4 eta² a y`; both are nonzero. This packet is geometrically regular whenever `Hc != 0` and the curve is smooth. Unlike the vertical packet, it requires no individual nonvanishing of `f'(x)` or `f'(u)`.

For each of two chord orientations `a=±2sqrt(H)`, changing the sign of `y` exchanges the two collision witnesses. There are **four ordered points**, **two witness-unordered points**, and **two double-quotient points**, each with a stabilizer of size 2 under the joint order-4 action. The curve involution and witness exchange coincide here. The source's free-action lift factor of 4 does not extend to this chart.

`full_incidence.py` substitutes this packet into all twelve original polynomial incidence equations, not the `b != 0` quotient. It also computes the exact full Jacobian determinant, with variable order `(x1,y1,x2,y2,x3,y3,x4,y4,lambda1,mu1,lambda2,mu2)` and equation order documented in the script:

\[
 \det J_{12}
 =-384a^4\eta y^4[-a^4\eta+4a^2\eta^3+6a^2y^2-16\eta^2y^2]
 =\frac{1024}{3}\eta H^2c^2D.
\]

Thus this horizontal packet is also self-transverse exactly when `D != 0`.

At the baseline `(eta,p,q)=(1,-1,1)`, take

\[
 x=(1-\sqrt6)/3,\quad u=(1+\sqrt6)/3,\quad y=\sqrt{37/27}.
\]

Then `P=(1-2sqrt(6)/3,0)`, `Q=(1+2sqrt(6)/3,0)`, and the full determinant is `-358875136/19683 != 0`. These are real, reduced, isolated four-distinct collisions. They have horizontal chord, so are disjoint from both the source's `a b != 0` component and the vertical packet.

### Completeness of the horizontal chart for p != 0

For a nonzero horizontal chord `(a,0)`, curve membership forces

\[
 3x^2+3ax+a^2+p=0.
\]

A second starting abscissa is either `x` or `-a-x`. The regularity assumptions imply `a != 0`, `y != 0`, and `2x+a != 0`. Solving the normal equations gives

\[
 P_x(x)=x+\frac{3x^2+p}{3(2x+a)},\qquad
 P_y(x,y)=y\left(1-\frac{2\eta}{3(2x+a)}\right).
\]

For the two distinct abscissae,

\[
 P_x(x)-P_x(-a-x)=-\frac{4p}{3(2x+a)}.
\]

Therefore `p != 0` excludes that case. In the same-abscissa case distinct witnesses require opposite ordinates, and `P_y=0` forces `3(2x+a)=2eta`. This is exactly the packet above. Thus the horizontal classification is complete for `p != 0`, including the baseline.

For `p=0` the different-abscissa case survives over C. Writing `w=2x+a`, its necessary curve/source equations reduce to `a²=-3w²` and `9w⁴+4eta²w²+12eta q=0`, with ordinate reconstruction and all regularity factors still to check. There are no nonzero real horizontal chords when `p=0`, but complex global enumeration must include this stratum. No count is assigned to this exceptional chart here.

## 5. What this proves, and does not prove, about global counts

The source's statement that the horizontal chart is empty and its global free-action assertion are false for the explicitly stated regular nonstationary four-distinct carrier. The four newly constructed baseline horizontal ordered points are certified independently of its generic eliminant. If the source's **112 ordered generic points** and **4 ordered vertical points** are all independently verified on their declared `b != 0` domain and exhaustive there, the completed baseline count would be **120 ordered / 60 witness-unordered**, with double quotient **31 = 28+1+2**. This is a **conditional arithmetic consequence**, not a global theorem proved in this task. The source's old degree-29 quotient certificate cannot certify the actual full carrier because it inverted `cH`.

A 12-variable exact universal incidence model over `Q(alpha,beta)` is generated in `generic_global_saturation.sing`. It uses the original four curve equations, four normal-intersection equations, and four source/return equalities. Each of the six foot-diagonal **ideals** is saturated separately, followed by saturation by the two normal determinants and four leg coefficients. This handles the union of diagonal bad sets without incorrectly replacing vector inequalities by products of x-coordinate differences. In particular it retains the valid same-x horizontal chart. Smoothness is a parameter restriction. The generated Singular program is a **reusable, unexecuted input**: Singular, Sage, and msolve were not available in this runtime. No native global saturation run, degree, or success is claimed.

There is a rigorous conditional constancy framework. Let `Z` be the correctly saturated incidence family over the smooth dimensionless parameter surface, and let `Zbar` be its projective closure in the witness variables. Remove from the parameter base the projective images of (i) the infinity boundary, (ii) closure points on excluded geometric loci, and (iii) the locus where the 12-by-12 incidence Jacobian vanishes. On any resulting **nonempty** open set, the morphism is proper, has zero-dimensional smooth fibers, and is finite etale. Its complex degree is constant on each connected component; on a connected real component its real point cardinality is also constant. Proving that this open set is nonempty, computing its full defining exceptional polynomial(s), and obtaining its degree require the missing global elimination/compactification verification. This work does not assert those unperformed steps.

The explicitly detected factors are smoothness `4alpha³+27beta²`, packet boundaries `alpha+1/3`, `beta-2alpha/3-8/27`, vertical parallel-normal boundary `alpha+4/3`, and the common self-transversality wall `4alpha+27(1+alpha)beta`. The `alpha=0` horizontal-chart change must also be examined in any global atlas. These are detected loci, **not an exhaustive global discriminant**.

## Reproduction and limits

Run `python3 derive.py`, `python3 full_incidence.py`, `python3 wall_local.py`, then `python3 test_f5.py` in this directory. Algebra uses exact SymPy rationals and symbolic expressions; there are no floating-point geometry probes presented as certificates. Twelve named exact tests cover curve/source identities, determinant factorizations, geometry at the smooth wall, scaling, and horizontal exclusion for different abscissae. Some tests independently recompute identities; archived-result comparisons are identified by their code and are not independent-engine checks.

Remaining obligations: independently reproduce all generic `b != 0` saturation components, repair the source's chart/quotient definition, handle `alpha=0` and every elimination-exceptional chart, compute the full parameter discriminant and infinity boundary, and then prove a generic/global degree. F5 is **partial**, with the vertical range and local wall solved and a source-level global-count obstruction exhibited constructively.
