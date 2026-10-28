# Constructive theory for derivative-free causal and homotopy execution

Date: 2026-10-27 KST. Baseline: PR90, commit
`68a28357d960c4c2743c299770f0a33ef63245b2`.
This unit executes the user-supplied PhysMath Research Loop 1.0.0, R1–R10,
with coding contracts transferred to the three native implementation lanes.

The mathematical result is a usable decomposition of the intended design:

1. Preserve the original stage equations while memoizing repeated causal work.
2. Use exact physical quotient structure to remove derivative callbacks and
   matrix construction when that structure is present.
3. Solve a directly supplied RHS fixed-point homotopy with a screened,
   vector-only depth-one extrapolation.
4. Transport target residuals through a finite causal majorant, rather than
   require a small global contraction constant for a triangular stage system.

These constructions complement one another. The first still uses the existing
frozen-base JVP basis. The second is a runtime derivative-oracle-free exact
specialization. The third has an interface containing only a map evaluation;
it does not construct a Jacobian, finite-difference JVP, dense matrix, or Newton
correction. Their distinct assumptions and actual cost receipts are retained.

## 1. Reused ideas and precise additions

The committed sources already contain finite causal propagation, a shared
diagonal projected factorization, residual-first derivative dispatch, causal
residual majorants, declared polynomial resolvents, and a fixed-P continuation
interface. This research uses those results rather than claiming them again.

| Prior source | Result reused | Addition in this unit |
|---|---|---|
| PR65, `common_w_accelerator_parity_20260929` | Repeated corrections can cost more than a sequential stage solve. | Compare exact work and dependency depth separately; cache actual repeated inputs. |
| PR74–75, `vigilode-coupled` | Raw-U target, original-space residual screen, counted parallel batches, residual-first policy. | Bitwise input cache and an unchanged-RHS solve-prefix cache. |
| `production_frontier_20261023/analysis/CAUSAL_TRAJECTORY.md` | Unique causal root and finite propagation; an explicit stage-error recursion. | General graph/prefix formulation and exact floating-execution reuse theorem. |
| PR77 and derivative-free originals | Collective/transverse closure, polynomial quotient, physical structural action, selective local AD. | An exact two-scalar-tableau consumer for every state and forcing in an immutable mean-field affine family. |
| `fixed_point_continuation.rs` | Whole-domain fixed-P contraction patches and conditional root-error bounds. | A generic map-only, no-inverse-callback homotopy with screened depth-one extrapolation. |
| PR90 theory | Residual/forward-error distinction and causal defect propagation. | Explicit residual transport for the new quotient and homotopy implementations. |

Primary literature was checked using SciSpace and Exa. The 1991 parallel
iterated-RK paper distinguishes total stages from sequential stages; the
Anderson literature supplies the standard extrapolation formula; the
Rosenbrock-K paper supplies the relevant warning that an approximate operator
has its own order conditions. The exact cache proof, sealed affine consumer,
and code-specific screened-iteration argument below are the present
construction. A broad claim of new general operator theory is not made.

## 2. Target, units, and assumptions

For an autonomous, identity-mass ODE `y'=f(y)`, use the repository's raw-U
RODAS5P target with `s=8` stages:

\[
 (I-h\gamma J_0)U_i
 =h\gamma f\left(y+\sum_{j<i}a_{ij}U_j\right)
   +\gamma\sum_{j<i}c_{ij}U_j.                 \tag{1}
\]

Here `U_i` has the units of `y`, `h` has time units, `J_0` has inverse-time
units, and `a,c,gamma` are dimensionless. Both coefficient arrays are strictly
lower triangular. `J_0` is the exact frozen base derivative for the original
RODAS target when temporal-order inheritance is claimed. A model-supplied
operator can instead define a different target, but its temporal order then
needs its own argument.

The projected implementation uses a fixed orthonormal basis `V`, a fixed
frozen action `J_0 V`, and `W_m=I-h gamma V^T J_0 V`. All proofs about a native
attempt assume fixed time, state, basis, coefficients, model epoch, arithmetic
environment, and pure deterministic callbacks for that attempt. A diagnostic
counter may count invocations; numerical callback outputs and success/failure
must not depend on that counter if replay parity is claimed.

The original-space residual screen remains a separately executed numerical
check. A projected root and a small sampled residual acquire forward-error
meaning only through the domain-valid inverse/Lipschitz information used below.

## 3. Theorem A: exact memoization of a causal floating computation

### A.1 Statement

Consider a deterministic represented-arithmetic program evaluating

\[
 x_i(z)=\operatorname{fl}\left(\sum_{j<i} a_{ij}z_j\right),\qquad
 R_i(z)=\operatorname{Project}\left[
 f(y+Vx_i(z))-f_0-(J_0V)x_i(z)\right].        \tag{2}
\]

The sum order and each subsequent operation order are fixed. A cache stores
the full bit pattern of `x_i` and the fully evaluated represented `R_i`.
All other inputs in (2) are immutable over the lifetime of the cache.

**Theorem A.** Reusing the stored `R_i` whenever every bit of `x_i` matches
produces the same represented remainder as recomputation. If an affine
block-lower-triangular solve uses a fixed factorization and a fixed operation
order, a maximal prefix of unchanged RHS blocks can also reuse the previous
solution prefix. Computing the remaining suffix in the original order yields
the same represented solution vector as a complete solve.

Consequently, with sufficient common budgets and the stated callback contract,
`BatchedPicardReuse` has the same iterates, residual decisions, stages, and
endpoint values as `BatchedPicard`. Its executed work counters differ by the
omitted evaluations/solves.

### A.2 Proof

Bit equality of the complete input vector in (2), together with immutable
operands and a fixed deterministic arithmetic program, gives bit equality of
the lifted displacement, frozen action, physical state, callback result,
subtractions, and projection. This is equality of executed programs, not a
continuity or small-error argument. Keeping only the physical state's bits
would be insufficient: two projected displacements could round to the same
physical state while producing different frozen-action terms.

For the solve, let the block matrix be `A` with nonsingular diagonal blocks
and let `b,b_old` agree in blocks `0,...,p-1`. Block zero executes the same
arithmetic from the same RHS and factorization, so its represented solution
is unchanged. If blocks before `i<p` agree, the ordered dot/subtraction
forming block `i` agrees, followed by the same deterministic diagonal solve.
Induction proves equality of the entire prefix. Starting the suffix with that
prefix, the same induction proves that every remaining block agrees with a
fresh solve. Combining the two replacement identities proves the iterated
algorithm claim. QED.

### A.3 Native contract and failure scope

