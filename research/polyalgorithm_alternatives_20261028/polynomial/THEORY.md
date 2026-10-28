# PR-N01 polynomial / Krylov alternatives: reuse and concrete constructions

**Scope and source.** R2 literature/reuse and R4 constructive alternatives of PhysMath 1.3.0, with supporting algebra; baseline `7eb2dd0ffda39afb2871d76700a720986ff09fe0`. This is a theory/design artifact. No solver source, router default, compiled experiment, benchmark result, or performance promotion is changed by this lane. Statements below distinguish established results, new derivations for this project, symbolic checks, and unproved implementation obligations.

## 1. What is already implemented, and must be reused

| Evidence | Current result | Consequence for this work |
|---|---|---|
| PR66 | Normalized augmented phi action; polynomial backend work then remained open | Preserve its scaled-input convention and earlier scalar counterexamples. The augmentation itself does not prove a general nonnormal error bound. |
| PR68, POLY-01/02/03 | Joint Chebyshev/Laguerre phi actions; warm Chebyshev historical 3.17x on the measured frozen-operator population; cold Chebyshev inconclusive; cold Laguerre blocked; statistical authority held | These are historical receipts, not new or generally admissible speed claims. Cold setup and warm reuse must be separate estimands. |
| PR69 | Laguerre positive majorant, continuous scale experiment, scaled Taylor prototype | Positive majorant was very loose (reported up to 3.5e65x); continuous scale lowered degree at 0/22 points and was rejected. Do not rerun the same scale proposal as a fresh success. |
| `rodas5p-core/src/laguerre_adjoint.rs` | Signed output-adjoint identity, Bernstein interval envelopes and counted setup; degree limit 128, subdivision depth 3 are explicitly policies | The signed-adjoint idea is already implemented. A degree-m envelope costs O(m² 2^depth) interval work; there are m+1 adjoint envelopes. Charge the implementation counters for the complete setup rather than treating the per-envelope bound as the whole cost. |
| `rodas5p-core/src/polynomial_action.rs` | Positive coefficient enclosures; verified or declared spectral status; complete Chebyshev total and opt-in Laguerre complete-total admission; exact cache keys; normalization and range terms | Reuse coefficients, target semantics, admission ownership and historical refusals. The introductory comment describing Laguerre as lacking propagation is historical; current signed-adjoint admission is authoritative within its scope. |
| `rodas5p-core/src/polynomial_router.rs` | FirstAdmitted / CompareAll across enclosed Leja, Chebyshev and Laguerre; failures charged; immutable dense symmetric operator; private fused result | The polyalgorithm already exists. New work adds admissible candidates or cheaper execution, not a replacement router. Krylov is not a certified fallback for this fused-action token. |
| `rodas5p-integrators/src/polynomial_resolvent.rs` and `conditioned_polynomial.rs` | Full-space polynomial-relation resolvents; original-space residual before commit; identity/epoch/scale/norm binding; conditioned chart and outward stored-coefficient defect | Prefer declared low-degree structure when available. A few probes cannot manufacture a full-space relation or symmetry witness. |
| `vigilode-coupled/src/algebra.rs`, `causal.rs` | Two-pass MGS and q/Jq storage; one common projected factorization across eight stages; exact-real finite causal Picard statement; original-space residual checking | Common-factor reuse is already done. Direct polynomial work must beat this amortized baseline, including repeated stage RHS and residual checks. |

Repository URLs are anchored by the baseline and paths above:
https://github.com/cosmosapjw-quantum/vigilode/tree/7eb2dd0ffda39afb2871d76700a720986ff09fe0
PR URLs: https://github.com/cosmosapjw-quantum/vigilode/pull/66 ,
https://github.com/cosmosapjw-quantum/vigilode/pull/68 ,
https://github.com/cosmosapjw-quantum/vigilode/pull/69 ,
https://github.com/cosmosapjw-quantum/vigilode/pull/92 .

## 2. Priority construction: one genuinely fused backward recurrence

**Derived, not implemented here.** The current forward joint action uses one recurrence for `SameVector`, but five vector recurrences for five `Distinct` inputs. Its inner loop records `poly_vector_products += width` for every degree. If a caller requests only the fused result, the algebra permits one backward recurrence even for distinct inputs.

