# Prospective source-locked experiment: VigilODE nonautonomous REI photon forcing

Date: 2026-11-01 KST. Baseline: VigilODE PR #180 HEAD 8cad5fdbcbde883be61f50ffdb51b3212aa2eab9. Exact source/references from PR #175. Original 46,080-node source with three global neutral moments and four photon groups; five forcing clock knots. This 15-state photoionization-only client is NOT the complete original 26-event MPRK22(1)+Alexander SDIRK2 IGM owner.

## Mathematical source declaration defect
Original five-clock client declares autonomous=true and partial_t=None even for timevarying forcing. Original RODAS5P thus returns f_t=0. For n_s=y_s>0 and group g, D_g=e_g(u)+sum_s w_gs(u)n_s, P_s=sum_g J_g(u)w_gs(u)n_s/D_g, E_g=J_g e_g/D_g. At fixed n,
D'_g=e'_g+sum_s w'_gs n_s,
P'_s=sum_g n_s[(J'_g w_gs+J_g w'_gs)/D_g-J_g w_gs D'_g/D_g^2],
E'_g=(J'_g e_g+J_g e'_g)/D_g-J_g e_g D'_g/D_g^2.
Derivative is per original normalized source time u. Original five-clock piecewise-linear forcing is C0 and not C1 at interior knots. Split the full integration into each original knot segment and supply the selected segment's one-sided analytic f_t to the existing matrix-free 8-stage RODAS5P.

## Predeclared method experiment
Keep original source/state/JVP/Radau reference and baseline CLI unchanged; add an opt-in Rust companion. Independently check analytic time derivative at strict interior clock times against central finite differences, and group photon ledger derivative identity. Compare corrected nonautonomous RODAS5P to existing RHS-only DOPRI5 on identical source, state, five segments and 11 original output sample times; frozen source is an autonomous control if affordable. Historic original misdeclared solver remains an evidence baseline, not a correct f_t oracle.

Fixed trial grid: rtol {2e-9,2e-11}, atol=max(2e-14,rtol*1e-3); first h <=0.01; max h<=0.1; 20000 attempted steps per segment. Independent original Radau/DOP853 reference benchmark; physical source-scoped original max absolute state-error budget 5e-10, source invariants maximum 2e-11. Record accepted, rejected, RHS, JVP and f_t work and all failures. Wall times are diagnostic only. Do not claim generic matched-error speedup without same achieved-error evidence. Exponential PEXPRB54S4 is autonomous-only and ineligible for the time-varying arm without another mathematical reformulation.

## Scope locks
No full 26-event heat/OTS, no new parent state, no independent-cell batching for shared three moments, no Hummer-Seaton event bypass, no clock/tolerance/source replacement. No canonical REI [160,161] admission. VigilODE 7/23, successor 0/12; REI central 28/88, working 33/88, Fast 4/7; delta 0; production HOLD; NO_MERGE/NO_FORCE_PUSH/NO_DEFAULT_CHANGE/NO_TOLERANCE_RELAXATION/AUTO_NEXT_NODE=false. Stop after one real source science result and same-author adversarial review; backup/PR only once actual evidence exists.

STATUS=PREREGISTERED_BEFORE_NEW_EXPERIMENT.
