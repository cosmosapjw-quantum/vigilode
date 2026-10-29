# F6: a trace-only aggregate ODE across a triple-root collision

Status: implemented and locally validated; independent review is pending. All scientific variables and time are dimensionless. This client intentionally has a different observable contract from the baseline selected-real-branch client.

## Observable contract and the quotient algebra

Start with the baseline constraints

\[
q_1^2+q_2=a,\qquad q_2^2+q_1=b.
\]

Putting \(z=q_1\), \(q_2=a-z^2\), gives

\[
P(z)=z^4-2az^2+z+a^2-b.
\]

For every complex parameter pair the algebra \(A=\mathbb C[z]/(P)\) has basis \(1,z,z^2,z^3\) and dimension four, because P is monic. It does not lose dimension on the discriminant. Multiplication by z in that basis is

\[
M=\begin{pmatrix}0&0&0&b-a^2\\1&0&0&-1\\0&1&0&2a\\0&0&1&0\end{pmatrix}.
\]

The observable is \(\operatorname{Tr}_A h(z,a-z^2)\) for a polynomial h. Over a splitting field this equals the sum of h at **all complex roots, counted with algebraic multiplicity**. A repeated root contributes its local algebra length. A Jordan block's strictly triangular terms have zero trace; diagonal entries repeat h evaluated at the root. This is the trace of the full quotient algebra, not the radical/distinct-root trace.

`PolynomialObservable` implements finite rational polynomials in \(q_1,q_2\) and their all-root trace; scalar arithmetic and polynomial combinations of these traces define the client vector field. The API accepts nonnegative integral exponents and exact rational coefficients. It rejects a branch indicator, a callable predicate, negative/fractional powers, and an arbitrary label or root-dependent weight not represented by the stated polynomial. This is an explicit supported class, not a claim that every nonpolynomial symmetric observable is impossible. For instance \(\sum(q_1^2+q_2)=4a\) is admissible. The value on the largest real root, an indicator \(q_1>0\), and a chosen physical amplitude are outside this contract.

No individual root, selected real branch, or branch-dependent amplitude can be recovered by treating a trace as a physical branch. Complex roots remain in the sum even where only some roots are real. The extension here therefore complements the baseline physical-branch ODE rather than replacing it.

## Exact identities and multiplicities

Newton identities give

\[
S_0=4,\ S_1=0,\ S_2=4a,\ S_3=-3,\ S_4=4a^2+4b,
\quad S_5=-10a,\quad S_6=4a^3+12ab+3.
\]

For \(k>4\), \(S_k=2aS_{k-2}-S_{k-3}-(a^2-b)S_{k-4}\). The implementation uses the supplied closed formulas through degree six and this recurrence above six. The independent matrix implementation repeatedly multiplies M using exact rationals. It agrees through degree fourteen at eighteen rational parameter pairs. These include

\[
P|_{a=b=3/4}=(z-1/2)^3(z+3/2),\qquad S_k=3(1/2)^k+(-3/2)^k,
\]

and

\[
P|_{a=b=-1/4}=(z+1/2)^2(z^2-z+5/4),
\]

which has a double root and a complex conjugate pair. The latter's sums are also independently evaluated in exact Gaussian rationals. In particular \(S_0=4\) at the triple root; replacing it by the number of distinct roots (two) would change the observable.

`symbolic_checks.py` checks seventeen exact polynomial identities, including the companion characteristic polynomial, traces, factorizations, derivatives, discriminant, and controlled path. These are exact symbolic algebra computations and analytic arguments, not a proof assistant formalization. `test_trace_client.py` has eight test methods with independent rational and floating-point comparisons. Root-coordinate accuracy at a multiple root is not certified by these comparisons.

## A genuinely coupled trace-driven ODE

Let \(x=(a,b)\), and define

\[
G_1=-a+S_2/32+S_4/128+S_6/512,\qquad
G_2=-b+S_2/64+S_4/64+S_6/256.
\]

Both components depend on a and b through aggregate observables; the coupling is nonlinear. On \([-1,1]\), prescribe the manufactured exact path

\[
p(t)=3/4+t/8+t^5/64,\quad p'(t)=1/8+5t^4/64,
\qquad \dot x=G(x)+p'(t)(1,1)-G(p(t),p(t)).
\]

Initialize \(x(-1)=(39/64,39/64)\). Direct substitution proves \(x(t)=(p(t),p(t))\), with terminal value \((57/64,57/64)\). This is a controlled test client with an exact solution; it is not the original baseline slow physical ODE, and no general application claim follows from this example. The fifth-degree path makes RK4 nonexact, supporting a meaningful convergence test.

Along the diagonal,

\[
\operatorname{disc}P=(4a-3)^3(4a+1).
\]