Let
\[
 F_m=\sum_{j=0}^{4}\sum_{k=0}^{m}c_{kj}P_k(X)w_j,\qquad
 d_k=\sum_{j=0}^{4}c_{kj}w_j.
\]
Linearity gives
\[
 F_m=\sum_{k=0}^{m}P_k(X)d_k.
\]
The vectors \(d_k\) can be formed on demand. No derivative, matrix factorization, or basis orthogonalization is introduced by this identity.

### Theorem P1 — fused Laguerre Clenshaw and exact perturbation transfer

For any fixed finite-dimensional linear operator \(X\), set \(b_{m+1}=b_{m+2}=0\), and evaluate backwards
\[
 b_k=d_k+\frac{(2k+1)I-X}{k+1}b_{k+1}
          -\frac{k+1}{k+2}b_{k+2},\quad k=m,\ldots,0.
\]
Then \(b_0=\sum_{k=0}^{m}L_k(X)d_k\), for ordinary Laguerre polynomials.

Suppose the computed recurrence is this exact recurrence plus additive local vector defects \(\delta_k\), including the error in forming \(d_k\), the supplied operator action, rounded scalar coefficients, and recurrence arithmetic. With zero tail states represented exactly,
\[
 \widehat b_0-F_m=\sum_{k=0}^{m}L_k(X)\delta_k.                 \tag{P1}
\]
If \(X\) is self-adjoint in a specified Hilbert norm and \(\sigma(X)\subset[0,L]\), then
\[
 \|\widehat b_0-F_m\|
 \le e^{L/2}\sum_{k=0}^{m}\|\delta_k\|.                       \tag{P2}
\]

**Proof.** The coefficient of an impulse inserted into backward position \(k\) obeys the three-term Laguerre recurrence with initial polynomials \(1\) and \(1-x\); hence it is \(L_k(x)\). Superposition proves both the exact action and (P1), including vector-valued impulses. The spectral theorem and \(\sup_{x\in[0,L]}|L_k(x)|\le e^{L/2}\) give (P2). The scalar inequality is DLMF 18.14.8 at \(\alpha=0\). This proof does not infer symmetry from eigenvalues.

The existing forward algorithm transfers its local defects through coefficient-dependent adjoint polynomials \(z_j(X)\). This proposed execution direction instead transfers backward defects through \(L_j(X)\). Both identities are valid; their local defects differ. One cannot substitute the new constant into old forward residuals. Backward intermediate vectors can still grow, and their actual local defects must be enclosed.

**Expected work change, an exact operation-count statement under the stated target.** For degree \(m\), five distinct inputs and a fused-only output, matrix-vector actions change from \(5m\) to \(m\); forming \(d_k\) costs O(5mn) scalar operations. Stored state is O(n) recurrence workspace plus the existing coefficient table and five inputs. There is no factor-five wall-time claim. `SameVector` already uses width one, and callers needing all five separate columns cannot use the fused-only shortcut.

The simple transfer factor also offers an O(m) accumulation alternative to degree-dependent Bernstein envelope preparation, potentially removing the *preparation policy* that currently restricts signed-adjoint Laguerre to degree 128. This does not authorize removing that limit from the existing backend. A new total-error proof and an admitted result type must come first.

### Theorem P2 — fused Chebyshev Clenshaw

For \(T_k\), set \(b_{m+1}=b_{m+2}=0\), compute
\[
 b_k=d_k+2Xb_{k+1}-b_{k+2},\quad k=m,\ldots,1,
 \qquad F_m=d_0+Xb_1-b_2.
\]
If \(\delta_0\) includes output assembly error and \(\delta_k\) are local backward defects, the exact error is
\[
 \widehat F_m-F_m=\sum_{k=0}^{m}T_k(X)\delta_k.
\]
For self-adjoint \(X\) with spectrum in \([-1,1]\), it is bounded by
\(\sum_k\|\delta_k\|\). If the represented chart is instead enclosed by
\([-1-\epsilon,1+\epsilon]\), use
\(\|T_k(X)\|\le\cosh(k\operatorname{arcosh}(1+\epsilon))\), with an outward implementation of that expression or the existing exponential majorant.

**Proof.** Substitute the backward recurrence into \(d_0+Xb_1-b_2\); consecutive terms telescope using the Chebyshev recurrence. Apply the same identity to the defect sequence and then the spectral theorem. This is a bound for the *represented chart*. The exact-arithmetic interval \([-1,1]\) must not be reused after rounding without justification.

### Complete budget and provider obligations

