# VIGILODE-CHEM6-MU01: source-structured signed logarithmic norm

2026-10-29 KST. Research baseline: `cosmosapjw-quantum/vigilode` PR102 head `8357faa5b42724a8be24609d4d2f16aec8a6eb3a`, code `0d722b4ed71413926c76bc0037f02f3a6bbdd362`. Frozen BASS owner source `cc602e209e7deeb511ab2a1f3b2268d55206c43c`; original binding `8cb268e8de741e284cf494013f1e0748d27f03c5e77ba0c64413ae92af082d16`. No changes to these identities or to the original 23-node DAG.

## R1–R3. Question, definitions, inherited evidence

Compute the **exact maximum of the signed weighted-infinity logarithmic norm** of the actual-source-restricted six-dimensional H/He/Li chemical Jacobian over a declared product simplex and an independent 12-rate box. This is a model-specific source-structured scalar certificate. It does **not** certify the full reduced30/physical33 Jacobian, the existing non-diagonal physical Gram metric, actual source rates, a full ODE trajectory, or an accepted owner capsule. Its role is to supply a less correlation-losing local/six-component ingredient for the nonlinear residual-to-error bridge after SYM02.

Source chemical coordinates are `z=(HII,HeII,HeIII,LiII,LiIII,LiIV)` and physical domain

\[
\mathcal S=[0,T_H]\times\{(u,v)\ge0:u+v\le T_{He}\}\times\{(l_1,l_2,l_3)\ge0:\sum l_i\le T_{Li}\}.
\]

`T_H` is the exact real value of binary64 `0x1.fffffffffffffp-1`, and `T_He=T_Li=1`. Thus there are exactly `2*3*4=24` state vertices. `e=(w_H,w_He,2w_He,w_Li,2w_Li,3w_Li)` with fixed positive isotope-weight inputs. Reconstructed source weights are conditional unless an owner-accepted scale array is supplied. In source clock `x=ln(a/a_ref)`, photo rates `gamma=Gamma/H` and recombination rates `kappa=n_b alpha/H` must each lie in independently declared nonnegative intervals, in the fixed order `photo(H0,He0,He1,Li0,Li1,Li2); recomb(H1,He1,He2,Li1,Li2,Li3)`.

The frozen source reduction (VigilODE PR95, `THEORY_KO.md` §4–5) is

\[
 F(x,z;r)=b(r)+P(r)z+(e^Tz)Q(r)z,\qquad
 J=P+(e^Tz)Q+(Qz)e^T.
\]

`P,Q,b` are linear in the twelve rates, with the exact transition topology specified by PR95 equations (4.1)–(4.4). The source-specific Jacobian is affine in `z` for fixed rates and linear in `r` for fixed state. The feedback outer product `(Qz)e^T` must not be dropped. Units: `[x]=1`, `[r]=[F]=1` for dimensionless fractions, `[J]=1` per unit `x`. The following norm is `||v||_w=max_i |v_i|/w_i` for **fixed strictly positive** dimensionless weights `w_i`; this is not an automatic replacement for the physical33 Gram norm.

## R4–R5. Exact finite extremum theorem

For a real matrix `A`, its weighted-infinity logarithmic norm is

\[
 \mu_w(A)=\max_i\left(A_{ii}+\sum_{j\ne i}|A_{ij}|\frac{w_j}{w_i}\right).
\]

Let `V` be the 24 vertices of `S`, and let `sigma=(sigma_j)_{j != i}` run over all `2^5=32` sign patterns. For fixed row `i`, state vertex `v`, and sign pattern define

\[
 L_{i,v,\sigma}(r)=J_{ii}(v,r)+\sum_{j\ne i}\sigma_j\frac{w_j}{w_i}J_{ij}(v,r)
 =\sum_{a=1}^{12} c_a(i,v,\sigma) r_a.
\]

Write `r_a^*(c)=r_a^hi` for `c>=0`, and `r_a^lo` otherwise (a zero coefficient may choose either). Then the **sharp scalar supremum** is

\[
 \boxed{\mu_*:=\max_{z\in S,\ r\in R}\mu_w(J(z,r))
 =\max_{i,v,\sigma}\sum_{a=1}^{12}c_a(i,v,\sigma)r_a^*(c_a).}
\]

Proof: (1) `|t|=max(t,-t)` yields the finite sign representation for every row. (2) For each fixed `r`, each signed row expression is affine in `z`, so its maximum over the convex product simplex occurs at one of its 24 vertices. (3) At fixed `v` and `sigma`, it is affine in the independent rate vector and its maximum is attained by the coefficient-sign endpoint rule. (4) Finite maxima commute, and each selected `v,r*` belongs to the original domain, so this is equality, not merely an upper estimate. **Do not** infer that all individual entry extrema are simultaneously attainable, that the nonlinear RHS has vertex extrema, or that a whole-system eigenvalue has this property. The 24×6×32×12 algorithm evaluates 55,296 signed rate coefficients, plus the finite basis Jacobian work. No 4096-corner rate sweep is required.

