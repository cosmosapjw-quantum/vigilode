# Sparse cyclic closure: finite circuit and native whole-path certificate

All variables and time are dimensionless. This is a constructive extension of the two-constraint baseline to n=4,8,16 genuinely coupled algebraic variables. It is a particular structured, nonstiff model. It does not establish a generic algebraic compiler or generic timing advantage.

## Model and the physical branch

Indices are cyclic. Set

G_i(x,q)=q_i+q_{i+1}²/4-x_i,

x_i'=-x_i+(q_i+q_{i+1})/16+c_i, c_i=1/32+(i mod 3)/256,

x(0)=q(0)=0, 0≤t≤1.

Each q_i depends algebraically on the next variable; the cycle closes through all n variables. Slow states depend on two algebraic coordinates. This is neither n uncoupled scalar problems nor root evaluation along an externally prescribed parameter path.

For X=[-1/8,1/8]^n and Q=[-1/4,1/4]^n, T_x(q)_i=x_i-q_{i+1}²/4 maps Q into [-9/64,1/8]^n⊂Q. Its infinity-norm contraction constant is L=1/8. Thus q(x) is unique in Q, with

||q(x)-q(z)||∞≤(8/7)||x-z||∞,

||qhat-q(x)||∞≤(8/7)||G(x,qhat)||∞, qhat∈Q.

The slow field is strictly inward on every face of X: at an upper face it is at most -1/8+1/32+10/256<0; at a lower face it is at least 1/8-1/32+1/32>0. The exact solution therefore remains in X. The reduced field F(x)=-x+Cq(x)+c has ||C||∞=1/8 and one-sided infinity-norm Lipschitz constant at most -1+(1/8)(8/7)=-6/7. Hence it is contractive at rate λ=6/7.

The branch token is `cyclic-small-real-zero`. The initial root is exactly zero. The F1 contract communicated by the atlas task identifies the branch by uniqueness on (X,Q), checks whole incoming/outgoing slab domains and original residuals, and requires a slow-state jump radius if slow coordinates change. This implementation uses one chart throughout. There is no chart switch or coordinate reset, so the slow jump is exactly zero. Every accepted Hermite slab is checked in X×Q; all residual-to-root comparisons use the same unique q(x). The common-domain continuation condition is thus met without inventing a switch. This is an application of the F1 interface, not a test of F1's separate two-variable transition implementation.

## Finite rational circuit and offline growth

Let b_i=q_{i+1}/2 and A_ij=b_i when j=i+1 cyclic, zero otherwise. Then G_q=I+A and A^n=P I, P=∏b_i. The exact finite geometric identity is

(I+A)∑_{k=0}^{n-1}(-A)^k=(1-(-1)^n P)I.

The denominator d=1-(-1)^nP obeys |d|≥1-(1/8)^n on Q. Thus no selected-chart singularity lies in this domain. An expanded matrix inverse would use O(n²) output entries. We compile only its action on one vector f, using the cyclic scalar recurrence v_i=f_i-b_i v_{i+1}:

1. Start r=f_{n-1}, p=-b_{n-1}.
2. For i=n-2,…,0 set r=f_i-b_i r and p=-b_i p.
3. Set v_0=r/(1-p).
4. Set v_{n-1}=f_{n-1}-b_{n-1}v_0 and recover v_i for i=n-2,…,1.

The composed recurrence says v_0=r+pv_0, with p=(-1)^nP. It gives the finite rational circuit directly. Generated code unrolls these steps. Its scalar arithmetic count is exactly 7n-3 when unary negation counts as one operation: 4n-3 multiplications, 2n-1 additions/subtractions, n negations and one division. There are two scalar recurrence temporaries plus two n-vectors b,v. Literal coefficients are only 1 and 1/2; source DAG size grows linearly. The total denominator polynomial has degree n; bounded literal coefficients and linear circuit size do not claim bounded expanded polynomial degree or constant bit complexity.

The compiled lift is q'=v(q,f(x,q)). Since G_q v=f identically wherever d≠0, G'=G_q q'-x'=0 identically, including off the zero set. With consistent initial data the exact lifted solution preserves the original constraints. The online compiled integration contains no Newton iteration, Jacobian callback, generic factorization, JVP, VJP or finite difference. The circuit mathematically encodes the derivative of G; this is not a derivative-free black-box theorem.

The Newton baseline uses the identical O(n) cyclic structured solve, implemented as loops, with a previous-root warm start and a map cache keyed by the exact binary64 x vector. No generic dense LU is imposed on it. Iteration stops when ||G||∞≤2e-16, with a 12-update cap; this stopping test alone is not a root certificate. Original-equation slab residuals subsequently certify the produced path.

## Krawczyk comparison

The method named `krawczyk` uses exactly the same Newton candidate generator, warm start, cache and tolerance, followed by interval Krawczyk inclusion for every new root solve. The known initial root uses an exact zero proof. It is not a standalone Krawczyk candidate generator or an implementation of a complete published homotopy tracker.

For candidate q, let B=q+[-r,r]^n, r=2^-42. Build a binary64 matrix R by n applications of the structured solver to coordinate vectors. R is treated as an exact binary64 preconditioner in the interval proof. Compute

K(B)=q-RG(x,q)+(I-RG_q(x,B))[-r,r]^n.

