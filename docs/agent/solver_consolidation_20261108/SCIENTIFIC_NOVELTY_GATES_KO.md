# VigilODE scientific novelty / significance gate

**Status:** novelty not established; candidate theorem program is CONJECTURAL. This is a claim gate, not a claim of discovery.

## 1. Prior-art ceiling

| Idea | Adjacent prior art | Not a novelty claim |
|---|---|---|
| Parallel two-step W / peer stages | Podhaisky 2002; Schmitt & Weiner 2004; Weiner et al. MOL 2003/04 | first parallelization of stiff ODE stages |
| Multi-implicit Peer-W / different shifts | Schmitt, Weiner & Podhaisky 2005 | parallel independent different shifts |
| PTSW local/global error control | Kulikov & Weiner 2011; 2015 | first Peer-W plus global error control |
| Variable-grid stiff norm / Peer | Lang & Schmitt, arXiv:2608.26913 (2026-08-27) | first variable-step Peer stiff norm bound |
| Residual-controlled Krylov / matrix-free | classical Krylov/exponential integrators; VigilODE PR74/90/105–112 | avoiding explicit Jacobian and using Krylov residual |
| Homotopy / fixed-point / inverse-Picard | continuation literature; VigilODE PR74/91/117/154/156/177 | Newton-free nonlinear solve alone |
| Finite-state / dwell-time switching | Della Rossa & Jungers 2024, SIAM JCO 62(5), DOI 10.1137/23M1589244 | symbolic dwell-time router alone |
| Semialgebraic guards / CAD | Collins CAD and real algebraic geometry | polynomial sign partition alone |
| Nonnormal stiffness/logarithmic norm | Higham & Trefethen 1993; Söderlind, Jay & Calvo 2015 | numerical-abscissa/transient diagnostic alone |
| Associative scan/affine prefix | Blelloch 1990; Hillis–Steele 1986 | parallel prefix for affine maps |
| Adaptive method selection / bounded pilots | method switching, algorithm portfolios, cost-based selectors; VigilODE switching work | an automatic dispatcher, pilot, hysteresis, bandit or MCTS label alone |

A selector choosing among existing algorithms is not itself novelty. Any numerical guarantee is method/table/source/grid/ratio specific; a 2026 Peer norm theorem does not automatically certify a different PTSW coefficient table.

## 2. Most promising theorem candidate

### N1. Source-attested compositional global-error certificate across inexact stage solves and portfolio transitions

For original (Wk=b), executed (widetilde W), computed (widetilde k), residual (widetilde r=b-widetilde Wwidetilde k):
[
W(widetilde k-k)=(W-widetilde W)widetilde k-widetilde r,
quad
|widetilde k-k|leq|W^{-1}|
(|W-widetilde W||widetilde k|+|widetilde r|).
]
This separates Jacobian/W approximation mismatch from actual linear-solve residual; it does not yet bound a nonlinear multistage method.

For a specified one-step map (Psi_n), Lipschitz factor (Lambda_n), local defect (d_n), executed-map perturbation (eta_n):
[
E_{n+1}leqLambda_nE_n+d_n+eta_n,qquad
E_Nleqleft(prod_{j=0}^{N-1}Lambda_jight)E_0+
sum_{k=0}^{N-1}left(prod_{j=k+1}^{N-1}Lambda_jight)(d_k+eta_k).
]
This is standard error propagation, not novelty. Peer/multistep methods require the full history in the augmented state. Method switches add history/state-transfer operators and their defects; norm changes need multiple-Lyapunov compatibility or dwell-time product bounds.

Potential contribution: a theorem and executable receipt composing truncation error, original-W mismatch, inexact Krylov/direct residual, source transform/chart defect, roundoff/output defect, method-history transfer and admissible supervisor transitions into one fail-closed global error bound, while accounting for all speculative/failed/fallback work.

Required: fix one method family; derive its exact stage-block perturbation map; bound resolvent/inverse norms over full ratio/operator domain; control nonnormal transient products; prove transform/chart bounds; prove admissible transition stability; compare predicted/achieved errors on adversarial stiff/nonnormal/oscillatory/nonautonomous/MOL cases; show the certificate's cost is amortized.

Falsifiers: unbounded transition product; missing source/branch identity; unusably loose bound; equivalent published theorem; certificate/setup/fallback cost removes full-IVP gain; adversarial nonnormal family violates claimed norm envelope.