`causal.rs::picard_reuse` keys the entire represented projected displacement,
checks finite values and the model epoch before and after batches, keeps the
cache local to a physical step/basis, joins all scheduled callbacks, and merges
their attempted work before propagating a failure. `affine_solve_reuse` tests
RHS blocks with `to_bits()` and uses the original compensated dot and common
LU solve for the suffix. Signed zero is included in the key/comparison.

The proof guarantees the numerical recurrence on the common successful
domain. Lower work can allow the new route to finish under a budget where the
old route stops. An omitted callback cannot reveal a future external side
effect or a call-count-dependent failure; such behavior violates the pure
replay premise rather than becoming a hidden numerical certificate.

The existing raw-U full-space check is unchanged. Cache parity neither needs
nor creates a new approximation-order assumption.

## 4. Theorem B: finite causal propagation and cost/span accounting

Let blocks form a finite directed acyclic dependency graph. A node `i` has
an exact evaluation `G_i(x_pred(i))`; its predecessors are strictly earlier
in a topological order. Assume all evaluations are defined on the values
visited. The graph has a unique causal root obtained by evaluating each node
once after its predecessors.

Assign depth zero to source nodes and depth `1+max depth(parent)` otherwise.
Suppose the initial iterate already equals the root at source nodes. Under
simultaneous iteration `x^{k+1}=G(x^k)`, all nodes of depth at most `k` equal
the causal root after `k` updates. Therefore all nodes are exact after at most
the maximum depth. This statement requires no global contraction constant.

**Proof.** A source value is independent of the iterate. If every predecessor
of a node is already exact, its next evaluation is the defining exact root
value. Previously exact nodes remain exact because their predecessor values
are unchanged. Induction on depth gives the assertion. QED.

For a chain of `s` stages with the first stage exact initially, evaluating
all `s-1` nontrivial stages before each of at most `s-1` updates and once at
the final residual screen uses at most `s(s-1)` such evaluations. Caching a
node after its full precursor ceases to change uses at most

\[
 (s-1)+(s-2)+\cdots+1=\frac{s(s-1)}2             \tag{3}
\]

evaluations in exact arithmetic. For `s=8`, the constructed maximal-chain
example has `56 -> 28` evaluations with identical iterates. This is an
independent rational example, not a native benchmark result.

The actual projected Picard iteration uses a causal affine inverse between
remainder batches. Its remainder for node `i` depends only on earlier stages;
the same information-propagation induction applies. Represented rounding may
prevent theoretical stabilization from becoming bit equality, so the native
implementation uses observed exact keys and still enforces its residual and
iteration limits. Its realized saving is measured, not assigned from (3).

### Work and span are different quantities

With `k` active batch sizes `a_1,...,a_k`, callback cost `C_f`, `p` workers,
and batch overhead `B`, an idealized callback-only work/span model is

\[
 W=C_f\sum_\ell a_\ell,\qquad
 S_p\approx C_f\sum_\ell\lceil a_\ell/p\rceil+kB.       \tag{4}
\]

Serial affine solve depths, projection/lifting, and validation must be added.
Cache reuse reduces the work summation and some solve suffixes. It does not
remove the remaining inter-batch dependencies. A direct forward causal solve
still uses only `s-1` fresh RHS evaluations, so fewer redundant batch calls
alone do not establish superiority to that distinct algorithm. The requested
parallel speed claim must be based on the complete implementation at matched
accuracy, including overhead and validation.

## 5. Theorem C: residual bounds for causal roots without global contraction

Let `X=X_0 x ... x X_{s-1}` with a norm on each block. Let
`G_i:X_0 x ... x X_{i-1} -> X_i` be causal. Suppose a root `x*` and a candidate
`x` are in a domain on which

\[
 \|G_i(x_{<i})-G_i(z_{<i})\|
 \le\sum_{j<i}L_{ij}\|x_j-z_j\|,\quad L_{ij}\ge0.       \tag{5}
\]

Set `r_i=G_i(x_<i)-x_i`, `e_i=||x_i-x_i*||`, and `rho_i=||r_i||`.
Then

\[
 e\le M\rho,\qquad
 M=(I-L)^{-1}=I+L+\cdots+L^{s-1}.                       \tag{6}
\]

The inverse is entrywise nonnegative. No assumption `||L||<1` is required.
An output `z=sum_i beta_i x_i` consequently has root error at most
`sum_i |beta_i| (M rho)_i`.

**Proof.** Adding/subtracting `G_i(x_<i)` yields `e<=rho+Le` componentwise.
Iterating this inequality `s` times yields
`e<=sum_{k=0}^{s-1}L^k rho+L^s e`. Strict lower triangularity gives `L^s=0`.
The same finite geometric identity proves the expression for `(I-L)^-1`.
The output follows by the triangle inequality. QED.

For a globally defined continuous causal map, the root exists uniquely by
forward substitution. On a restricted physical domain, that existence
argument also needs each recursively constructed value to remain in the
domain. Domain-valid (5), rather than a sampled slope at the final iterate,
is the assumption used by the error bound.

### Homotopy branch and a constructive weighted norm

For `G_lambda(x)=(1-lambda)a+lambda G(x)`, replace `L` by `lambda L`.
For `0<=lambda<=mu<=1`, the roots satisfy

\[
 e_{\mu,\lambda}
 \le(\mu-\lambda)(I-\mu L)^{-1}
       |G(x^*(\lambda))-a|_{\rm block}.               \tag{7}
\]

To prove (7), subtract the two root equations, separate the change in `x`
from the change in `lambda`, apply (5), and then the same finite inverse.
This supplies a finite branch-motion estimate even when an unweighted norm
of the causal map is greater than one.

For any selected `0<q<1`, positive weights can also be built recursively:
`w_0=1` and

\[
 w_i\ge\max\left(1,q^{-1}\sum_{j<i}L_{ij}w_j\right).  \tag{8}
\]

Then the block weighted maximum norm `max_i ||x_i||/w_i` gives
`||G(x)-G(z)||_w<=q||x-z||_w`. This is useful as an existence/convergence
representation, but the weights can be large and converting back to physical
units can amplify the bound. Formula (6) is the direct implementation form;
one must retain those weights or finite propagation factors when reporting
physical error.

An exact check uses `||L||_infinity=13`, first-block residual `10^-6`, and
last-block root error `119*10^-6`. It demonstrates why a residual tolerance
should be transported rather than equated with a state-error tolerance.

## 6. Theorem D: exact derivative-free quotient for a mean-field affine family

Let `n>=1`, `P v=mean(v) 1`, `Q=I-P`, and consider

\[
 f(y)=b-dy+\kappa P y,\qquad d\ge\kappa\ge0.            \tag{9}
\]