For each i the code bounds |(RG)_i|+r∑_j |(I-RG_q(B))_ij| and requires strict inequality with r; it also checks B⊂Q. The Jacobian has only diagonal and cyclic-neighbor entries, so (RG_q)_ij=R_ij+R_{i,j-1}q_j/2, and the interval calculation is O(n²), rather than generic O(n³) interval multiplication. The strict radius test also implies ||I-RG_q(q)||∞<1, hence RG_q(q) and R are nonsingular. Strict inclusion therefore proves a root in B; the preexisting global contraction theorem gives its branch identity and uniqueness in Q. All interval arithmetic here is outward. Time spent building R and evaluating inclusion is recorded separately as `closure_krawczyk_seconds`; it remains included in integration time and total online time. The shared polynomial whole-path certificate is applied afterwards to all three methods. Krawczyk endpoint inclusions alone are not advertised as a whole-path proof.

## Native outward polynomial certificate

Each stored endpoint is a binary64 value interpreted as an exact dyadic rational. N is restricted to powers of two; h=1/N is therefore exact in binary64. Endpoint slopes are the exact rational compiled lift evaluated with interval arithmetic, producing enclosures of the exact slopes. For normalized slab coordinate θ∈[0,1], each component uses the cubic Hermite polynomial

H(θ)=y0+h f0 θ+[3(y1-y0)-h(2f0+f1)]θ²+[2(y0-y1)+h(f0+f1)]θ³.

The mathematical reported candidate uses exact rational f0,f1. The floating implementation holds interval coefficients enclosing that exact polynomial; there is no assumption that a rounded scalar slope equals the exact slope. Degree-six algebraic residuals and degree-three slow residuals are formed in the power basis, using outward interval operations:

R_g,i=Q_i+Q_{i+1}²/4-X_i,

R_f,i=X_i,θ/h+X_i-(Q_i+Q_{i+1})/16-c_i.

For a power polynomial ∑a_k θ^k of degree m, its Bernstein coefficients are β_j=∑_{k≤j} [binom(j,k)/binom(m,k)]a_k. Each integer binomial is exactly representable here; each rational ratio is itself computed by outward interval division. The convex hull of the Bernstein coefficient intervals encloses the polynomial over the entire slab. This checks X,Q domains and yields Dg,Df. Sampling is not used to prove a supremum.

Each primitive +,-,*,/ evaluates the endpoint operation in round-to-nearest binary64 and applies `nextafter` toward ±infinity. Multiplication considers four endpoint products. Division requires a denominator interval excluding zero and encloses its reciprocal before multiplication. Unary negation is exact. This enclosure strategy includes underflow, with FTZ/DAZ disabled. The executable checks IEEE binary64, runtime FE_TONEAREST, and on this x86/SSE build the FTZ/DAZ flags; the build disables fast-math and FMA contraction. Inputs and all observed computations are finite, far below overflow. This is not a verified compiler or a proof for arbitrary floating hardware. The exact Fraction comparisons check the executable's enclosures on representative data and supplement, rather than replace, the primitive enclosure argument.

Using the residual-to-root bound, reduced slow defect is δ=Df+Dg/7. For incoming slow radius E, the Dini inequality e'≤-(6/7)e+δ gives the safe bounds

Ewhole=E+hδ,

Eend=E/(1+6h/7)+hδ,

Qwhole=(8/7)(Ewhole+Dg).

The second bound follows from exp(-λh)≤1/(1+λh), and (1-exp(-λh))/λ≤h. Every constant division and accumulation operation above is outward in the native implementation, including the non-dyadic 6/7 and 8/7. Start E=0 exactly and propagate the outward Eend. The physical budget is max(Ewhole,Qwhole)≤1e-8 over every slab of [0,1]. It bounds all slow and algebraic coordinates of the single original DAE branch, rather than only endpoints or a transformed state.

## Campaign, validation and limits

Run `python run.py`, then `python tests.py`. The compiler command is recorded in results.json and uses `g++ -std=c++17 -O2 -fno-fast-math -ffp-contract=off -frounding-math`. The small campaign tests n=4,8,16; N=16,32,64,128; compiled, Newton and Newton-plus-Krawczyk; seven in-process timed repeats for each configuration. Raw repeats, medians, extrema, call counts, actual inclusion costs, integration time, shared whole-path certificate time and total online time are retained. Total excludes process startup and JSON formatting. Offline code generation and isolated circuit compilation time, object bytes and whole native executable compilation time are reported separately, without hiding their one-time cost in a steady-state speed claim. Isolated circuit compile probes measure actual generated circuit growth; they are not full end-to-end builds per dimension.

Every method first meets the tested physical budget at N=64; N=16 and 32 fail that budget and remain recorded as failures of those resolutions. This is the first passing tested resolution, not a claim of globally optimal or adaptively minimal work. The certificate exceeds most integration costs. Consequently compiled-versus-Newton total times are close and may reverse order under timing noise; the data do not support a universal speedup. Krawczyk adds real post-Newton interval work; it is an extra certification guarantee at RHS closure calls, whereas all methods share the final whole-path guarantee.

Validation consists of 204 interval cases with 816 exact Fraction corner-operation assertions, 15 exact cyclic inverse identities, and 27 exact Fraction slab comparisons (first/middle/last slabs of the matched configurations). Each comparison verifies native bounds for Dg,Df,Ewhole,Eend,Qwhole against the exact rational calculation at the same incoming binary64 radius. A separate eight-test suite rechecks those slabs, branch constants, native budget results, accounting, circuit growth, rejected invalid configurations and Krawczyk rejection of six deliberately invalid candidates. Exact reference validation is representative, not exhaustive over all floating inputs. Distinct configurations, repeated integrations, slabs and unit-test counts are kept separate in results.json.

Remaining obligations are independent review, an atlas with actual switches for this larger family if its domain is extended, arbitrary sparse systems and conditioning regimes, generic compiler preprocessing growth, stiff integration, broader hardware/compiler validation and large campaigns with rigorous statistics. None is silently promoted from this finite cyclic experiment.
