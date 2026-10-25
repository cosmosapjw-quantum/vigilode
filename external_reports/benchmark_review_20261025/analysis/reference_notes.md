# Benchmarking literature and additional cost analysis

Checked on 2026-10-25. SciSpace was used for bibliographic discovery; Exa fetched the primary publication and official project pages. The accompanying JSON contains sources, retrieval status, and suggested uses. No private manuscript was sent to external services.

## Verified primary references

1. P. N. Brown, G. D. Byrne and A. C. Hindmarsh, “VODE: A Variable-Coefficient ODE Solver,” *SIAM Journal on Scientific and Statistical Computing* **10**(5), 1038–1051 (1989). DOI: https://doi.org/10.1137/0910062 . Publisher text: https://epubs.siam.org/doi/10.1137/0910062 .

   VODE combines variable-coefficient Adams–Moulton and backward differentiation methods in Nordsieck form. The paper describes Jacobian reuse and associated efficiency changes. For this report it supports explaining the comparator family and the importance of counting reuse; the paper's historical speed figures should not be mixed with VigilODE measurements. The publisher's “31 July 2006” date is electronic publication; the journal issue is September 1989.

2. G. Steinebach, “Construction of Rosenbrock–Wanner method Rodas5P and numerical benchmarks within the Julia Differential Equations package,” *BIT Numerical Mathematics* (2023). DOI: https://doi.org/10.1007/s10543-023-00967-x . Full publisher text: https://link.springer.com/article/10.1007/s10543-023-00967-x .

   Section 3 explicitly sets **s=8**; the embedded fourth-order formula uses seven stages. The fifth-order method is constructed with additional conditions addressing stiff order reduction, and the article derives dense-output formulas. The report should distinguish the classical coefficient formula from VigilODE's mechanisms for carrying out or accelerating its stage equations. “Six RODAS5P stages” is incorrect. State fifth-order convergence under the applicable assumptions rather than importing an unconditional fifth-order PDE assertion.

3. SciML, *SciMLBenchmarks.jl: Benchmarks for Scientific Machine Learning and Equation Solvers*, official project methodology page, https://sciml.github.io/SciMLBenchmarks.jl/ , accessed 2026-10-25.

   Its reference-solution section explains that work–precision comparisons need a sufficiently accurate reference and that an error plateau shared by several methods can indicate reference error. Use attained error against computational work, not tolerance alone. This is directly relevant to the 17 current native cases without a verified reference: their run times can describe execution, but accuracy-normalized ranking requires reference construction. The page does not establish that every plateau is reference error; rounding and implementation effects need separate diagnosis.

4. SciML, *Rosenbrock methods*, official OrdinaryDiffEq documentation, https://docs.sciml.ai/OrdinaryDiffEq/stable/semiimplicit/Rosenbrock/ , accessed 2026-10-25. This is an implementation reference for solver nomenclature and current capabilities. The original Steinebach paper is the authority for the stated stage count and derivation.

## Short analysis suitable for the report

Let B be the baseline cost per solve, P the candidate's solve cost, C the candidate's necessary certification cost, and S its one-time setup cost. Assume these are fixed for the measured workload and that setup is reusable across m solves. Then

\[
\mathcal S(m)=\frac{mB}{S+m(P+C)}.
\]

For B>0, P>0, C≥0, S>0 and m>0, the candidate is faster exactly when

\[
B>P+C,\qquad m>\frac{S}{B-P-C}.
\]

Indeed the saved work is m(B-P-C)−S. The limiting speedup is B/(P+C), and its derivative with respect to m is BS/[S+m(P+C)]²>0. This is an elementary cost identity, useful for interpreting reused factorization and original-residual verification. It is not a new solver convergence theorem. Wolfram Language independently checked the logical equivalence and exclusion of B≤P+C: `Reduce` returned `{False, True, False}` for the two counterexample searches and the algebraic identity. For a numerical application, extract S,P,C,B from the *same* run family; do not substitute certificate-only speedup for full-route speedup.

For a solver with an unaffected fraction 1−f and an accelerated fraction f, replacing the latter by a component speedup r gives total speedup 1/(1−f+f/r). This follows by normalizing the original time to one. It makes the measured certificate-only benefit interpretable without equating it to whole-integration acceleration.

## Statistical presentation recommendations

- Treat an independently launched process as the replication unit when estimating variability across executions. Multiple solves inside the same process share thermal state, allocation state, and often a single coefficient setup; display these as within-process observations rather than separate process samples.
- When method A and B are executed as a matched pair in the same experimental block, compute each paired ratio B_i/A_i. Report its median and a range or an explicitly labelled paired-bootstrap interval when enough independent blocks exist. Do not silently pair separately ordered batches by row number.
- If only one process exists, show descriptive within-process distributions. Avoid a confidence interval purporting to generalize across hosts or independent processes.
- Keep all failed/timeout cases in the coverage table. A performance ratio is defined only where both methods satisfy the same measured accuracy and output contract. Display missing references and unavailable comparators rather than assigning them zero time or omitting them from a success denominator.
- Use runtime versus attained error (both axes log where positive), plus separate work-counter and setup/solve/certification plots. Distinguish a mathematical residual bound from measured error.

These statistical recommendations are analysis choices derived from the sampling structure, not claims that the SciML page endorses a particular confidence-interval estimator.