An exact witness consists of `(row, state_vertex, signs, chosen_rate_endpoints, value)`; it attains the scalar supremum. A binary64 outward implementation may enclose `mu_*` but cannot claim exact equality of rounded outputs. A simple entrywise bound `max_i(upper(Jii)+sum_j max|Jij|wj/wi)` is valid but loses correlations; compare it only at the same `S,R,w`.

## R5–R7. Residual transport and event connection

For a continuous represented curve `p(x)` in `S`, exact solution `z(x)` in `S`, same clock and rates, and original-equation residual `rho=p'-F(x,p)`, assume a **whole-slab** bound `||rho||_w <= delta`, initial error `||p(x0)-z(x0)||_w <= E0`, and a source proof that `r(x)` lies in the declared box. Since `S` is convex and the chemical source preserves its simplex for nonnegative rates, the mean-value segment between `p` and `z` lies in `S`. Hence

\[
 D^+E\le\mu_* E+\delta,\qquad
 E(s)\le E_0 e^{\mu_*s}+\delta s\,\phi_1(\mu_*s),\quad\phi_1(q)=(e^q-1)/q.
\]

This comparison is valid when the actual solution exists and remains in `S`; source positivity gives inward boundary directions for the restricted chemical block, not full BASS30. For a purely rational, coarse whole-slab bound, let `m=max(mu_*,0)`. If `m h<1`, then

\[
 \boxed{\sup_{s\in[0,h]} E(s)\le (E_0+h\delta)/(1-mh).}
\]

Derivation: on `[0,t]`, `M_t=sup E(s)` obeys `M_t<=E0+t delta+t m M_t` by the Dini integral inequality. For `mh<1` rearrange and maximize. This is a sufficient bound, not an optimal stiff integrator estimate; if `mh>=1`, the rational gate returns unresolved, not a physical failure. If the reconstructed `p` leaves `S`, or if source clock/coefficients/E0 are unbound, this comparison must not be admitted.

For electron-fraction event `g(z)=e^Tz-theta`, its norm dual constant is `L_g=sum_i |e_i| w_i`. Thus `|g(z)-g(p)|<=L_g E`. Strict event signs follow only if the represented event is separated from zero by more than this bound. **Unique crossing** additionally requires a certified sign of `d g/dx=e^T F(x,z)` on the relevant tube/whole slab; opposite endpoint signs alone do not exclude multiple crossings. A fixed whole-simplex `mu_*` cannot substitute for that transversality test.

## R6–R10. Audit, reuse and stop condition

1. Independently verify the 12 source rate-basis matrices against PR95 equations (4.3)–(4.4), including the `(Qz)e^T` term. Wolfram's corrected 3D symbolic generic check returned `{0}` for `J-(P+(e.z)Q+(Q.z)e^T)`; the initial uppercase-symbol CAS attempt produced recursion warnings and is **not** a PASS.
2. Implement an exact-rational source-topology oracle, return attaining witness, compare to an independent full-corner oracle for reduced active-rate cases, and test randomized interior values only as additional diagnostics.
3. Implement a separate native Rust 1.94.1 outward *upper* evaluator and compare its upper bound to exact Fraction `mu_*` on matching represented binary64 inputs. Preserve the original source/binding and do not change `crates/` or Cargo.lock unless full integration/build is actually verified.
4. Preserve negative controls: omitted feedback, vertex-only nonlinear RHS, entrywise correlation loss, negative/NaN rates, zero norm weights, wrong clock, and the `mh>=1` rational gate.
5. Accept only the scoped finite chemical6 lognorm result if tests pass. The existing 23-node/34-edge DAG stays inherited `7/23` with delta0; central REI `28/88` remains separate. Actual BASS30 owner capsule, physical33 norm transfer, whole-slab residual/rate enclosures, query rounding, full Jacobian and production qualification remain **HOLD**.

Primary mathematical background: Arnold Neumaier, *Global, rigorous and realistic bounds for the solution of dissipative differential equations. Part I: Theory*, Computing 52 (1994), original manuscript https://arnold-neumaier.at/ms/ode.pdf (Theorems 2.6–2.8 and existence conditions). G. Söderlind, *The logarithmic norm: History and modern theory*, BIT 46 (2006), DOI 10.1007/s10543-006-0069-9. Source-specific prior: VigilODE PR95 §4–6 and BASS PR233/234. Akrivis–Makridakis–Nochetto (2009), DOI 10.1007/s00211-009-0254-2, supports residual-based reconstruction methods but is not evidence of this model-specific extremum theorem.