All components have common state units. The constants `d,kappa` have
inverse-time units, `b` has state/time units, and the model is immutable
throughout the integration. State and forcing
are arbitrary n-vectors. Define `delta_c=d-kappa`, `delta_t=d`.

Because `P^2=P`, `Q^2=Q`, `PQ=QP=0`, the exact derivative is the structural
operator

\[
 J=-\delta_cP-\delta_tQ,\qquad
 (I-h\gamma J)^{-1}
 =\frac{P}{1+h\gamma\delta_c}
  +\frac{Q}{1+h\gamma\delta_t}.                        \tag{10}
\]

For `h gamma>=0` both denominators are at least one. No state matrix needs
to be formed to apply either formula. In Euclidean norm `P,Q` are orthogonal
projections; thus the inverse norm is
`1/(1+h gamma delta_c)<=1`.

Let `f0=f(y)`. For each `delta` in `{delta_c,delta_t}`, define eight scalar
time-valued coefficients recursively by

\[
 t_i(\delta)=
 \frac{h\gamma+
 \sum_{j<i}(\gamma c_{ij}-h\gamma\delta a_{ij})t_j(\delta)}
 {1+h\gamma\delta}.                                  \tag{11}
\]

**Theorem D.** The unique original raw-U root (1) for model (9) is

\[
 U_i=t_i(\delta_c)P f_0+t_i(\delta_t)Q f_0.             \tag{12}
\]

Only two scalar tableaux and vector averaging/lifting are required to construct
the stages. This construction costs `O(s^2+s n)` and storage `O(s n)` if all stages
are retained, or `O(n+s)` if only the corresponding scalar/stage outputs
are needed. There is no Jacobian callback, JVP/AD call, dense/reduced state
matrix, nonlinear Newton iteration, or approximate stage root in this
exact-arithmetic construction.

The retained direct original-space screen additionally costs `O(s^2 n)`
arithmetic in the present implementation. Its work is charged in both native
arms. Thus the construction improvement does not remove the screen's cost;
for the fixed eight-stage method both complete arms remain linear in `n`.

**Proof.** Equation (9) gives
`f(y+v)=f0+Jv`. Substitute this into (1):

\[
 (I-h\gamma J)U_i=h\gamma f_0+
 \sum_{j<i}(h\gamma a_{ij}J+\gamma c_{ij}I)U_j.         \tag{13}
\]

The subspaces `range(P)` and `range(Q)` are invariant, and `J` acts on them
by the scalars `-delta_c` and `-delta_t`. Project (13) onto either subspace;
induction on `i` yields `P U_i=t_i(delta_c)P f0` and
`Q U_i=t_i(delta_t)Q f0`, where (11) is exactly the projected recurrence.
Their sum proves (12). Nonsingularity in (10) and causal substitution give
uniqueness. QED.

The two scalar tableaux suffice even when `b` has arbitrary transverse
components: the initial derivative `f0` already combines the initial state
and forcing on each invariant subspace. Restricting the input to a uniform
state or a particular spatial pattern is unnecessary.

### D.1 Original temporal order and observations

Since (12) solves the original target with its exact `J`, every exact
endpoint and embedded combination is the same as the ordinary RODAS5P
formula. Its classical order on this model is inherited from that formula;
no new approximate-Jacobian order assumption is introduced. Binary64
quotient evaluation and a full n-dimensional sequential evaluation may
round differently, so bitwise equality between those two algebraically
equivalent implementations is not required.

An independent exact observation formula is

\[
 y(t)=e^{-\delta_ct}P y_0+e^{-\delta_tt}Q y_0
       +t\varphi_1(-\delta_ct)P b
       +t\varphi_1(-\delta_tt)Q b,\quad
 \varphi_1(z)=\frac{e^z-1}{z},\quad\varphi_1(0)=1.      \tag{14}
\]

Use `expm1` near zero in a floating reference. Equations (10)–(14) include
the nondecaying collective mode `d=kappa` and the constant-forcing case
`d=kappa=0` without dividing by a zero decay rate.

### D.2 Scalar defects, lifting error, and a full-space check

For approximate scalar coefficients, define

\[
 \epsilon_i(\delta)=(1+h\gamma\delta)\widehat t_i(\delta)
 -h\gamma-
 \sum_{j<i}(\gamma c_{ij}-h\gamma\delta a_{ij})
       \widehat t_j(\delta).                          \tag{15}
\]

If the vectors in (12) are lifted exactly, the original stage residual is

\[
 \rho_i=\epsilon_i(\delta_c)P f_0+
         \epsilon_i(\delta_t)Q f_0.                  \tag{16}
\]

Orthogonality gives the exact squared Euclidean norm
`||rho_i||^2=epsilon_c^2||P f0||^2+epsilon_t^2||Q f0||^2`.
If the represented stages have lifting perturbations `ell_i`, add

\[
 (I-h\gamma J)\ell_i-
 \sum_{j<i}(h\gamma a_{ij}J+\gamma c_{ij}I)\ell_j      \tag{17}
\]

to (16). Bounds for the represented `f0`, scalar arithmetic, dot means, and
residual evaluation must also be included if a rigorous floating enclosure
is claimed. A direct full-space evaluation of (1) is a useful operational
screen and charges its actual work; the scalar identity does not authorize
deleting that screen without the missing rounding allowances.

For a residual-to-stage bound on this affine family, set

\[
 K=(1+h\gamma\delta_c)^{-1},\qquad
 L_{ij}=\max_{\delta\in\{\delta_c,\delta_t\}}
 \left|\frac{\gamma c_{ij}-h\gamma\delta a_{ij}}
 {1+h\gamma\delta}\right|.                         \tag{18}
\]

Subtract (13) for exact and approximate stages. Simultaneous diagonalization
by the orthogonal projectors gives `e<=K|rho|+Le`; Theorem C gives
`e<=K(I-L)^-1 |rho|`. This can be substantially sharper than replacing every
coefficient with a product of unrelated worst-case norms.

### D.3 Exact applicability and selective derivative use

The closure follows from the declared immutable model, not from matching a
few sampled RHS values. Perturbing (9) by a general nonlinear term or a
spatially heterogeneous rate can destroy the two-projector algebra.
The correct repair is to retain its explicitly computed defect, expand the
structural family, or evaluate a permitted derivative-based candidate with
charged measured work. Replacing `J` by zero while leaving all raw-RODAS
coefficients unchanged does not inherit this theorem.

## 7. Theorem E: screened vector-only homotopy acceleration

Let `D` be a closed subset of a Banach space and let
`G_lambda:D->D` be a contraction with constant `q_lambda<1`. Write
`r_lambda(x)=G_lambda(x)-x`. At the current iterate `x`, compute the Picard
point `p=G_lambda(x)` and evaluate its actual residual. Construct any
domain-admissible trial `e`. Choose `e` only if