## 3. Adaptive portfolio and switching: contribution boundary

The architecture is a **sparse, selectively activated portfolio**, not a universal pipeline. Existing `AutomaticOde` is currently a limited opt-in DOPRI5-versus-authorized-RODAS selector, with default derivative permission forbidden. C8 R2/R4 is a scoped finite-state router proposal; C8 numerical validation remains pending.

Potential contribution is not “automatic selection” itself. It would require a solver-specific transition contract that:
- filters capabilities before callbacks;
- probes only when uncertainty/deterioration justifies the probe cost;
- minimizes the complete remaining-IVP cost over admissible candidates;
- preserves original source/output/global-error authority;
- transfers method history or restarts with a bounded defect;
- proves stability/error propagation for the actual accepted-step transition maps;
- retains per-regime negative evidence and prevents probe-induced economic regression.

No global optimality/regret claim is made without a corresponding proof. The controller's “best choice” is conditional on its admitted candidate set, evidence and remaining budget.

## 4. Other contribution candidates

### N2. Semialgebraic source-attested structural admission
Offline exact sign-invariant decomposition of low-dimensional polynomial source families into graph/diagonal-margin/chart cells, with compact runtime interval guards and direct original-equation residual. CAD itself is established; only a new source-specific certificate plus economic benefit may be publishable.

### N3. Concurrency/accuracy Pareto method design
Construct/select a Peer-W/SEPP coefficient table whose actual stage DAG, variable-grid stability and method-specific global error controller improve a provable Pareto frontier over closest tables and RODAS/BDF/Radau. Parallel Peer-W itself is established prior art.

### N4. Certified source-specific structural correctors
RHS-only homotopy, polynomial chart, quotient/Lawson, Volterra and weighted observable may matter where they produce a valid original-target residual and an amortized full-IVP gain beyond existing source-scoped work. Combining known tools alone is not novelty.

## 5. Scientific significance criteria

A result must show one or more of:
- lower complete-IVP CPU at identical achieved global error and acceptable accepted-step count;
- no explicit Jacobian or derivative action in a nontrivial declared RHS-only domain;
- certified operator economy preserving original W/ODE residual;
- observed CPU overlap of genuinely independent stages in a named method;
- reliable switching with history/epoch rollback and a transition theorem;
- geometry/structure leverage on MOL with correct measure, invariants and operator conditioning.

## 6. Competitive comparison

Use identical source/IC/span/output/reference. Compare protected RODAS5P/JVP, appropriate direct/banded/sparse RODAS, BDF/Radau only after comparator qualification, closest published PTSW/Peer/SEPP with exact coefficients/startup/estimator, and the complete candidate including startup/history/rejection/events/output/transforms/fallback. Use ≥7 randomized paired CPU blocks where feasible, achieved-error curves, uncertainty, original residual, accepted/rejected steps, RHS/JVP/Jacobian/linear/preconditioner work, allocations/peak RSS, stage depth and observed concurrency. Work counts and synthetic kernel timing are not complete-IVP time.

## 7. Claim status

| Claim | Status |
|---|---|
| RODAS5P has 8 causally ordered stages | ESTABLISHED |
| JVP-based matrix-free RODAS is derivative-action based | ESTABLISHED |
| C5 graph-Schur full-IVP slower in tested scope | NUMERICALLY_CHECKED negative |
| R10 affine prefix associative on admitted path-forest maps | DERIVED; scan economics pending |
| Peer stages independent for selected coefficient families | LITERATURE_SUPPORTED |
| PTSW2B/2C tableau certificates | scoped exact algebra; no VigilODE runtime |
| C8 finite-state router | candidate design; numeric validation pending |
| PR98 guarded-word ownership refinement | scoped implementation verified; not physical ODE coding |
| Compositional global certificate across inexact stages and switches | CONJECTURAL / UNRESOLVED |
| Full adaptive portfolio performance/production qualification | NOT_ESTABLISHED; HOLD |