For either basis, admission must establish
\[
 E_{\rm trunc}+E_{\rm coeff}+E_{\rm source}
  +E_{\rm recurrence}+E_{\rm normalization}+E_{\rm output}
 \le \varepsilon_{\rm requested}.
\]
One valid coefficient term is
\[
 E_{\rm coeff}\le
 \sum_{k,j}\operatorname{rad}(c_{kj})\,
             \sup_{\sigma(X)}|P_k|\,\|w_j\|.
\]
Input scaling factors belong to the represented input semantics; their products and underflow losses must be included. No cancellation in the fused result permits discarding a large input-column error.

The initial implementation target should reuse the **current dense symmetric provider** and its local directed arithmetic, then compare with the current forward-adjoint path. Porting to a matrix-free callback is a separate proof obligation: the callback must provide an action-error enclosure and immutable operator/norm/domain identity. A callback name or successful sample does not supply these. If enclosing local defects needs a dense matrix or an additional operator action, report that memory and work explicitly.

## 3. Chebyshev residual polynomials for Rosenbrock resolvents

This is a different target from the existing phi actions. Let \(A=I-h\gamma J\), or any fixed self-adjoint positive operator in a declared metric, with verified spectrum in \([a,b]\), \(0<a<b\). Put
\[
 c=(a+b)/2,\quad d=(b-a)/2,\quad \xi=c/d,\quad
 p_m(z)=\frac{T_m((c-z)/d)}{T_m(\xi)}.
\]
Because \(p_m(0)=1\), \(q_{m-1}(z)=(1-p_m(z))/z\) is a polynomial. For
\(r_0=f-Ax_0\), set \(x_m=x_0+q_{m-1}(A)r_0\).

### Theorem P3 — exact worst-case residual constant and degree

In exact arithmetic,
\[
 r_m=p_m(A)r_0,\qquad
 \|r_m\|\le\delta_m\|r_0\|,\qquad
 \delta_m=\frac1{T_m(\xi)}
 =\frac{2q^m}{1+q^{2m}},\quad
 q=\frac{\sqrt{b/a}-1}{\sqrt{b/a}+1}.
\]
For \(0<\varepsilon<1\), the smallest degree satisfying this uniform bound is
\[
 m_{\min}=\left\lceil
 \frac{\operatorname{arcosh}(1/\varepsilon)}
      {\operatorname{arcosh}(\xi)}
 \right\rceil.                                               \tag{P3}
\]

**Proof.** \(Aq_{m-1}(A)=I-p_m(A)\) proves the residual identity. The spectral theorem reduces its norm to the supremum of \(|p_m|\) on \([a,b]\), where the numerator has maximum one. Since \(T_m(\xi)=\cosh(m\operatorname{arcosh}\xi)\) and
\(\exp[-\operatorname{arcosh}\xi]=q\), the constant and degree follow. Alternation at the \(m+1\) extrema gives the minimax residual polynomial among degree-\(m\) polynomials equal to one at zero: a strictly better polynomial would force its difference from \(p_m\) to have \(m\) roots between the extrema plus the root zero outside the interval. This contradicts its degree.

The degenerate case \(a=b\) means \(A=aI\) under self-adjointness and is a direct scalar solve. In production, outward arithmetic must verify the proposed integer degree; rounded logarithms alone are not an admissible degree decision. For the illustrative interval \([1,9]\), \(q=1/2\), and \(m=28\) is the exact least degree for \(\delta_m\le10^{-8}\). This is not a runtime measurement.

For \(J\) self-adjoint with spectrum \([-\beta,-\alpha]\), \(\alpha\ge0\), \(h\gamma\ge0\), valid endpoints are \(a=1+h\gamma\alpha\) and \(b=1+h\gamma\beta\). An operator norm/spectral witness is required; observed Ritz values need not enclose the spectrum.

**Norm boundary.** Existing conditioned relation kernels certify the infinity norm. A metric-\(M\) proof cannot silently reuse that label. For diagonal \(M=\operatorname{diag}(w_i)>0\),
\[
 \|x\|_\infty\le(\min_i w_i)^{-1/2}\|x\|_M,\qquad
 \|x\|_M\le(\sum_iw_i)^{1/2}\|x\|_\infty.
\]
Use the actual norm or charge this conversion. True physical residual, evaluation allowance and a valid inverse bound are still required before output commit.