\[
 \|r_\lambda(e)\|\le\eta\|r_\lambda(p)\|,
 \qquad 0\le\eta\le1;                              \tag{19}
\]

otherwise choose `p`. Then the chosen point `x+` obeys

\[
 \|r_\lambda(x^+)\|\le q_\lambda\|r_\lambda(x)\|,
 \qquad
 \|x-x^*_\lambda\|\le
 \frac{\|r_\lambda(x)\|}{1-q_\lambda}.               \tag{20}
\]

**Proof.** Since `p=G_lambda(x)`,
`r_lambda(p)=G_lambda(G_lambda(x))-G_lambda(x)`.
Contractivity bounds its norm by `q_lambda||r_lambda(x)||`. The acceptance
rule chooses a point with residual no larger than this Picard residual.
For the second inequality, subtract the fixed-point relation and use
`||x-x*||<=||x-G_lambda(x)||+q_lambda||x-x*||`.
Banach's theorem supplies the unique fixed point. QED.

The proof does not require differentiating `G`, forming a secant matrix,
or assuming that a proposed extrapolate improves the residual. The
evaluation in (19) supplies the necessary comparison.

### E.1 Concrete depth-one trial and computational meaning

The implementation uses two consecutive same-station pairs `(x_old,r_old)`
and `(x,r)`. In an inner-product space, set

\[
 \Delta r=r-r_{old},\qquad
 \alpha=\frac{\langle r,\Delta r\rangle}{\|\Delta r\|^2},
 \qquad
 e=G_\lambda(x)-\alpha\left(G_\lambda(x)-G_\lambda(x_{old})\right).
                                                               \tag{21}
\]

This is the standard depth-one Anderson trial: `alpha` minimizes
`||r-alpha Delta r||^2`. A small/degenerate denominator, a nonfinite value,
an excessive coefficient, or an inadmissible domain point disables that
trial. The proof of (20) remains valid for every admitted construction of
`e`; the coefficient cap regulates finite precision and cost rather than
serving as an unproved convergence condition.

Only vectors, dot products, scalar arithmetic, and direct map evaluations
are used. All rejected trial evaluations count toward work. Reuse the
already evaluated chosen map value as the next state's cache where the
callback contract permits it. Reset history when `lambda`, map identity,
epoch, anchor, scaling, or domain contract changes.

For a scalar affine map `G(x)=q x+b`, `q!=1`, and two distinct iterates,
(21) gives `e=b/(1-q)` in exact arithmetic. This explains a strong favorable
case, including some noncontractive values of `q`; it does not prove
convergence for every nonlinear map with `q>=1`.

### E.2 Continuation branch displacement

For the implemented affine-in-parameter homotopy

\[
 G_\lambda(x)=(1-\lambda)a+\lambda G(x),              \tag{22}
\]

assume `D` is closed and convex, `a` is in `D`, and `G:D->D` is
q-Lipschitz. Convexity makes every `G_lambda` a self-map of `D`, and its
Lipschitz constant is `lambda q`. One may instead separately establish
the common self-map property on a nonconvex domain. Whenever `mu q<1`,

\[
 \|x^*_\mu-x^*_\lambda\|
 \le\frac{(\mu-\lambda)\|G(x^*_\lambda)-a\|}{1-\mu q}.
                                                               \tag{23}
\]

Indeed, insert `x_lambda*` into the `mu` root equation, add/subtract
`mu G(x_lambda*)`, and absorb the `mu q` term. For a known bound `B` on
`||G(x_lambda*)-a||`, choosing a continuation increment with
`Delta lambda B/(1-mu q)` below the admissible root-neighborhood radius
gives a sufficient transport condition.

For a causal stage map with a finite lower-triangular bound, equation (7)
provides the corresponding finite transport formula even if `mu q>=1` in
the unweighted norm. The generic implementation's ordinary residual
threshold remains an operational stopping test unless the caller supplies
these bounds and floating evaluation allowances.

### E.3 Represented evaluations and an honest certificate extension

If the computed Picard point differs from `G_lambda(x)` by at most `eps_p`,
then

\[
 \|r_\lambda(\widehat p)\|
 \le q_\lambda\|r_\lambda(x)\|+(1+q_\lambda)\epsilon_p. \tag{24}
\]

This follows by adding/subtracting the exact Picard point and using the
Lipschitz bound for `r=G-I`. An interval-certified comparison
`R_upper(e)<=eta R_lower(p_hat)` preserves (19) against the mathematical
residual. Plain floating norm comparisons test represented residuals; they
do not by themselves produce those interval bounds. The new solver's
receipts therefore keep the native operational test distinct from the
conditional exact-arithmetic theorem.

Root error and ODE error also remain distinct. If stage errors satisfy
Theorem C and the one-step endpoint is `y+sum beta_i U_i`, its algebraic
error is bounded by `sum |beta_i| e_i`. A stable order-p step map retains
global order p when this added local algebraic error is `O(h^(p+1))` with
uniform constants; this is the existing causal-budget/Grönwall argument.
Changing the step target or its approximation order requires a separate
order calculation.

### E.4 Nonlinear implicit-RK consumer with an independent root bound

The native experiment also applies the map solver to the implicit midpoint
increment for the scalar ODE

\[
 y'=-k y-g y^3,\qquad k>0,\quad g\ge0,\quad h>0.
\]

With anchor zero, write `Phi(z)=k z+g z^3` and

\[
 F_\lambda(K)=K+\lambda h\Phi(y+K/2),\qquad
 G_\lambda(K)=-\lambda h\Phi(y+K/2).                 \tag{25}
\]

For every real `K`,

\[
 F_\lambda'(K)=1+\frac{\lambda h}{2}
     [k+3g(y+K/2)^2]\ge1+\lambda h k/2>0.           \tag{26}
\]

The polynomial is onto and strictly increasing; hence its root is globally
unique, including at `lambda=0`. The mean-value theorem gives the sharper
global forward-error estimate

\[
 |K-K^*_\lambda|\le
 \frac{|F_\lambda(K)|}{1+\lambda h k/2}.              \tag{27}
\]

For independent vector components, apply (27) componentwise and take the
chosen norm. This bound remains useful when the Picard map is noncontractive;
for the linear case, its Lipschitz constant is `lambda h k/2`, which can
exceed one. The root bound does not assert convergence of arbitrary
iterations to that root.

