# R2 | Literature and predecessor reuse

## 1. Nonnormal stiffness is finite-time information

Higham & Trefethen, Stiffness of ODEs, BIT Numerical Mathematics 33 (1993), 285–303, DOI 10.1007/BF01989751. They explain why eigenvalues are asymptotic descriptors and can miss finite-time transient effects in nonnormal Jacobians; pseudospectra and the resolvent help expose this gap. Reuse: do not classify the solver regime by spectral radius/abscissa alone. A practical hierarchy is (i) spectral abscissa, (ii) Euclidean numerical abscissa/logarithmic norm, and (iii) a bounded resolvent/transient-gain probe when the first two disagree. A dense pseudospectrum calculation is not required on every step.

Söderlind, Jay & Calvo, Stiffness 1952–2012: Sixty years in search of a definition, BIT Numer. Math. 55 (2015), 531–558, DOI 10.1007/s10543-014-0503-3. Their stiffness indicator uses logarithmic norms to define a local dynamical time scale and distinguishes mathematical damping scales from method/accuracy-dependent step-size constraints. Reuse: measure a local damping scale along the trajectory; treat weakly damped high-frequency oscillation as a different regime, not automatically as classical stiffness.

For the Euclidean norm,
\[
\mu_2(J)=\lambda_{\max}\left(\frac{J+J^T}{2}\right),
\qquad
\|e^{tJ}\|_2\le e^{t\mu_2(J)}.
\]
For a stable but nonnormal J, spectral abscissa alpha(J)<0 can coexist with mu_2(J)>0, allowing initial transient growth.

## 2. Parallel affine scan

Blelloch, Prefix Sums and Their Applications, CMU-CS-90-190 (1990), gives the work-efficient all-prefix-sums primitive; Hillis & Steele, Data Parallel Algorithms, CACM 29(12) (1986), is a classic data-parallel construction. Reuse: treat affine maps as an associative but noncommutative monoid. Compare the existing Hillis-Steele implementation (logarithmic rounds but O(n log n) work) with a work-efficient tree scan (O(n) work, O(log n) ideal span) and a plain sequential recurrence (O(n) work, O(n) span). Also compare parallelizing independent path components, which preserves O(n) work without intra-chain scan barriers.

For maps F(x)=ax+b, define ordered concatenation of an earlier segment L followed by a later segment R as
\[
L\star R := R\circ L
=(a_Ra_L,\ a_Rb_L+b_R).
\]
Associativity follows from function composition; commutativity generally fails. Any scan must preserve segment order.

## 3. Symbolic dynamics and finite-state switching

Della Rossa & Jungers, Multiple Lyapunov Functions and Memory: A Symbolic Dynamics Approach to Systems and Control, SIAM J. Control Optim. 62(5) (2024), 2695–2722, DOI 10.1137/23M1589244. They connect constrained mode sequences and memory to multiple-Lyapunov analysis of switched systems. Reuse: encode solver-mode history as a finite automaton with hysteresis/dwell-time constraints rather than a memoryless threshold selector. Important boundary: a control-theory Lyapunov result applies to the matrices/maps whose stability it analyzes. It does not automatically certify a numerical-integrator selector. For a numerical solver, the relevant objects would be error-propagation/transfer maps and the accepted-step handoff maps.

## 4. Sparse factorization and graph costs

Davis & Duff, An Unsymmetric-Pattern Multifrontal Method for Sparse LU Factorization, SIAM J. Matrix Anal. Appl. 19 (1998), DOI 10.1137/S0895479894246905; Davis, Algorithm 832: UMFPACK V4.3, ACM TOMS 30(2) (2004), DOI 10.1145/992200.992206. Reuse: sparse-direct cost depends on ordering, frontal assembly, fill and memory traffic, not just nnz. The cost model must include symbolic analysis/factorization and repeated triangular solves. A narrow path forest is a special low-fill structure; irregular high-degree graphs can favor ordinary sparse/banded direct or Krylov.

## 5. Algebraic geometry guard

Collins, Quantifier Elimination for Real Closed Fields by Cylindrical Algebraic Decomposition, in Quantifier Elimination and Cylindrical Algebraic Decomposition (1975; reprinted in the 1998 volume), gives sign-invariant semialgebraic cell decomposition. Reuse: for low-dimensional polynomial source families, determinant, principal-minor, pivot and diagonal-margin polynomials can partition parameter space into regions with fixed sign/regularity decisions. Full CAD is often too expensive as a runtime tool; use it offline to derive compact guards, then evaluate conservative interval/sign checks online.

## 6. Concrete predecessor reuse map

| Existing project result | Current use | Relation |
|---|---|---|
| R10 affine prefix theorem and exact Fraction checks | ordered affine monoid and path-forest domain | direct, conditional on disjoint path forest |
| C5 native graph-Schur full-IVP evidence | source guard, original W residual, protected BandedStageLu fallback | direct for same frozen source and API |
| C5 result: graph/fast-direct median 2.242 | baseline economic negative result | direct; only 3 physical families, clipped-grid driver |
| PR #76 three-solver study | problem-dependent solver winners; Brusselator N=400 DVODE banded counterexample | historical empirical context, not transferable timing |
| PR #90 LuSolveWorkspace and zero-allocation LU scratch | preallocation/workspace reuse pattern | design reuse; not claimed already wired into C5 |
| PR #125 derivative-free BDF/BE comparisons | whole-IVP BDF-vs-RODAS is a distinct-method comparison | contextual only, never substitute BDF root for ROW stage |
| Existing Rayon dependency in rodas5p-integrators | parallel scan/chain experiment | direct package dependency; no Cargo dependency change |

## Source URLs

- https://people.maths.ox.ac.uk/trefethen/publication/PDF/1993_56.pdf
- https://doi.org/10.1007/BF01989751
- https://homepage.divms.uiowa.edu/~ljay/publications.dir/Stiffnessdefinition.pdf
- https://doi.org/10.1007/s10543-014-0503-3
- https://www.cs.cmu.edu/afs/cs.cmu.edu/project/scandal/public/papers/CMU-CS-90-190.html
- https://doi.org/10.1145/7902.7903
- https://doi.org/10.1137/23M1589244
- https://doi.org/10.1137/S0895479894246905
- https://doi.org/10.1145/992200.992206
- https://link.springer.com/book/10.1007/978-3-7091-9459-1

R2 conclusion: the current bottleneck is not a generic failure of graph-Schur mathematics. It is an implementation/economics question: extra scan work, allocation, source probing, residual checks and core setup must be amortized against the chosen direct factorization path.