**Cost limitation.** A small coupled basis is already reused across eight stages. A degree-28 direct action on each of eight causal right-hand sides can cost much more than that shared basis. Compare with existing small GMRES, the shared coupled solve, CG when its SPD assumptions apply, and the existing specialized direct/low-degree relation solver. Polynomial approximation removes reductions and basis storage; it does not remove operator actions or causal RHS work.

## 4. Hybrid near-structure correction: retain the best old algebra

### Theorem P4 — right polynomial correction without commutation

Let a reusable approximate inverse \(P\) satisfy
\(\|D\|\le\theta<1\), where \(D=I-AP\). Define
\[
 Q_r=P\sum_{j=0}^{r-1}D^j.
\]
Then \(AQ_r=I-D^r\), so
\[
 \|f-AQ_rf\|\le\theta^r\|f\|.
\]
No commutation of \(A\) and \(P\) is required.

**Proof.** \(AP=I-D\); the finite geometric sum telescopes. If
\(A=A_0+E\), then \(D=I-A_0P-EP\), giving the full-space bound
\(\theta\le\|I-A_0P\|+\|E\|\|P\|\). Thus existing exact exchange, minimal-polynomial, conditioned-chart or block formulas can become low-cost preconditioners when a model supplies an explicit perturbation bound.

Choose the smallest \(r\) passing \(\theta^r\le\varepsilon\), by outward integer verification. If the bound fails, do not tune a contraction factor from a few residual samples and label it proven. A fixed polynomial preconditioner has a fixed linear action; changing its degree or coefficients during a standard outer Krylov solve changes that assumption. Freeze the plan for the solve or use an explicitly flexible outer algorithm. Wathen–Rees provides the primary linearity motivation.

A polynomial warm start followed by the existing true-residual-controlled Krylov solver is also legitimate, but must charge failed polynomial setup, candidate actions, and the residual used to start the fallback. For phi actions, the current unadmitted Krylov estimate cannot be advertised as a certified-token fallback.

## 5. Nonnormal alternatives and a precise counterexample

Let \(N=\begin{pmatrix}0&1\\0&0\end{pmatrix}\), \(A=I+KN\). Then
\[
 \sigma(A)=\{1\},\quad p(A)=p(1)I+Kp'(1)N.
\]
For \(p(z)=1-z\), \(p(1)=0\) but \(\|p(A)\|_2=|K|\). Eigenvalue placement alone therefore does not justify the SPD residual bound. This is a counterexample to that specific inference, not a general impossibility statement. Indeed \(N^2=0\) gives \(A^{-1}=I-KN\), exactly the sort of declared low-degree structure the existing relation kernel can exploit.

A broader mathematical branch can use the verified numerical range
\[
 W_M(A)=W(M^{1/2}AM^{-1/2}).
\]
Crouzeix–Palencia gives, for functions holomorphic near the set,
\[
 \|g(A)\|_M\le(1+\sqrt2)\sup_{z\in W_M(A)}|g(z)|.
\]
This exact constant is established; no use is made of a conjectured constant 2.

### Derived elliptic Chebyshev/Faber branch

Let \(C=(cI-A)/d\), \(c/d=\xi>1\), and suppose a full-space witness encloses \(W_M(C)\) inside the filled Bernstein ellipse
\[
 \mathcal E_R=\{(w+w^{-1})/2: |w|=R\}\text{ and its interior},
 \quad 1\le R<\zeta=\xi+\sqrt{\xi^2-1}.
\]
Then the same residual polynomial satisfies
\[
 \|p_m(A)\|_M
 \le(1+\sqrt2)\frac{R^m+R^{-m}}{\zeta^m+\zeta^{-m}}.            \tag{P4}
\]
This follows from \(T_m((w+w^{-1})/2)=(w^m+w^{-m})/2\), the maximum principle and the numerical-range theorem. Chebyshev polynomials are the ellipse-adapted Faber family; a new general Faber engine is unnecessary for this first branch.

The witness can come from analytic bounds on Hermitian and skew-Hermitian parts, with a rigorously enclosing ellipse. Sampled eigenvalues and a posteriori good behavior are insufficient. The current dense symmetric provider cannot supply this authority automatically. A general Leja/Faber construction remains exploratory until both scalar-domain error and full-space numerical-range/contour bounds are available. The Leja primary paper below provides a parameter-selection template; it does not certify this proposed implementation.

## 6. Laguerre semigroup, exact constants and the old scaling negative result