For `y>=0`, the independent bisection interval is `[-2y,0]`: the endpoint
residuals are `-2y` and `lambda h Phi(y)`. For `y<0`, reverse the interval
to `[0,-2y]`; `y=0` has root zero. A root enclosure built from these endpoint
signs is an independent comparator to the fixed-point iteration. Subtracting
`F_mu(K_lambda*)` from `F_mu(K_mu*)=0` also yields

\[
 |K^*_\mu-K^*_\lambda|\le
 \frac{(\mu-\lambda)h|\Phi(y+K^*_\lambda/2)|}{1+\mu h k/2}.
                                                               \tag{28}
\]

Thus both branch transport and residual-to-root accuracy are available
without a contraction assumption for this physical consumer. The symbolic
derivative in the proof is an analytical bound; the implemented solver does
not evaluate it or request a derivative callback.

An independent continuous solution used to score the midpoint trajectory is

\[
 y(t)=\frac{y_0 e^{-kt}}
 {\sqrt{1+(g/k)y_0^2(1-e^{-2kt})}},\quad t\ge0.       \tag{29}
\]

Direct differentiation verifies the ODE and initial condition. The midpoint
time discretization has its own order-two truncation error; convergence to
the root of (25) does not remove that error. Its numerical residual is an
operational binary64 value until evaluation/rounding allowances are supplied
for the inequality (27).

## 8. Cost selection with a derivative-free preference

The user-defined preference is implemented mathematically as a policy over
qualified candidates. A successful derivative-free candidate retains
priority until evidence at the same target/accuracy contract supports a
permitted derivative-based candidate. Historical or current trials must
charge setup, failed probes, residual validation, and fallback before
comparing total cost.

Let qualified routes have complete same-window costs `C_DF,C_J`. A switch
to a permitted JVP/AD route at ratio `rho<1` is justified by
`C_J<=rho C_DF` for that window. If a completed derivative-free candidate
costs `E` and a running competitor has monotone accumulated cost `L`,
`E<=rho L` proves that competitor can no longer win under that criterion;
this reuses PR90's prefix-cost theorem. No observed cost proves what a
future changed regime will cost. Reuse of a selection certificate should
therefore be tied to its model/epoch, output target, physical error norm,
and regime evidence rather than the name of an algorithm.

The present root policy and experiments determine which concrete routes
meet their actual acceptance contract. This theorem authorizes neither a
new global default nor uncharged speculative derivative work.

### 8.1 Same-checkpoint entry-pilot proposal reuse

A newly observed adapter experiment exposed a separate startup cost: an
explicit session had an already available proposal at the replay checkpoint,
but every implicit entry pilot reset to the global initial `h0`. The new
opt-in repair changes only the pilot's initial proposal. Let the accepted
replay interval have length `H>0`, let `h0>0` be the original requested
proposal, and let `h_e>0` be the explicit checkpoint's proposed step recorded
**before** advancing that explicit batch. The candidate proposal is

\[
 h_{\rm candidate}=\min\{H,\max(h_0,h_e)\}.           \tag{30}
\]

The configured native input retains the existing minimum-step floor; the
native driver clips its actual first attempt to the endpoint, including
representable intervals shorter than that floor. Nonfinite/nonpositive
checkpoint proposals fall back to the original requested proposal. The
option is false by default, including factory configurations.

**Proposition.** If the physical checkpoint, ODE, replay endpoint, error
norm/tolerances, native acceptance checks, callback-work ceiling, attempt
caps, and final same-window comparison predicate are unchanged, replacing
the replay's initial proposal by (30) preserves that replay's target and
selection qualification contract. A selected implicit route still requires
an actually completed, qualified, cheaper same-window replay. Every failed
or discarded pilot remains part of the charged work.

**Proof.** The only changed input is the starting step proposal. Every native
trial, including the first larger trial, is still evaluated by the same
accept/reject criterion and charged before the unchanged ceiling and attempt
checks. The physical initial state and endpoint are unchanged, so a successful
replay approximates the same stated IVP/output interval. The unchanged final
predicate continues to require completion, qualification, and the observed
cost inequality before routing. Unsuccessful pilots cannot supply such an
endpoint and their recorded work is still accumulated. QED.

This proposition does not require the explicit proposal to be an error bound
for an implicit method. It is a starting guess subjected to the implicit
method's own tests. Qualification here means native completion and local
acceptance; the experiment's analytic trajectory-error cap is checked
separately. Since adaptive meshes and later routing may change,
the two complete integrations need not be bitwise identical; compare them
using the same independently evaluated physical accuracy cap.

The startup mechanism can also be quantified. In real arithmetic, suppose
the first accepted step is at most `h_s` and subsequent accepted steps grow
by at most `g>1`. Covering a span `H` with `m` accepted steps requires

\[
 H\le h_s\frac{g^m-1}{g-1},\qquad
 m\ge\left\lceil\log_g\left(1+\frac{(g-1)H}{h_s}\right)\right\rceil.
                                                               \tag{31}
\]

The bound follows by summing the geometric upper bound on accepted step
lengths; rejections add work. Larger available checkpoint proposals reduce
this particular startup lower bound. Actual accepted mesh, rejection cost,
and route preference remain measured quantities. This repair addresses the
old B3 cold-seed mechanism rather than weakening the derivative-free
preference or treating a method's name as performance evidence.

## 9. Adversarial checks and resolved obligations

| Object | Observed issue or risk | Constructive treatment |
|---|---|---|
| Old repeated Picard batch | Previously settled precursor values were recomputed. | Exact represented precursor cache; full-space screen remains. |
| Solve-prefix reuse | Reusing a non-prefix row can invalidate lower-triangular dependence. | Reuse only the maximal unchanged RHS prefix and recompute suffix in original order. |
| Small residual | Causal amplification can exceed one. | Finite majorant (6), demonstrated by a tight affine example. |
| Mean-field quotient | Arbitrary initial/forcing transverse information must be retained. | Lift both `P f0` and full `Q f0`; no uniform-state assumption. |
| Approximate base operator | Old temporal order cannot be inherited by a changed target. | Exact structural `J` for the sealed family; retain order obligation elsewhere. |
| Anderson trial | Optimized linear combination of old residuals need not equal the true new residual. | Evaluate and screen the actual new residual against computed Picard. |
| Continuation | Cross-station history changes the map being approximated. | Reset history and bound branch displacement using (23) or (7). |
| Wolfram first expression | `Sum[MatrixPower[L,k],{k,0,3}]` left a singular zero-power expression unevaluated. | Replace with explicit identity plus positive powers; corrected four identities returned True/zero. Original response retained. |

The finite exact checker independently verifies 32 arbitrary-state/forcing
mean-field cells (832 stage-component equalities, all original residuals
exactly zero), the nilpotent inverse, an eight-stage cache example, and 12
scalar affine acceleration cases. These are algebra/indexing audits; the
general proofs above do not rely on extrapolating finite tests.

