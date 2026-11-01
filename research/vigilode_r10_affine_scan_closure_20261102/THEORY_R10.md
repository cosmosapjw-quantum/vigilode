# VigilODE R10 | Source-owned path-prefix theorem and theory-cycle closure

2026-11-02 KST. Selected PhysMath R10 only: durable science claims, stage-local constructive lemma, bounded search and next executable handoff, not a new R1–R10 cycle or C5 implementation. Parent PR #185 immutable HEAD 9694303d453c55ebcaf1ec4d851d0eb37cc87d52. Prospective preregistration commit dd076af6fd279ff478a6503652199d59d35ed003 was published before R10 source computations. Scope standalone original VigilODE G0, excluding BASS/REI applied work.

## Frozen source and original economic mission

Original eight-stage RODAS5P raw-U native source Git blob d68239d53977e36f98b186b627aa0c4c5c7de7cb, cooperative cubic RHS blob d78c5c5f6f476f2a5b1ea88628ed25daa6cfc482, original initial conditions fixture blob 1bfc17ec2ed4b902cc06073807388f6e332e31d6, original tableau SHA256 211255f6125b648c6955833c3da537d9c031ec78168b2918563c29fa88c8bce6. Original R9 exact source result SHA256 a39becdbc2c5855669c42fedfd69f740a3a159263c314927c221ec5fd80bd828. R7 real-native protected/transactional median wall ratio 0.21069269165 at same achieved whole-IVP error on 3 independent small original source systems. Preserve protected default, no unsupported native gain.

The actual Rosenbrock equation is W U_i=r_i with W=I-h gamma Df(y_n) frozen within step. The BDF/BE nonlinear root is a different mathematical stage. Source-owner supplies polynomial coefficients, including analytic 3*g_i*y_i^2 in W; runtime lacks JVP callback for this candidate but it is NOT a universal unknown-RHS derivative-free solver.

## Theorem R10.A: associative affine prefix within admitted path-forest graph

R9's source conditional cooperative cubic yields W=D(I-T), T>=0 and ||T||inf<1. Declared small feedback set B with |B|<=3 has acyclic complement A, source-authenticated time/epoch and original ROW stage RHS. For path-forest subclass A, each component is one directed path v0,...,v(m-1) with no additional incoming/outgoing A edges. The DAG inverse action R_A z=(I_A-T_AA)^-1 z on this component is exactly

   x_v0=z_v0;
   x_vi=z_vi+T_(vi,v(i-1))*x_v(i-1), i>=1.

Associate with root map F0(u)=z_v0 (pair (0,z_v0)) and each next affine map Fi(u)=t_i*u+z_vi (pair (t_i,z_vi)). Composition of outer (a,b) with inner (c,d) is (a*c,b+a*d), identity (1,0); it is associative but not generally commutative. Pair-composing F_i through F0 recovers exact x_vi in its additive coordinate.

By induction on recursive doubling stride 2^k, synchronous Hillis-Steele scan obtains every prefix in L=ceil(log2 m) rounds with

  N_pair=sum_{k=0}^{L-1}(m-2^k)=m L-(2^L-1)

affine pair compositions per path invocation (for m=1, zero rounds/compositions). Sequential topological propagation has depth m-1 and O(m) work. For a disjoint path forest, all component scans are independent, with total work the sum of their N_pair and depth max L.

This is a known associative-scan construction; see Ralf Hinze, An Algebra of Scans (2004), https://www.cs.ox.ac.uk/ralf.hinze/publications/MPC04.pdf and Seal, Perumalla and Hirshman (2012), DOI 10.1016/j.jpdc.2012.10.003. The project-specific contribution is exact application to R9 source-owned original Rosenbrock W inverse with measured work/span evidence and preserved source/error/fallback gates, not a new universal theorem.

## Theorem R10.B: same original stage root, different cost

The path scan changes ONLY evaluation order for R_A z, not the R9 graph-Schur equations

  S_B=I-T_BB-T_BA R_A T_AB;
  u_B=S_B^-1(b_B+T_BA R_A b_A);
  u_A=R_A(b_A+T_AB u_B);
  b=D^-1 r.

Consequently in exact rational arithmetic the new result is the same original W^-1 r as the R9 serial path solver, including all eight original ROW stages. Existing M-matrix row-margin residual-to-error certificate and the original R6 eight-stage lower-triangular residual propagation are preserved; no new native floating error enclosure or physical global truncation error theorem follows.