## References
- Podhaisky (2002), Parallele Zweischritt-W-Methoden, DOI 10.25673/3017.
- Schmitt & Weiner (2004), Parallel Two-Step W-Methods with Peer Variables, DOI 10.1137/S0036142902411057.
- Schmitt, Weiner & Podhaisky (2005), Multi-Implicit Peer Two-Step W-Methods, DOI 10.1007/s10543-005-2635-y.
- Kulikov & Weiner (2011), Global Error Estimation and Control in Linearly-Implicit Parallel Two-Step Peer W-Methods, DOI 10.1016/j.cam.2011.08.006.
- Kulikov & Weiner (2015), A Singly Diagonally Implicit Two-Step Peer Triple with Global Error Control, DOI 10.1137/140979952.
- Lang & Schmitt (2026), Stiff Norm Estimates for Variable-Stepsize Peer Two-Step Methods, arXiv:2608.26913.
- Higham & Trefethen (1993), Stiffness of ODEs, DOI 10.1007/BF01989751.
- Söderlind, Jay & Calvo (2015), Stiffness 1952–2012, DOI 10.1007/s10543-014-0503-3.
- Della Rossa & Jungers (2024), Multiple Lyapunov Functions and Memory, DOI 10.1137/23M1589244.
- Blelloch (1990), Prefix Sums and Their Applications, CMU-CS-90-190.
- Collins (1975), Cylindrical Algebraic Decomposition; Coste, An Introduction to Semialgebraic Geometry.


## 3. Peer-W prior-art boundary and new falsifiable target

The high-order fully stage-parallel Peer-W concept is established prior art. Podhaisky–Schmitt–Weiner (2002) report parallel W methods with p=s and stage order q=s, while Schmitt–Weiner (2004) prove order p=s-1 and stability for general step sequences for an independent-stage Peer-W subclass up to order seven. Kulikov–Weiner (2011) already provide global/local error estimation and automatic control for linearly implicit parallel two-step Peer-W. A claim of first parallel stiff Peer or first Peer global-error control is therefore excluded.

The next concrete candidates are PTSW2C (order 3, A(81.1 degree)-stable source result, not L-stable) and PTSW2B (order 2, source L(90)-stable control). Neither reaches G0. For G0 the source recovery task must identify and independently verify either a five-stage p=q=5 W tableau or a six-stage p=5 Peer-W tableau. Family-level literature existence is not a qualified coefficient set.

The strongest current conjectural contribution remains a source-attested compositional certificate that combines original stage residuals, inexact W solves, source/homotopy defects, peer-history/startup transfer, step rejection and transactional method changes into one global-error budget, paired with complete CPU work/span accounting. It must be proved for the actual selected tableau and falsified on step-ratio products, nonnormality, poor conditioning, abrupt stiffness changes and failure/rollback cases. Prior art on Peer global-error estimation makes this a narrow integration theorem target, not a guaranteed novelty.

Derivative policy: a W-method may avoid nonlinear Newton and explicit dense J, but a JVP-based operator remains derivative-based. The strict derivative-free homotopy worker belongs to a separate implicit Peer family and must certify the original stage equation. If a residual-to-local-error theorem fails, retain the route as an experimental solver only or NO-GO.


## 4. PW07: heterogeneous transactional switch certificate (2026-11-09)

Kulikov-Weiner (2011) already estimate/control local and global error in linearly implicit parallel Peer-W. ITSP3 (Kulikov-Weiner 2015) targets a true scaled global-error bound for a singly diagonally implicit Peer triple. Constantinescu (2017) already uses coupled GL solution/defect propagation under same-order, common-initial-data and proportional-leading-LTE hypotheses. Arevalo et al. (2021) show adaptive LMM error models depend on recent step ratios/controller history. Lang-Schmitt (2026) provide a family-specific variable-step Peer stiff-norm analysis. These exclude generic claims about Peer global control, coupled solution-defect estimation, or affine scan.

The remaining candidate is a **source-attested heterogeneous transactional method-switch certificate** combining original-W residual and operator perturbation, complete two-step history/startup transfer gain/defect, method-specific nonnormal propagation factor, RHS-only homotopy root residual where applicable, rounding/output and rejection/restart defects in one global IVP budget. PW07 derives the conditional recurrence but does not validate constants for general VigilODE sources. Novelty remains CONJECTURAL / UNRESOLVED.

Stop if source-bound propagation kappa, transfer chi/eta, original residual or rounding enclosure is missing. Ordered affine scan can evaluate a supplied recurrence with O(N) work/O(log N) span, but Blelloch scan is known prior art and does not parallelize the IVP. See PEERW_ERROR_CERTIFICATE_THEORY_20261109.md and PEERW_NOVELTY_PRIOR_ART_BOUNDARY_20261109.md.
