# M0 new independent reference protocol v1 (registered before new output)
The previous all-level verdict is immutable. New protocols are separate,
with the same fixed global WRMS threshold 1e-10, not a relaxation. A new
empirical reference is qualified only if BOTH directions of all cross-family
and successive-level discrepancies pass, each trajectory covers exact
archived 101 nodes, finite output and exact initial state. No extrapolation
or post hoc tolerance dropping. Reference agreement is not a rigorous bound.
1. Four low-dimensional canonical stiff models: Julia native BigFloat
   RadauIIA9 (order 9) at 128-bit working precision, rtol 1e-15,1e-17,1e-19;
   atol=original component scale*rtol*0.01. Independent Wolfram NDSolve
   linearly implicit/explicit extrapolation stiffness switching at working
   precision 40/50/60 and AccuracyGoal/PrecisionGoal 15/17/19. Use actual
   represented binary64 physical constants (including pre-rounded 1/77.27),
   initial data and time nodes. Wolfram availability failure remains blocked;
   do not substitute success. Resource cap 180s per case+method+level,
   Julia maxiters 1e6. Both method families are independent of historical
   RADAU5/CVODE implementations; all new levels required.
2. Brusselator and nine spatial systems: same archived Rust Model source
   identity, callback shared by independently implemented SciPy DOP853 and
   Julia Vern9. Binary64 rtol 1e-12,1e-13,3e-14; atol=reference_atol times
   0.01,0.001,0.0003 respectively. This is an empirical same-RHS comparison,
   not an independent spatial discretization; actual source and exact y0
   verified. 120s per trajectory, Julia maxiters 1e6. Linear spherical
   diffusion and two sphere photon models additionally examine represented
   linear operator via action on basis and matrix exponential as an
   orthogonal diagnostic, not replacement for required ladder.
3. If both new paths converge and agree in the unchanged metric, qualify
   separate empirical reference from the finest native Julia output, plus
   report both families' finest discrepancy and historical discrepancy.
   No change to historical dispute flags or new matched-error classification.
4. Extra diagnostic high-order or high-precision runs may explain failure
   but cannot retroactively remove a failed level. Incomplete cases remain
   unresolved or bounded blockers. All failure rows retained.