No new strong no-go theorem is proposed. The negative controls and restricted
order statements retain the exact scopes of the claims they address; NG1–NG7
are therefore not invoked as an admission shortcut.

## 10. Research-loop completion and code transfer

| Stage | Actual result/evidence |
|---|---|
| R1 | Fixed PR90 baseline, original raw-U target, runtime derivative-free preference and separate cost/span/accuracy criteria. |
| R2 | Read PR65/74/75/77 material, prior causal theorem and fixed-P implementation; primary SciSpace/Exa papers in REFERENCES.json. |
| R3 | Fixed/represented input identities, model (9), contraction and causal-domain assumptions stated separately. |
| R4 | Selected exact key reuse, two-projector quotient, and screened depth-one map extrapolation; rejected target-changing B=0 shortcut. |
| R5 | Complete proofs A–E, finite causal branch transport, scalar residual lifting, and coding recurrences. |
| R6 | Independent algebraic checks, Wolfram corrected identities, and direct code review; findings in REVIEW.md. |
| R7 | Executed `exact_checks.py`; results in EXACT_RESULTS.json. Native implementation experiments belong to the corresponding lane receipts. |
| R8 | Adopt proofs within their stated domains, retain noncontractive controls and explicit original-space/physical-error obligations. |
| R9 | Equations (2), (11), (12), (18), (19), (21), (22) transferred to native implementations and reviewed against code. |
| R10 | This report, exact checker, CAS/retrieval receipts and review are saved in the theory directory; root integrates native results. |

Code-oriented obligations are carried in the native C1–C10 records. The
remaining general research questions are concrete: obtain tight full-domain
majorants for nonlinear clients; expand quotient closures with controlled
defects; choose homotopy increments from branch and cost evidence; and
identify hardware/RHS regimes where lower work produces wall-clock gain.
They do not prevent the scoped implementations and measurements from being
completed and used now.

## 11. Follow-up: constants derived from the target and accuracy objective

The follow-up request replaces discretionary gain margins with quantities
derived from the target equations. Three different kinds of statement must
remain separate: a sharp operator constant, a proved but potentially
conservative upper bound, and a work-scheduling heuristic. The constructions
below supply the first two explicitly. The generic legacy `SlowOnly`
thresholds remain labeled heuristics; no sampled contraction ratio is
promoted to a full-domain Lipschitz constant.

### 11.1 Sharp cubic midpoint constants and invariant domains

Write `phi(z)=k*z+g*z^3`, where `k>0`, `g>=0`, and let
`a=lambda*h/2`. With `m=y+K/2`, the exact homotopy stage equation becomes

\[
 F_\lambda(K)=K+2a\phi(m)=0,
 \qquad m+a\phi(m)=y.                                      \tag{32}
\]

The second map is strictly increasing from minus infinity to plus infinity.
It has exactly one root. For `y>=0`, its midpoint lies in `[0,y]`, giving
the stage bracket `[-2y,0]`; for `y<0`, use odd symmetry. On `|m|<=Y`,

\[
 \min\phi'(m)=k,\quad
 \max\phi'(m)=L_Y:=k+3gY^2,\quad
 D_\lambda:=1+ak\le F'_\lambda(K),\quad
 q_{\lambda,Y}:=aL_Y.                                     \tag{33}
\]

The derivative extrema and global lower bound `D_lambda` are sharp: the
lower extrema occur at `m=0`, and the upper derivative occurs at `|m|=Y`.
The associated exact Lipschitz constant of `G_lambda(K)=-2a*phi(y+K/2)`
on the corresponding stage interval is `q_lambda,Y`. These constants use
the physical polynomial parameters, with no callback differentiation.

The root bracket by itself does not imply Picard invariance. For `y>0`,
the map sends `[-2y,0]` into itself if and only if

\[
 a(k+gy^2)\le1.                                          \tag{34}
\]

Indeed its image is `[-2a*phi(y),0]`. The same condition with `|y|` holds
for negative `y`; `y=0` is a singleton fixed point. The stronger condition
`a(k+3gY^2)<1`, `|y|<=Y`, establishes both self-mapping and contraction.
The older noncontractive linear control `h*k/2=1.1` does not satisfy (34)
and is deliberately outside this particular convergence proof.

For any trial midpoint `m` and exact root midpoint `m_*`, polynomial
factorization gives the exact divided-error identity

\[
 F_\lambda(K)-F_\lambda(K_*)
 =(K-K_*)\{1+a[k+g(m^2+mm_*+m_*^2)]\}.                   \tag{35}
\]

Since `m^2+m*m_*+m_*^2=(m+m_*/2)^2+3m_*^2/4>=0`,

\[
 |K-K_*|\le\frac{|F_\lambda(K)|}{1+ak}.                  \tag{36}
\]

This is a global residual-to-stage bound, including noncontractive cases.
The same bound is the exact endpoint bound when `y+K` is formed in real
arithmetic. A sharper local divisor can be obtained without differentiation
of callback values: if a certified interval for `m_*` is available, minimize
the quadratic in (35) on that interval. Its minimizing point is the clipped
value `-m/2`. The present implementation uses the simpler sharp global
divisor `1+ak`.

### 11.2 Nonexpansive physical step and a proved truncation constant

For the target station `lambda=1`, define the exact implicit midpoint map
`Psi_h(y)=2m-y`, where `m+(h/2)*phi(m)=y`. Implicit differentiation of
this scalar identity is a proof operation, not a runtime derivative call:

\[
 \Psi'_h(y)=\frac{1-(h/2)\phi'(m)}{1+(h/2)\phi'(m)},
 \qquad |\Psi'_h(y)|\le1.                               \tag{37}
\]

Thus `Psi_h` is globally nonexpansive for every `h>0`, independently of
Picard convergence. This property transports local stage accuracy to a
trajectory bound without an exponentially growing Gronwall factor.

Suppose the true solution starts in `[0,Y]`. It remains in that interval.
Set

\[
 \Phi=\phi(Y),\quad L=k+3gY^2,\quad
 M_2=L\Phi,\quad M_3=6gY\Phi^2+L^2\Phi,
 \quad C=\frac{L^2\Phi}{8}.                              \tag{38}
\]

Here `M_2` bounds `u''=phi'(u)*phi(u)` and `M_3` bounds
`(phi(u))''=phi''(u)*phi(u)^2+phi'(u)^2*phi(u)` along the true orbit.
Each polynomial is nonnegative and increasing on `[0,Y]`. The constant
`C` is a derived safe upper bound; it is not asserted to be the smallest
possible local-truncation constant.

