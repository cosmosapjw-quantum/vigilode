# Phase 2 — Evidence acquisition

## Primary evidence

1. SciMLBenchmarks benchmark policy: comparisons should be optimized, fair, reproducible, and measured at approximately matched error or through work-precision diagrams.  
   https://docs.sciml.ai/SciMLBenchmarksOutput/
2. Current stiff ODE work-precision examples use high-accuracy reference solutions, repeated timings, and endpoint/L2 errors; solver ranking changes with problem and accuracy regime.  
   https://docs.sciml.ai/SciMLBenchmarksOutput/dev/StiffODE/VanDerPol/  
   https://docs.sciml.ai/SciMLBenchmarksOutput/stable/StiffODE/ROBER/
3. SUNDIALS CVODE uses method-specific local error control and variable-order BDF machinery, so equal input tolerances do not imply equal external global error across families.  
   https://sundials.readthedocs.io/en/latest/cvode/Mathematics_link.html
4. PETSc exposes method-independent tolerance controls through `TSAdapt`, while global-error control is available only for methods that provide the required information.  
   https://petsc.org/release/manual/ts/
5. Higham's tolerance-proportionality analysis shows that the relation between user tolerance and global error is method/controller dependent rather than a universal identity.  
   https://doi.org/10.1016/0377-0427(93)90277-I

## Local evidence inherited from v0.9

- Four native BDF/Radau anchors pass order, stiff-decay, and noncommuting-mass gates.
- Existing RODAS5P, SABR, homotopy, and Krylov candidates have a common Rust backend and work counters.
- The missing authority is external global error versus total cost, not another internal residual threshold.