For self-adjoint \(H\ge0\), \(\sigma(H)\subset[0,\beta]\), let \(X=\sigma H\), \(L=\sigma\beta\), \(\tau\ge0\), and
\(t=\tau/(\sigma+\tau)\). The ordinary generating function gives
\[
 e^{-\tau H}=(1-t)\sum_{k=0}^\infty t^kL_k(X).
\]
Thus its degree-\(m\) truncation satisfies
\[
 \|e^{-\tau H}-(1-t)\sum_{k=0}^m t^kL_k(X)\|
 \le e^{L/2}t^{m+1}.                                       \tag{P5}
\]
The proof sums the geometric coefficient tail and uses the same Laguerre norm bound. This bound, its scale optimization, and joint phi coefficient machinery are already present in the project; they are reused here, not claimed as new.

The least-squares Laguerre/FCR paper motivates short recurrences, but its consecutive-iterate stop is not a complete operator-norm certificate. Laguerre's half-line orthogonality does not make the finite polynomial uniformly accurate on every unbounded positive spectrum.

Raw truncation has value \(1-t^{m+1}\) at zero. Adding \(t^{m+1}I\) preserves a zero-mode exactly and increases the simple bound to \((e^{L/2}+1)t^{m+1}\). For augmented phi chains, preserving a scalar zero-mode alone is insufficient; the nilpotent chain also constrains derivatives at zero. Prefer the existing direct joint-phi coefficient semantics over transplanting an unqualified exponential polynomial into the augmentation. Any alternative must prove its normalized-chain identities and total transform error.

For fixed \(m\), differentiating the logarithm of (P5) gives the unconstrained optimal scale
\(\sigma_*=2(m+1)/\beta-\tau\), if positive. This belongs to the same continuous-scale family already explored negatively in PR69. Do not market this derivative as a new optimization result. The new experiment changes evaluation direction and fusion, while retaining the old scale branch as a comparator.

## 7. Derivative-free scope and polyalgorithm contract

An action \(v\mapsto Av\) can be matrix-free while still using a Jacobian. Classify the provider explicitly:

1. **Model-declared linear generator or exact block algebra:** can be fully derivative-free, with no Jacobian assembly, AD or Newton step.
2. **Frozen supplied JVP/AD:** a selective derivative-assisted provider, allowed only under the project's capability policy and a demonstrated need.
3. **Nonlinear finite differences:** not an exact fixed linear operator in general; any theorem above requires a bounded action defect. Repeated nonlinear RHS evaluations cannot be relabeled an exact polynomial action.

The proposed selection is a *capability and total-cost* choice: declared exact low-degree relation; admissible fused polynomial action; SPD residual polynomial or CG; near-structure polynomial preconditioning; current Krylov fallback where its target-specific admission permits; numerical-range Leja/ellipse branch when its stronger witness exists. There is no universal ranking.

For every request distinguish:

- Adapter/switcher work: domain/witness validation, norm conversions, coefficient and envelope cache keys, planning, input transforms, candidate attempts and refusals.
- Mode work: actual operator/RHS actions, scalar-vector recurrences, local-error enclosure, reductions, residual evaluation, factorization/orthogonalization, output commit.
- Cold and warm costs: if measured fixed setup and per-use costs are \(S_P,S_K,C_P,C_K\), the algebraic break-even inequality is \(S_P+NC_P<S_K+NC_K\). The timing estimates remain hardware evidence, not theorem constants.
- Sequential span: fusion removes redundant products for one action; it does not make a dependent ODE trajectory or the eight causal stage right-hand sides independent.

Do not sum counters with incompatible units into a fictitious FLOP total. The user-requested removal of arbitrary mathematical hyperparameters means derive admissibility and degree from proofs and the requested budget. Memory/degree caps remain explicit resource policies, and hardware cost models remain calibrated measurements. These categories must not be mislabeled as mathematical constants.

## 8. Prospective implementation and experiment contract

**First priority, P-FUSE:** implement a separate fused-only backward Chebyshev/Laguerre candidate on the current verified dense symmetric domain. Reuse coefficient tables, input semantics, token ownership, complete-budget checks and existing refusals. Preserve the current forward path as the per-column and comparison option. Do not change default ordering before qualification.

**Second priority, P-RES:** add a typed SPD/weighted-SPD residual-polynomial plan for \(I-h\gamma J\), with actual norm and exact identity binding. Keep specialized relation/direct providers first. Compare standalone Chebyshev, CG, small GMRES and shared coupled basis on identical represented right-hand sides.