Since \(p'(t)>0\), the exact path crosses the discriminant with a change of sign at t=0, where a=b=3/4 and P has the triple root above. The full quotient trace is polynomial and needs no labeling, determinant inversion, deflation, or root-continuation step there. The numerical trajectory approximates this exact crossing; the separate multiplicity tests also evaluate the singular point exactly.

## Whole-slab residual certificate

The exact reduced G is polynomial. On the convex box \(D=[1/2,1]^2\), its Jacobian is Metzler (off-diagonal entries are nonnegative), with negative diagonal entries:

\[
DG=\begin{pmatrix}
-7/8+a/16+(12a^2+12b)/512 & 1/32+12a/512\\
1/16+a/8+(12a^2+12b)/256 & -15/16+12a/256
\end{pmatrix}.
\]

The row sums increase with a,b on D, and their upper bounds at (1,1) are \(-91/128,-39/64\). Hence the infinity-norm logarithmic norm is at most \(-39/64<-1/2\). The time-dependent control has no state derivative, so the same bound applies to the controlled field. For the error between the exact solution and a differentiable candidate, the standard norm differential inequality is

\[
D^+\|e\|_\infty\le -\tfrac12\|e\|_\infty+\|\rho\|_\infty.
\]

On each closed slab \([t_i,t_{i+1}]\), build the cubic Hermite interpolant from the RK4 nodes and **exact rational evaluations** of endpoint slopes. Binary64 node values are converted with `Fraction(float)`, preserving the exact represented value. In local coordinate \(s\in[0,1]\), compose the polynomial field and control with this cubic and the affine time polynomial. The residual \(\rho=\dot{\widehat x}-f(t,\widehat x)\) is an exact rational polynomial. Its Bernstein coefficients give a valid range over the entire slab by the convex-hull property; no sampled residual is substituted for a bound.

With \(h=t_{i+1}-t_i\), a bound \(\delta_i\ge\sup\|\rho\|\), and an incoming error bound \(E_i\), the implementation uses

\[
E_i^{\rm whole}=E_i+h\delta_i,\qquad
E_{i+1}=\operatorname{roundUp}_{112}\left(\frac{E_i}{1+h/2}+h\delta_i\right).
\]

These follow from variation of constants and \(e^{-h/2}\le1/(1+h/2)\). The rational upward rounding controls fraction growth while preserving validity. Initial error is measured against the stated exact initial path value; arbitrary input is never assigned zero error silently.

Each candidate component's Bernstein range must have distance strictly greater than \(E_i^{\rm whole}\) from the boundary of D. This checks the whole error tube, not just candidate endpoints. A first-exit argument makes the contraction estimate self-consistent; the bounded tube also permits solution continuation to the end of the interval. This supplies both a whole-path residual bound and a domain check across the discriminant crossing. The logic does not require any root to remain simple.

The emitted `certificate_*.json` files preserve every exact rational input node and record every slab's exact rational residual, whole error, propagated error and domain margin. They also record an independent bound on \(\widehat x-(p,p)\), obtained directly by Bernstein bounding that exact polynomial difference. The latter is reported separately; it is not used to inflate the apparent strength of the residual-only certificate. The certified object is the explicitly reconstructed cubic continuous path, not an unspecified RK4 dense output.

## Executed comparison and boundaries

`results.json` records twelve integration/certification pairs: both trace and all-root enumeration modes at n=8,16,32,64,128,256. The root baseline calls `numpy.roots` once at every state RHS evaluation and sums all four complex roots, with multiplicity, using the same prescribed control. For real coefficients these symmetric sums are real; the baseline discards only numerical imaginary roundoff. It does not select a real branch. The exact rational certificate is shared by both modes and certifies their reduced polynomial dynamics.

Trace RK4 node errors decrease with observed order about four. At n=256 the trace node error is about 7.80e-12 and the whole-path residual certificate is **5.752e-9**, below the 1e-8 budget. The direct exact-reference whole-path bound is about 2.901e-11. The root-enumeration version also passes, and its maximum node discrepancy from trace is about 2.22e-16.

The n=256 integration uses 1024 state RHS calls. Trace mode uses 1024 state trace batches and zero root enumerations; baseline uses 1024 root enumerations returning 4096 roots. Shared analytic control evaluation is counted separately. Certification adds 512 exact endpoint RHS evaluations and polynomial residual construction. There are no runtime Jacobian evaluations, factorizations, or root solves in trace integration or its certificate.

Five additional n=256 full integration-plus-certificate repetitions per mode measure the same certified budget. In the final campaign, median integration alone is approximately 0.00544 s (trace) versus 0.05611 s (roots), and median full time is approximately 0.700 s versus 0.906 s. This records an observed full-time ratio of 1.29 on this run. Rational certificate cost and host variation dominate; the modes were measured in sequential batches, so **the experiment does not establish a general or robust end-to-end speedup**. Symbolic identity preprocessing is timed separately. This fixed small polynomial example supplies no offline circuit-growth scaling result, general large-system speed claim, binary64 outward-arithmetic claim, or physical branch-reconstruction result.

Reproduce from this directory with:

```bash
PYTHONPATH=/workspace/scratch/eadbfca26db8/research_deps python -m unittest -v test_trace_client.py
PYTHONPATH=/workspace/scratch/eadbfca26db8/research_deps python symbolic_checks.py
PYTHONPATH=/workspace/scratch/eadbfca26db8/research_deps python run_experiments.py
python replay_certificates.py
```

The implementation uses NumPy for the independent numerical all-root baseline; the production trace RHS itself uses only scalar polynomial arithmetic. SymPy is used only for independent symbolic checks and exact complex-root tests. All twelve certificate artifacts replay exactly from their persisted rational nodes (`replay_results.json`); that is reproducibility using the same routine, not independent mathematical review. All work is isolated in this F6 directory; no baseline source was changed.