**Theorem F (cubic physical error budget).** The local error of the exact
implicit midpoint method satisfies

\[
 |\Psi_h(u(0))-u(h)|\le\tau(h):=
 \frac{C h^3}{1+hk/2}.                                  \tag{39}
\]

**Proof.** Put `Delta=u(h)-u(0)`, `z=(u(0)+u(h))/2`, and
`v=u(h/2)`. The original stage residual of the true increment is

\[
 F(\Delta)=A-B,\quad
 A=h[\phi(z)-\phi(v)],\quad
 B=\int_0^h\phi(u(t))\,dt-h\phi(v).                     \tag{40}
\]

The true solution is convex and `phi` is increasing, so `A>=0`.
The function `phi(u(t))` is convex, so `B>=0`. The endpoint-average
Taylor remainder and midpoint quadrature remainder give

\[
 A\le\frac{h^3LM_2}{8},\qquad
 B\le\frac{h^3M_3}{24}.                                 \tag{41}
\]

The exact polynomial identity

\[
 2\phi'(Y)^2-\phi''(Y)\phi(Y)
 =2k^2+6kgY^2+12g^2Y^4\ge0                             \tag{42}
\]

implies `M_3<=3*L*M_2`. Consequently `|A-B|<=max(A,B)<=C*h^3`.
Apply (36) at `lambda=1` to convert the residual to the local endpoint
error (39). QED.

Let `h=T/N` and let a computed endpoint `yhat_(j+1)` approximate
`Psi_h(yhat_j)` with a certified error at most `a_j`. By (37),

\[
 |\widehat y_j-u(jh)|\le j\tau(h)+\sum_{\ell=0}^{j-1}a_\ell.
                                                               \tag{43}
\]

Proof is induction: add the algebraic endpoint error, the nonexpansive
transport of the preceding error, and the local truncation error evaluated
from the true starting state. Approximate states need not remain positive
for (37) or (43) to hold.

For a user-specified physical budget `epsilon>0`, choose the smallest
admissible **dyadic** `N` satisfying

\[
 E_{\rm tr}(N)=\frac{C T^3}{N(N+kT/2)}<\epsilon,
 \qquad \frac{T}{2N}\{k+3g(Y+\epsilon)^2\}<1.             \tag{44}
\]

Assign the remaining budget, rather than a free fractional split, by

\[
 a_{\rm step}=\frac{\epsilon-E_{\rm tr}(N)}{N},\qquad
 \rho_{\rm stage}=(1+hk/2)a_{\rm step}.                   \tag{45}
\]

Then `N*tau+N*a_step=epsilon` exactly in real arithmetic. The second
condition in (44) is legitimate by induction: previously accepted points
obey `|yhat_j|<=Y+epsilon`, so their Picard brackets meet (34) and their
exact maps are contractions. Minimality is only among the allowed dyadic
grids satisfying these proved inequalities, not among all possible
discretizations or hardware execution plans. A dyadic subdivision is
admitted only if the represented `h` and `N*h=T` are exact; underflow or
resource limits lead to a stated failure rather than a changed target.

### 11.3 A floating certificate for each accepted cubic step

Interpret represented `k,g,h,yhat,K` as exact real inputs. Evaluate the
original polynomial residual using outward interval operations, obtaining
`F(K) in R`, and take a proved lower enclosure `Dlo<=1+hk/2` with `Dlo>0`.
If `z=fl(yhat+K)` and `e_add` bounds `|z-(yhat+K)|`, then

\[
 |z-\Psi_h(\widehat y)|\le
 \frac{\operatorname{mag}(\mathcal R)}{D_{\rm lo}}+e_{\rm add}.
                                                               \tag{46}
\]

The proof is (36) plus the final-addition displacement. This is the
certificate used for `a_j`; an ordinary computed small residual only
proposes a candidate. Constants in (38)--(45) are evaluated in the safe
directed direction. Actual outward accumulated errors are compared with
the remaining physical budget before accepting each observation.

If an ordinary root passes but (46) is too large, a value-only bisection
uses the analytic bracket from (32). A strictly positive residual interval
moves the upper endpoint; a strictly negative interval moves the lower
endpoint. A straddling interval that cannot establish the required error,
or a bracket with no representable interior point, yields a precision-floor
failure. All original-map and interval-polynomial evaluations are charged,
including rejected accelerated proposals and polishing evaluations.

For an observation labeled by `t_hat=fl(j*h)`, also enclose the exact
`j*h` and add `Phi*|t_hat-j*h|` to (43); `|u'|<=Phi` proves this time-label
allowance. The final admitted endpoint has `N*h=T`. Rounded Picard maps
can lose exact-arithmetic self-mapping near boundaries; this may cause a
safe failure or extra work, but cannot invalidate a successful interval
certificate (46). The generic callback API does not receive the cubic
certificate automatically: the sealed polynomial model supplies its full
analytic structure and outward evaluation.

### 11.4 Residual dominance without a discretionary gain margin

Theorem E needs only `eta=1`. Let the already evaluated baseline be
`p=G_lambda(x)`. Choosing any admissible trial with
`||r_lambda(e)||<=||r_lambda(p)||` preserves the exact contraction bound
in Theorem E. No smaller gain fraction and no coefficient-magnitude cap
is required for that implication. An arbitrary finite coefficient can
still produce a poor or inadmissible trial; the true residual and domain
checks decide whether to keep it. This is a convergence statement per
selected update, not a guarantee that an extra map evaluation pays for itself.

The new `ResidualDominance` mode therefore sets the acceptance factor to
one by construction and bypasses the empirical slow-ratio, coefficient-cap,
and rejection-count fields. For the depth-one coefficient, normalize both
residuals by their maximum absolute component before the dot products.
This changes neither the exact quotient nor its mathematical purpose and
reduces overflow from squaring a common residual scale. If the represented
denominator is zero or the resulting quotient is nonfinite, skip the trial.
No arbitrary epsilon is needed: a poorly resolved nonzero denominator may
produce a finite but inaccurate proposal, which must still pass the actual
residual/domain screen. Nonfinite proposals are skipped before callback
dispatch. Ordinary floating residual comparisons remain operational; the
cubic consumer's final (46), rather than the extrapolation heuristic,
authorizes physical accuracy.

### 11.5 Sharp original-stage residual amplification for the structural model

Fix the autonomous mean-field model of Theorem D, dimension `n`, step `h`,
and the actual stored tableau. Let a stage residual `R_i` be defined by the
original raw-U equation, with the sign convention `M U - rhs`. For a
scalar decay rate `r`, define the lower triangular scalar stage operator

