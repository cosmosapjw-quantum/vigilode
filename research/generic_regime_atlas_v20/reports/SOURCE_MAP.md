# External source map

The canonical atlas code and claims are grounded primarily in repository evidence. External
literature is used only for method-selection and benchmark-family context.

- Linda Petzold, *Automatic Selection of Methods for Solving Stiff and Nonstiff Systems of
  Ordinary Differential Equations*, SIAM J. Sci. Stat. Comput. 4 (1983), 136–148,
  DOI 10.1137/0904010. Relevance: end-of-step information, problem-character changes,
  and low selector overhead.
- INdAM-Bari Test Set, Robertson problem. Relevance: canonical Robertson is a three-state
  chemical-kinetics problem with a fast transient followed by a smooth phase.
- INdAM-Bari Test Set, van der Pol problem. Relevance: canonical stiff van der Pol uses a
  large nonlinear parameter; the present replicated ramp is not the canonical test.
- CWI, *Test set for IVP solvers* (1996). Relevance: diverse ODE/IDE/DAE problem families
  are required for solver comparison.
- Gaudreault, Rainwater, Tokman, *KIOPS: A fast adaptive Krylov subspace solver for
  exponential integrators*, JCP 372 (2018) 236–255, DOI 10.1016/j.jcp.2018.06.026.
  Relevance: fused phi-function actions and adaptive Krylov work.
- Deka and Einkemmer, *Efficient adaptive step size control for exponential integrators*,
  Comput. Math. Appl. 123 (2022) 59–74, DOI 10.1016/j.camwa.2022.07.011.
  Relevance: cost per step depends on step size when matrix-function actions are iterative.