**Third priority, P-HYB:** declare near-structure perturbation bounds and compare exact-relation preconditioned residual polynomials, polynomial warm start + Krylov, and unchanged baseline. Charge all attempts.

**Exploratory branch, P-NONNORMAL:** build one analytically verified numerical-range ellipse case and the Jordan counterexample. Separate proof of the witness from polynomial scalar approximation; no automatic general-nonnormal promotion.

Each task gets at most three correction iterations after a registered baseline. The smallest discriminating matrix includes: distinct versus same-vector phi inputs; fused-only versus per-column requests; cold versus warm caches; degree around and above 128; zero/nilpotent modes; strong input cancellation; physical norm mismatch; changed epochs; callback failure; refused action followed by fallback; SPD increasing condition number; a weighted-self-adjoint case; the Jordan family.

Accuracy uses independent exact/sufficient-precision references plus the full claimed total bound. Measure operator and RHS actions, coefficient/envelope/local-defect work, reductions, allocations/peak resident workspace, true residuals, and all failures. Profile complete call boundaries as well as mode kernels. Reuse the historical registry and retain negative results; this artifact authorizes no new speed claim.

## 9. Actual execution, verification and sources

Completed in this lane: source/PR inspection; Exa and SciSpace discovery; direct reading of primary sources; derivations above; one exploratory Wolfram algebra call. The Wolfram context service first returned an internal error; the subsequent stateless evaluator succeeded. Its exact result was zero for Chebyshev identities at degrees 0..8, zero for the degree-5 perturbed Laguerre Clenshaw identity, and true for Laguerre derivative identities through degree 8. These were exploratory supporting checks before this node's registration, not prospective experiment evidence.

`CHECK.py` is an unexecuted prospective exact-arithmetic checker at authoring. It checks finite identities and one exact degree example; it does not prove the general theorem, verify compiled rounding, or measure solver performance. Parent registration must precede its execution.

Primary sources actually read:

- Golub–Varga, *Chebyshev semi-iterative methods… Part I* (1961), introductory SPD assumptions and polynomial iteration framework: https://www.math.kent.edu/~varga/pub/paper_17.pdf .
- Wathen–Rees, *Chebyshev semi-iteration in Preconditioning* (2008 report; later ETNA), Sections 2–3, fixed linear polynomial preconditioners: https://www.cs.ox.ac.uk/files/1540/NA-08-14.pdf .
- Sheehan–Saad–Sidje, *Computing exp(-tau A)b with Laguerre polynomials*, ETNA 37 (2010), Sections 2–3: https://etna.ricam.oeaw.ac.at/vol.37.2010/pp147-165.dir/pp147-165.pdf .
- NIST DLMF generating function 18.12.13 and inequality 18.14.8: https://dlmf.nist.gov/18.12.E13 and https://dlmf.nist.gov/18.14.E8 .
- Crouzeix–Palencia, *The numerical range as a spectral set* (2017), equation (3), Theorem 6: https://arxiv.org/pdf/1702.00668 .
- Al-Mohy–Higham, *Computing the Action of the Matrix Exponential, with an Application to Exponential Integrators* (2010 preprint/2011 paper), polynomial scaling and normalized augmentation context: https://eprints.maths.manchester.ac.uk/1536/3/paper16.pdf .
- Caliari–Kandolf–Ostermann–Rainer, *The Leja method revisited*, primary author preprint excerpts for parameter selection and Hermitian/skew-Hermitian enclosures: https://iris.univr.it/retrieve/e14ff6e5-9f54-0209-e053-6605fe0ad24c/CKOR16_pre.pdf . Exa retrieved primary excerpts; the separate web-open endpoint failed, so this is not recorded as complete-paper verification.

The four user-supplied homotopy/DAE/geometric-integration sources belong to the sibling theoretical lanes; this lane does not cite them as support for polynomial error bounds.

**Skill context used:** `physmath-research-loop` version 1.3.0, `c38/physmath-research-loop/references/THEORY_LOOP.md`, R2 and R4 excerpts plus status/evidence and strong-no-go constraints. Original derivative-free-first causal Rosenbrock/RK/homotopy design preserved; definitions connect to equations, candidate code and prospective validation. No claim that all ten theory/coding stages were executed; no recursive delegation; no runtime edit or publication by this worker.