\[
 M(r)_{ii}=1+h\gamma r,\qquad
 M(r)_{ij}=h\gamma r a_{ij}-\gamma c_{ij}\quad(j<i).        \tag{47}
\]

Solve `M(r)^T w(r)=b_code` by reverse scalar recurrence. There is no
state-sized matrix construction. Put `w_t=w(d)`, `w_c=w(d-kappa)` and

\[
 T_i=w_{t,i}Q+w_{c,i}P,\quad
 S=R_tQ+R_cP,\qquad
 R_r=1-r\sum_i b_i t_i(r).                               \tag{48}
\]

The first map transports original-stage residuals to endpoint error;
the second is the exact discrete propagation of an input error. For any
two real scalars define

\[
 \nu_n(t,c)=\left|t+\frac{c-t}{n}\right|
             +(n-1)\left|\frac{c-t}{n}\right|.            \tag{49}
\]

**Theorem G (sharp structural residual budget).** The sharp induced
stacked-infinity-to-endpoint-infinity norm of one stage-error map and the
sharp propagation norm are

\[
 A_n=\sum_i\nu_n(w_{t,i},w_{c,i}),\qquad
 G_n=\nu_n(R_t,R_c).                                     \tag{50}
\]

Starting from an exact common initial state and allowing independent
stage residuals at each step, the sharp accumulated norm after `N` fixed
steps is

\[
 C_N=\sum_{j=0}^{N-1}\sum_i
       \nu_n(R_t^j w_{t,i},R_c^j w_{c,i}).                \tag{51}
\]

Thus `||R_(ell,i)||_infinity<=delta` for every step/stage implies
`||yhat_m-y_m||_infinity<=C_N*delta` for every grid index `m<=N`.
An algebraic trajectory budget `epsilon_alg` determines

\[
 \delta=\epsilon_{\rm alg}/C_N.                          \tag{52}
\]

If `C_N=0`, the endpoint is insensitive to all such stage residuals and
requires a separate zero-gain handling; the current tableau has positive
gain and the native API requires a finite positive result.

**Proof.** If `E` is the stage error, the linear raw-U equations give
`M(r)E=R` in each projector channel. Consequently
`b_code^T E=w(r)^T R`, proving the first equation in (48). Changing the
input state by `e` changes the initial RHS by `J e`; Theorem D's scalar
coefficients give endpoint change `S e` with the stated `R_r`. A matrix
`tQ+cP` has diagonal `t+(c-t)/n` and off-diagonal `(c-t)/n`, so every row
has absolute sum (49). Concatenating independent stage inputs adds their
absolute row sums, proving (50).

For a residual sequence, the exact difference recurrence is

\[
 e_{\ell+1}=S e_\ell+\sum_i T_iR_{\ell,i},\qquad
 e_N=\sum_{\ell=0}^{N-1}\sum_i
            S^{N-1-\ell}T_iR_{\ell,i}.                  \tag{53}
\]

Because `P^2=P`, `Q^2=Q`, and `PQ=QP=0`,
`S^j T_i=R_t^j*w_(t,i)*Q+R_c^j*w_(c,i)*P`. Its concatenated absolute
row sum is (51). Equality is attainable by choosing the sign of each
independent residual component to align with one fixed output row. The
triangular stage system is invertible, so such residuals correspond to
actual perturbed stages. This proves sharpness. Finally `C_m<=C_N`
because the added terms in (51) are nonnegative, proving the common bound
at every prefix. QED.

The simpler geometric upper bound remains useful for comparison:

\[
 C_N\le A_n\sum_{j=0}^{N-1}G_n^j.                        \tag{54}
\]

It follows from submultiplicativity, without a contraction assumption.
Equality can occur; the audited native fixture exhibits equality, so
the tighter formula is not claimed to improve that particular budget.
Compute the powers and finite sum by recurrence, including `G_n=1`,
without a singular divided expression. The new setup costs `O(N*s)`
scalar arithmetic and two scalar reverse tableaux. Its ordinary binary64
evaluation is an operational derived tolerance, not an outward enclosure.
For a full floating or physical certificate one must additionally bound
coefficient evaluation, actual residual evaluation, endpoint rounding,
initial-state error, and the separate temporal truncation error. None of
those errors is silently included in `epsilon_alg`.

### 11.6 Exact selective-entry comparison in the existing work objective

For each already recorded interval define the existing deterministic work
objective, now before conversion to floating point,

\[
 W=N_{\rm RHS}+N_{\rm JVP}+N_{f_t}+nN_J+N_{\rm solve}.   \tag{55}
\]

The opt-in strict entry comparison admits a successful same-checkpoint,
same-endpoint pilot exactly when its checked integer count is smaller than
the accepted explicit prefix count. Ties retain the explicit route. No
empirical gain factor is required for that specified objective.

If the incumbent count is `E>0` and a candidate's charged callback-prefix
count is `L`, then `L>=E` rules out a strict win: its final total is at
least `L`. The native guard stops *after* charging a callback whose count
exceeds its ceiling, so the exact integer ceiling is `E-1`. For `E=0`
there is no strict winner; a zero ceiling safely terminates on the first
charged callback. If checked arithmetic overflows or the ceiling cannot
be represented below the native unarmed sentinel, the code declines that
evidence/early ceiling rather than truncating a count. Existing attempt
limits still apply, and every completed failed-pilot callback is retained.

This policy applies to enabled entry pilots. Zero pilot budget retains
the previously documented pressure-only direct-entry mode. Pressure,
retry/backoff, return policy, and resource limits are separate mechanisms;
their existing heuristic parameters are not made theorem constants by
the exact entry comparison. The theorem establishes a local choice in
objective (55). Whole-trajectory work savings and wall-clock gains remain
actual measured outcomes.

### 11.7 Follow-up evidence and transfer

The focused follow-up adds model identities and budget equalities in
`exact_constant_checks.py`, with executed results in
`EXACT_CONSTANT_RESULTS.json`. These rational checks audit algebra and
implementation-ready formulas; the general differential and quadrature
arguments are the proofs above. The parent independently audits (47)--(54)
against exact rational stored coefficients in
`evidence/exact_structural_constants.py`. The native implementation lanes
own their measured trajectories, work counts, and runtime receipts.

The coding transfers are concrete: `CubicDecayAccuracyPlan` derives the
grid and residual budget; `cubic_verified_step` implements (46);
`ResidualDominance` implements the margin-free screen; the structural plan
computes (51)--(52); and the adapter uses checked integers for (55).
This extension revisits R1/R3/R5 with the requested accuracy objective and
domain constants, executes the narrow R6/R7 algebra/code checks, and
completes R8--R10 through the saved focused review and theorem handoff.