The scan must be applied once for each source-owned core setup column (|B| calls) AND once for each original stage RHS (8 calls per first step), plus remaining core operations, rejected speculative work and actual protected fallback. This source mathematical optimization reduces an INNER PATH dependency, NOT the full outer eight-stage causal depth. For general DAG branching/merging, refuse the specialized path scan; the previous serial graph-Schur or protected original RODAS fallback remains valid. Signed H-matrix residual bounds are a separate conjectural extension without M-matrix positivity.

## Actual R10 computation, evidence and negative controls

Four ORIGINAL frozen first-step cohort labels, 3 independent physical models and one resolution control, and four distinct SYNTHETIC graph controls: 8*8=64 original mathematical-form raw-U stage comparisons. The new scan was exactly equal in Fraction arithmetic to the R9 original-graph source solver on every stage; all retained original first-step error upper bounds agree exactly with R9. Additional SYNTHETIC n48 and n96 unidirectional rings validated exactly ONE source-owned W linear stage each, NOT original physical trajectories and not whole eight-stage runs.

| graph | sequential depth | scan layers | pair operations |
| --- | ---: | ---: | ---: |
| ring12 (synthetic R9 control) | 10 | 4 | 29 |
| ring24 (synthetic R9 control) | 22 | 5 | 84 |
| dag8 (synthetic R9 control) | 7 | 3 | 17 |
| ring48 (synthetic R10 one-stage holdout) | 46 | 6 | 219 |
| ring96 (synthetic R10 one-stage holdout) | 94 | 7 | 538 |

These are algebraic operation counts and idealized independent scan rounds, NOT measured native parallel execution times. The ring96 data falsify any general claim of less TOTAL work: 538 pairs vs 94 serial predecessor edge operations. A source-valid DAG with two incoming dependencies to one node was REFUSED because it is not a disjoint path forest. Source script and deterministic JSON SHA256 a95e69a9a42e18de608dab52f6c2f9b4054005e5f377ac384c350ba04e7c6535. Fourteen same-author adversarial numerical and source-lock tests PASS; existing official 23-node DAG remains exactly 7/23.

## Actual proof assistant and mutation harness

Offline OCaml5.3/OPAM2.6/HOL Light3.1 generated three named LCF-kernel thm objects from local hol_r10_scan.ml: affine composition associativity, exact 3-node path and 4-node path polynomial identities. Exit 0, marker HOL_VIGILODE_R10_SCAN_KERNEL_PASS_3 exactly once and zero Error/Exception diagnostics. These three are PARTIAL formal lemmas, not full arbitrary-length scan correctness or native ROW implementation correctness.

Actual OpenEvolve 0.4.0 ProgramDatabase + UCT MCTS + Gaussian-process expected improvement selection (three cold starts and five GP acquisition waves), fixed 4+4 operator quotas across 8 waves: 29 distinct evaluated candidates, 6 actual arm switches, 5 false mathematical claims refuted with witnesses, 8/8 waves include a nontrivial new math node and hybrid requiring two distinct integrators and theory families. Scores come from source-locked R9/R10 original results and real R7 native negative wall evidence, not proof probabilities or fabricated speedup. Same-author test suite 11/11 PASS, including separate process byte-identical replay with PYTHONHASHSEED=0. No external LLM calls or trained neural MCTS. Sandbox full source/output/visualization ZIP SHA256 627167ecf557e55ad18690b90887dd8e225a8377b7a4447d5db17d0c9d149ff4.

## Final R10 decision and stop

ADMIT: conditional original source affine scan mathematical result, exact original source-stage computations and partial HOL lemmas; no universal mathematical novelty claim. HOLD: native provider, native full-IVP performance, C9 distinct-author referee, general DAG path-scan, signed-H positivity, and production/default route change. Keep the proven fallback and the R7 measured slow transactional q2 negative result.

The theory selected R10 stage is complete when report, verified source results and executable next-stage contract are committed and backed up. Do NOT silently begin another R1 loop or implement C5. The next genuine CODING step if requested is C5: native source/clock/epoch authenticated graph-Schur+path scan fast W provider, original raw-U stage residual, exact error/rollback, all speculative and fallback cost accounting, matching global achieved-error full-IVP benchmarks on original and larger stiff/nonnormal/oscillatory holdouts. Failure/overhead implies protected RODAS5P, not tolerance relaxation.

OFFICIAL_DAG=7/23 unchanged; SUCCESSOR_ADMITTED=0/12; PRODUCTION=HOLD. No merge, main, original solver, default, tolerance or authority-checker modification.
