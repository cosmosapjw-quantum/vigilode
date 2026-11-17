---
schema: vigilode.production-solver-audit-roadmap.v1
document_id: VIGILODE-PROD-ROADMAP-20261104
document_version: 1
created_at_kst: 2026-11-04T06:23:06+09:00
project: cosmosapjw-quantum/vigilode
repository: /home/cosmosapjw/vigilode
branch: research/switcher-v2-20261103
head: 54297bb65e3c1337e29fe98072fa4e95e32df94e
parent: dc60d6f6bf372c1b866fc51a55cf912c9cd37378
remote_branch_head: 4339facc62258b5fb9a2469f16a3af8b93763e24
ahead_commits: 29
pull_request_for_branch: none_found
tracked_worktree: clean
untracked_at_snapshot:
  - CTSH_Chat_Native_Theory_Search_Harness_v2.0.2.zip
  - VigilODE_OPTIMIZATION_PLAN_MACHINE_20261104.md
ctsh:
  version: 2.0.2
  archive_sha256: eddc8ad6e41e3a72e1e6a9cf1460c1d4de39c405752970d56b718cc4ad859121
  archive_members: 57
  archive_checksums: pass
  pytest_with_lean_toolchain: 55_passed
  wolfram_encoded_identity: symbolically_checked
  accepted_candidates_open: 3
  oracle_promotions: 0
official_dag:
  total_nodes: 23
  scoped_closed: 7
  local_pending: 15
  authority_pending: 1
  successor_admission: 0
  successor_total: 12
production:
  status: HOLD
  merge_allowed: false
  default_change_allowed: false
  tolerance_relaxation_allowed: false
  retroactive_gate_change_allowed: false
plan_dag:
  total_nodes: 9
  accepted_implementation_nodes: 0
  audit_complete: true
  stop_condition: no_default_change_without_final_release_gate
---

# VigilODE production solver: critical audit and research roadmap

## 0. Executive decision

Current decision: HOLD production promotion.

The active repository state is branch research/switcher-v2-20261103 at HEAD 54297bb65e3c1337e29fe98072fa4e95e32df94e. The tracked worktree was clean at the snapshot time. The branch is 29 commits ahead of origin/research/switcher-v2-20261103 (4339facc62258b5fb9a2469f16a3af8b93763e24); no PR for this branch was found. The current HEAD therefore represents local Git history, not a published or merged production release.

The original RODAS5P route remains the protected baseline. Newer routes and algorithms are research candidates, even where code, unit tests, and small numerical experiments exist. In particular, the latest commit message claims “Gate G5 PASS”, but its comparator does not enforce the declared G5 contract. The measured baseline/candidate comparison is useful evidence of default-path neutrality, not evidence of speed improvement: 96/96 work-counter tuples match exactly and the geometric-mean work ratio is exactly 1.0000.

Three certificate-soundness issues in polynomial/Leja work are release blockers:

1. Product Chebyshev defect estimation uses the maximum of per-factor inverse values rather than a bound on the normalized product over the union of spectral intervals. A concrete two-interval example underestimates the actual defect by about 9.9845.
2. The Gershgorin helper clips discs crossing the imaginary axis into strictly negative real intervals. For a positive scalar matrix it can return a negative interval that excludes the actual eigenvalue while being labelled Certified.
3. Complex Leja certification accepts a caller-supplied boolean is_normal and a caller-supplied spectral scale beta; neither normality nor spectrum inclusion is verified at the operator boundary.

These issues must be resolved before any polynomial or complex-Leja result contributes to a certified production error or stability argument.

## 1. Authority and provenance

### 1.1 Repository state

| Field | Value |
|---|---|
| Repository | cosmosapjw-quantum/vigilode |
| Branch | research/switcher-v2-20261103 |
| HEAD | 54297bb65e3c1337e29fe98072fa4e95e32df94e |
| HEAD parent | dc60d6f6bf372c1b866fc51a55cf912c9cd37378 |
| Remote branch head | 4339facc62258b5fb9a2469f16a3af8b93763e24 |
| Ahead | 29 commits |
| PR | None found for this branch |
| Tracked worktree | Clean |
| Existing untracked files | CTSH ZIP and pre-existing optimization plan |
| This document | New audit/roadmap artifact; not committed or posted |

The two existing untracked artifacts were preserved. This document is intentionally separate from VigilODE_OPTIMIZATION_PLAN_MACHINE_20261104.md; that pre-existing file was not overwritten.

### 1.2 Current HEAD composition

| Commit | Scope | Authority interpretation |
|---|---|---|
| c27b2913 | WP-00..WP-03 and WP-05/WP-08 foundations | Code implementation; needs the relevant source-bound gates |
| 5250dfad | WP-04 DOPRI5 spectral/secant screen and WP-06 Chebyshev4 JVP rung | Screen failed its off-registry agreement gate; observer only |
| a73bc4d0 | WP-11 Hermite splice and WP-14 preconditioned fixed-point map | Unit/integration tests exist; not yet a production admission |
| dc60d6f6 | WP-07 secant preconditioner, WP-09 Chebyshev polynomial generation, WP-10 complex Leja phi action | Public/additive research APIs and tests; certificate and end-to-end gaps remain |
| 54297bb6 | Candidate battery artifact and comparison tool | Comparator audit finds the claimed G5 pass unsubstantiated |

The active ledger ends at L-0214. No ledger entries were found for WP-07, WP-09, WP-10, WP-11, WP-14, or the latest battery comparison commit. Their presence in HEAD does not itself close an official DAG node.

### 1.3 Official DAG state

Authority: docs/vigilode_original_mission_realign_20261101/REFOCUSED_DAG.json.

- 23 original local-development nodes.
- 7 scoped closures.
- 15 local nodes pending.
- 1 authority-pending node.
- 12 successor nodes, with 0 admitted.
- Milestones M1/M2 are ready for scoped execution; M3/M4 remain blocked on evidence and qualification.
- The DAG explicitly forbids automatic next-node selection, merge, default change, tolerance relaxation, and retroactive gate changes.

The new roadmap DAG below is separate from this 23-node official DAG. It has nine proposed research/qualification nodes, currently 0/9 implementation nodes accepted. The audit itself is complete, but it does not count as an implementation-node closure.

## 2. Evidence inventory

Evidence classes used in this document:

- PASS_SCOPED: a preregistered result passed only for the stated domain and data.
- FAIL: a preregistered gate failed. Preserve the failure and its default/refusal consequence.
- INVALID: the earlier claim was invalidated by provenance or gate changes.
- IMPLEMENTED_UNVERIFIED: code exists, but its mathematical or numerical contract is not closed.
- NUMERICALLY_CHECKED: a finite set of calculations was executed; this is not a general theorem.
- SYMBOLICALLY_CHECKED: the exact encoded expression simplified successfully in Wolfram; statement equivalence remains separate.
- BLOCKED: execution or authority is incomplete.
- OPEN: conjecture or obligation without a successful oracle/admission.

### 2.1 Ledger rows L-0205 through L-0214

| Ledger | Verdict | Actual scope / consequence |
|---|---|---|
| L-0205 | PASS, superseded by L-0208 | Initial E0 admission claim was later narrowed |
| L-0206 | INVALID, superseded by L-0207 | WorkModel V2 used literals changed after preregistration; do not reuse as a valid calibration |
| L-0207 | INVALID | Re-run with verbatim preregistered inputs failed: max relative error 0.5015 and transition-39 gate failed |
| L-0208 | PASS, scoped | Only default-path neutrality under SwitchingConfig::evidence_driven was admitted; opt-in strict/seed/sentinel/dominated flags were not exercised |
| L-0209 | FAIL | SharedBasis reduced JVP vectors by 6.22x–6.29x, but its per-stage dense Householder QR projection cost more instruction work than preconditioned per-stage GMRES; keep opt-in |
| L-0210 | FAIL | Both smaller first-batch rules delayed stiff-forced implicit entry and increased RHS calls from 400 to 1840 / 892; retain original max(budget, k_d0) |
| L-0211 | FAIL | Live Callgrind WorkModel V2 calibration failed: transition-39 w_v2=264.2 WU >= 200 WU, Spearman rho 0.7667 < 0.90; V1 remains default |
| L-0212 | FAIL | Hysteresis bias |b|=1.2069 exceeded every candidate half-band; entry/return margins remain unset |
| L-0213 | FAIL | DOPRI5 secant stiffness gate achieved only 0.5833 off-registry agreement on nonlinear 1-D Brusselator, below 0.90; stiffness_gate_fraction=None remains default |
| L-0214 | PASS_SCOPED | CPR coloured assembly passed for 1-D heat and 1-D Brusselator at n=256 and three tolerances; 3/4 colours, dense-FD assembly difference 3.2e-14, state difference 2.1e-14, and 2.46x–2.87x lower Ir than SharedBasis. No holdout was run. |

The invalidation rows are not optional historical details. L-0206 must not be quoted as a valid WorkModel V2 success. L-0208 is narrower than the original L-0205 wording. K1, B7, E1R, H1, and D1 remain failed research results and must not be silently reclassified.

### 2.2 Historical graph-Schur and path-forest results

The preceding original-G0 graph work remains useful as scoped research, not as a production replacement:

- L-0203: source-bound path-forest affine-prefix theory closure passed its stated exact and synthetic controls. It did not demonstrate a native full-IVP speed gain.
- L-0204: native C5 graph-Schur implementation compiled and its focused tests passed. The 36 measured whole-IVP cases gave median graph/fast-direct wall ratio 2.242, i.e. graph-Schur was slower on that tested small source. Default promotion was rejected for that scope.
- Earlier native graph and path methods should be revisited only on large sparse, stiff, nonnormal holdouts, with the same achieved global error and full setup/solve/fallback cost. Do not discard them merely because the small-source case was slower, but do not infer a large-system advantage either.

### 2.3 Latest 96-case baseline/candidate comparison

Source artifacts:

- bench/out/opt_baseline_4339facc.json
- bench/out/opt_candidate_dc60d6f6.json
- tools/compare_opt_battery.py

Observed comparison:

- 96/96 cells present in both files.
- 8 problems × 4 tolerances × 3 route policies.
- 96/96 work-counter tuples match exactly.
- Geometric mean W_base / W_candidate = 1.0000.
- Work ratio minimum and maximum are both 1.0000.
- 12 adaptive-switcher cells recorded a route switch in the candidate artifact; this is not evidence of reduced cost.
- The comparison is diagnostic and is not a separately preregistered holdout.

Comparator defects:

1. It prints the geometric-mean work ratio but never enforces geomean >= 1.0.
2. It flags a work regression only above 5%, while declared G5 requires a geometric mean of at least 1.0 and no individual regression above 5%.
3. It allows a 2x error regression, while declared G5 requires 1.5x.
4. It does not fail closed when cells are missing or extra.
5. It compares counters, not bitwise trajectory/output-array hashes, so it does not prove G1 bit identity.
6. Output records identify the baseline commit but do not bind the candidate source SHA and full run manifest strongly enough for independent replay.
7. It does not separate the B04/B05 holdout population from calibration/tuning.
8. The primary field is named work_model_v2_ir, but the latest WorkModel V2 calibration was invalidated/failed in L-0207/L-0211. It must not be treated as a calibrated production cost oracle.

Conclusion: classify this as DEFAULT_PATH_NEUTRALITY_SCOPED, not G5_PASS and not a speedup. The commit message alone does not override the gate implementation.

## 3. Mathematical and implementation audit

### 3.1 WP-04: DOPRI5 secant stiffness screen

Source: crates/rodas5p-integrators/src/dopri5_stiffness.rs, crates/rodas5p-integrators/src/dopri5.rs, crates/vigilode/src/switching.rs.

The scalar quantity
\[
\widehat{h\lambda}
= h\,\frac{\langle d,g\rangle}{\langle d,d\rangle},
\qquad
d=Y_7-Y_6,\quad g=K_7-K_6,
\]
recovers h lambda for a scalar linear test equation. For a nonlinear/vector system it is one directional secant quotient. It is not a spectral enclosure, a numerical-abscissa bound, or a pseudospectral bound. It can miss unexcited fast modes and can mix nonnormal and oscillatory effects.

The real-axis stability boundary is useful for a scalar observer, but it does not turn the secant quotient into a multidimensional stability certificate. L-0213 already provides a falsifying off-registry result. Keep the screen observer-only. No route switch may depend on it until a prospective, source-bound gate passes across the full regime matrix.

### 3.2 WP-06: JVP accuracy ladder

Source: crates/vigilode/src/fd.rs.

The 4-node Chebyshev directional derivative weights were independently recomputed from the derivative of the cubic Lagrange interpolant at zero. They match the source constants to floating-point precision:

- nodes: cos((2q+1) pi / 8);
- weights: approximately [-0.11208538229199136, 1.577161014949475, -1.577161014949475, 0.11208538229199126];
- moments: sum of weights approximately 0, first moment 1, second and third moments approximately 0.

The symmetric node set cancels the cubic-order derivative error term, giving truncation O(delta^4); balancing with roundoff O(u/delta) motivates delta proportional to u^(1/5). This supports the nominal O(u^(4/5)) scaling only for sufficiently smooth RHS and a valid local Taylor regime.

Remaining obligations:

- Test error scaling on smooth nonlinear, badly scaled, near-zero, and nonsmooth RHS families.
- Do not apply high-order stencils across RHS kinks or event discontinuities.
- Make the rung selector and error estimator source-owned. The enum/one helper is not a complete accuracy ladder by itself.
- Complex-step requires a genuinely holomorphic complex callback and its own trait/contract. A real callback plus a label is not sufficient.
- Preserve the R0 default and prove default bit identity before any selector can affect a solve.

### 3.3 WP-09: product Chebyshev defect is unsound

Source: crates/rodas5p-integrators/src/conditioned_polynomial.rs, especially generate_polynomial.

Current implementation forms
\[
q(z)=\prod_j T_{d_j}\!\left(\frac{z-c_j}{\rho_j}\right)
\]
but stores max_j eta_j, where
\[
\eta_j = \left|T_{d_j}\!\left(\frac{1-c_j}{\rho_j}\right)\right|^{-1}.
\]
This is not an upper bound on
\[
\sup_{z\in K}\left|q(z)/q(1)\right|
\]
for a union of intervals K. On one interval, another factor can be very large.

Concrete reproducible witness, both degrees equal to one:

- I1 = [-1000,-999], center -999.5, semi-width 0.5.
- I2 = [-1.1,-1], center -1.05, semi-width 0.05.
- q(1) = 2001 × 41 = 82041.
- At z=-1000, |q(z)| = 19979.
- Actual normalized value = 19979/82041 = 0.24352457917382772.
- Current proxy = max(1/2001,1/41) = 1/41 = 0.0243902439.
- Underestimate factor = 19979/2001 = 9.9845077461.

This is a certificate-soundness blocker. Unit tests showing each factor is bounded on its own interval do not cover the product over the union.

Required repair:

1. Compute the true product supremum over every connected spectral component.
2. For rational/exact coefficient paths, isolate every real critical point of q or q' on every interval using Sturm sequences; include endpoints and multiple-root cases.
3. For floating coefficients, use interval coefficients and outward evaluation/root isolation. Ordinary f64 derivative roots are estimates only.
4. Certify q(1) away from zero with an outward interval.
5. Include any skew quadratic factor in the actual global supremum.
6. Until all premises are closed, expose EstimateOnly, never Certified.

### 3.4 WP-09: Gershgorin conversion is not a certified spectral enclosure

The code forms row Gershgorin radii, then clamps a right endpoint at or beyond zero to -1e-8. For a one-dimensional matrix A=[1], the source radii produce a positive interval, but the clamp/rewrite can return a small strictly negative interval that excludes the eigenvalue +1. The resulting enum source still maps to CertificateStatus::Certified.

Additional problem: Gershgorin gives complex disks, not real intervals containing the full spectrum of an arbitrary nonsymmetric matrix. A real-part interval does not justify evaluating a polynomial over that interval for a general complex spectrum.

Required repair:

- Never clip a true enclosure. If the enclosure intersects the forbidden/stability region, return a typed refusal.
- Preserve complex disks/rectangles or restrict the theorem to a declared matrix class whose spectrum is rigorously real.
- For a symmetric negative-definite or H-self-adjoint domain, supply the corresponding proof object and outward bounds.
- Do not allow arbitrary caller-provided intervals plus source=GershgorinCertified to manufacture authority. Use a sealed provenance-bearing enclosure type.
- Test A=[1], A=[-1], matrices with complex conjugate eigenvalues, nonnormal triangular matrices, and discs crossing Re(z)=0.

### 3.5 WP-10: complex Leja phi action

Source: crates/rodas5p-core/src/leja_action.rs.

The harmonic-oscillator unit tests pass for the tested 2x2 skew matrix, and the warmed workspace test checks stable capacities. This is a useful scoped functional result, not a general certificate.

Main authority defect: ComplexLejaConfig exposes is_normal: bool and beta: f64. The evaluator returns Certified if is_normal is true, but it does not itself verify normality or that the full spectrum lies on the imaginary interval bounded by beta. A caller assertion cannot be the proof premise of a certificate.

Required repair:

- Replace the boolean with a sealed normality witness or an operator type whose contract proves normality.
- Bind beta to a verified spectral enclosure with source/epoch/hash identity.
- A Ritz estimate, guessed beta, or user boolean must return EstimateOnly.
- Include nonnormal matrices, spectra outside beta, near-defective operators, high-order phi functions, and large dimensions in adversarial tests.
- Keep this API disconnected from a production route until its total error proof is independently reviewed and the complete IVP work comparison passes.

### 3.6 WP-07: rank-k secant preconditioner

Source: crates/rodas5p-krylov/src/secant_preconditioner.rs, GMRES optional wrappers.

For a rank-k model J_hat = U V^T, the Woodbury identity gives
\[
(I-h\gamma U V^T)^{-1}
= I+h\gamma U(I-h\gamma V^TU)^{-1}V^T.
\]
The implementation follows this algebra with a small capacitance matrix and a right-preconditioner interface. The apply_into hot loop uses fixed stack arrays; its targeted zero-allocation test passed.

The implementation is not yet a production stage solver:

- S and Y are not yet bound to an accepted RODAS5P stage epoch/history at the native stage-solve site.
- The optional GMRES wrapper exists, but the original stage linear-solve route does not automatically consume this preconditioner.
- Rank selection, Gram/QR pivots, capacitance singular values, and the 1e-6 condition filter are ordinary floating-point estimates, not outward certificates.
- Build/setup allocations and factorization work must be charged.
- The preconditioned solve must always test the true residual in the original unpreconditioned operator coordinates.

Next evidence: synthetic exact low-rank matrices first, then source-bound stage data, then large sparse nonlinear full-IVP holdouts. Accept only if full work at matched achieved error improves, not merely if JVP counts fall.

### 3.7 WP-11: Hermite dense-output splice

Source: crates/vigilode/src/hermite_splice.rs, crates/vigilode/src/solution.rs.

Six focused integration tests pass, including endpoint preservation and derivative-continuity contracts. Two questions remain before production switching:

- Solution::at_into uses evaluate_into for the fast extension, but the sequential extension branch calls seg.evaluate(t) and then copies the returned vector. That allocates. The public zero-allocation claim is therefore not true for every extension mode.
- The splice object guarantees its own polynomial endpoint conditions, but production use still needs route-derived endpoint derivative mismatches. Arbitrary user-provided correction slopes do not prove that a real solver switch is C1.

Required repair: make the sequential dense segment support evaluate_into; test both extension modes with an allocation counter. Then add an end-to-end route-switch test where the left/right vector fields, endpoint values, endpoint slopes, dense outputs, and splice correction all come from the actual solver routes.

### 3.8 WP-14: preconditioned fixed-point map

Source: crates/vigilode-coupled/src/rhs_homotopy.rs.

The map T(x)=x-PF(x) is mathematically appropriate. The standard sufficient condition is: on a closed ball B(xc,r), prove sup ||I-P DF(x)|| <= q < 1 and ||P F(xc)|| <= (1-q)r. Then T maps the ball into itself and is a q-contraction.

The current QContractionCertificate::verify(q,r,p_residual) checks scalar inequalities on supplied numbers. It does not compute or prove the supremum of the Jacobian map, nor that the residual is an outward upper bound. Treat it as a checker for externally certified inputs, not a complete certificate.

The PreconditionedFixedPointMap heap-allocates a temporary vector in the dimension >64 path on each evaluation. Charge this in any total-work/allocation comparison; do not describe this path as allocation-free.

## 4. Dynamical-systems and symbolic-dynamics theory to introduce

### 4.1 Nonnormality: logarithmic norms and transient growth

For a frozen linearized system z'=Jz and a fixed SPD metric H, define
\[
A_H=H^{1/2}JH^{-1/2},\qquad
\mu_H(J)=\lambda_{\max}\left((A_H+A_H^T)/2\right).
\]
Then
\[
\|z(t)\|_H \le e^{t\mu_H(J)}\|z(0)\|_H.
\]
This bounds finite-time energy growth and is more informative than the spectral abscissa alone for nonnormal systems. For time-varying J(t), integrate the corresponding logarithmic-norm bound; if H varies, include its derivative contribution.

Wolfram control example:
\[
J=\begin{pmatrix}-1&10\\0&-2\end{pmatrix}.
\]
Eigenvalues are -1 and -2, but the Euclidean numerical abscissa is (-3+sqrt(101))/2 = 3.52493781056. The maximum 2-norm transient amplification over the checked interval t in [0,20] is approximately 2.5634928857. This is a finite example demonstrating why eigenvalues alone are not a sufficient nonnormal risk classifier.

A logarithmic-norm value from a finite probe is still an estimate. A certificate requires a source-owned metric, a rigorous upper bound on the symmetric part, and a trajectory-tube argument if the Jacobian varies.

### 4.2 Resolvent/pseudospectral bound

For a contour Gamma in the resolvent set enclosing the spectrum,
\[
\|e^{tJ}\|
\le \frac{\operatorname{length}(\Gamma)}{2\pi}
\sup_{z\in\Gamma}e^{t\operatorname{Re}z}\|(zI-J)^{-1}\|.
\]
This follows from the holomorphic functional calculus and the contour integral for the matrix exponential. It is a strong independent nonnormal audit, but a discrete sample of the contour does not bound the supremum. A usable certificate needs between-sample resolvent enclosures, linear-solve residual bounds, contour quadrature/geometry bounds, and a variation term for changing Jacobians.

### 4.3 Lyapunov and fast/slow structure

Explore source-bound Lyapunov metrics from A^T P + P A = -Q for Hurwitz frozen linearizations. This can produce a contraction metric when Q is positive definite. The cost and conditioning of constructing P must be charged. Do not assume a common P exists across all route regimes; switched systems need a common Lyapunov function or a rigorously bounded transition/multiple-Lyapunov argument.

Use invariant sets, fast/slow splitting, spectral gaps, one-sided Lipschitz constants, dissipativity, and transient growth as complementary regime descriptors. Stiffness is not a single scalar property.

### 4.4 Symbolic dynamics of the solver controller

Treat the route controller as a finite-state hybrid automaton. Its symbolic words encode route sequences such as explicit -> RODAS5P, RODAS5P -> matrix-free, or polynomial action -> protected fallback.

Required invariants:

- no illegal state/cause transition;
- no Zeno/chattering sequence;
- a minimum dwell or hysteresis condition whose entry/return guards preserve registered decisions;
- every rejected pilot and fallback cost is charged;
- the state and dense-output continuity condition is preserved at each route boundary;
- a route word accepted by the controller has a corresponding measured total-work/error record.

The failed H1 and B7 results must inform guard design. Do not tune a hysteresis band or first-batch length after observing the same data used to claim success.

### 4.5 Algebraic and numerical structure

Combine the following only when their premises fit:

- M-matrix and nonsingular M-matrix theory for sign-structured stage matrices; inverse positivity and Schur complement closure need their matrix-class assumptions.
- Graph Schur complements and elimination/fill estimates for sparse stage systems.
- CPR colouring for low-colour Jacobian assembly, currently the strongest scoped pass.
- GMRES/FGMRES and low-rank Woodbury preconditioning with true residual checks.
- Chebyshev/Sturm/interval methods for globally bounded polynomial defects.
- Leja phi actions for oscillatory normal/skew operators only after a verified normality and spectral enclosure contract.
- Exact rational algebra and directed interval arithmetic for small proof certificates; do not use floating roots as exact evidence.

## 5. CTSH 2.0.2 exploration record

Archive:
- Path: /home/cosmosapjw/vigilode/CTSH_Chat_Native_Theory_Search_Harness_v2.0.2.zip
- SHA-256: eddc8ad6e41e3a72e1e6a9cf1460c1d4de39c405752970d56b718cc4ad859121
- 57 members; archive manifest checksums passed.
- Python 3.12.3; Lean 4.34.1 and wolframscript passed environment probes.
- Run with PYTHONPATH=<extracted-archive>/src python3 -m ctsh; a global ctsh executable is not installed.

Validation:
- CTSH tests without Lean test environment variable: 54 passed, 1 skipped.
- CTSH tests with CTSH_TEST_LEAN_TOOLCHAIN=leanprover/lean4:v4.34.1: 55 passed.
- A safe Wolfram arithmetic identity for the quadratic-form derivative was SYMBOLICALLY_CHECKED. The candidate was deliberately not promoted because this checks only one encoded algebraic identity, not the entire natural-language claim.
- ULA/Langevin mutation was disabled.
- A locked production-solver goal was initialized with decision id 24e0f15afd76.
- Three candidates passed CTSH scope/novelty ingestion, all with status OPEN and code ACCEPTED_FOR_ORACLE_EVALUATION:
  - CTSH-VIG-LOGNORM-001
  - CTSH-VIG-RESOLVENT-002
  - CTSH-VIG-STURM-003
- These are not theorem-proved or formally verified. CTSH's ingestion gate is a schema/scope/novelty gate, not a theorem prover. No candidate has oracle promotion or BO feedback.

Workspace: /tmp/ctsh_vigilode_production_20261104. It is ephemeral to the current research host. Candidate JSON and goal/prompt hashes are recorded in the handoff below.

## 6. Production-readiness research DAG

All nodes are initially OPEN. The audit itself is complete, but no implementation node is accepted as closed.

| Node | Title | Depends on | Required output | Current status |
|---|---|---|---|---|
| R0 | Freeze provenance, repair comparator and baseline authority | none | source manifest, baseline/holdout split, corrected G1/G5 comparator, valid work accounting | PARTIAL |
| R1 | Repair spectral enclosure and Chebyshev defect soundness | R0 | sealed enclosure type, no clipping, product supremum certificate, Sturm/interval tests | OPEN, BLOCKER |
| R2 | Repair complex Leja certification | R0 | verified normality/spectrum witness, nonnormal/under-enclosed refusal | OPEN, BLOCKER |
| R3 | Nonnormal regime observer | R0 | logarithmic norm plus selected resolvent audits, all EST until certified | OPEN |
| R4 | Explicit/JVP observer calibration | R0 | verify DOPRI5 secant and Chebyshev4 accuracy limits across regimes; no switch by default | OPEN |
| R5 | Stage linear-solve candidates | R0, R1, R2 | CPR/GMRES/secant/graph-Schur/Leja comparison with true residuals and full costs | OPEN |
| R6 | Hybrid route automaton and dense-output switching | R3, R4, R5 | guard invariants, no Zeno, C1/Ck endpoint contract, all-path no-allocation tests | OPEN |
| R7 | Large sparse same-error holdout battery | R1..R6 | independent references, complete work and error records, raw data and hashes | OPEN |
| R8 | Independent referee and release qualification | R7 | full tests, clippy, provenance review, no open P0/P1, prospective production gate | OPEN |

### 6.1 R0 acceptance

R0 cannot close until:

- baseline and candidate outputs bind exact source/commit SHA, config SHA, fixture SHA, toolchain, environment, and command;
- comparator fails on missing/extra/duplicate cells and nonfinite values;
- G1 compares output trajectory arrays bitwise, not only counters;
- G5 executes the actual declared inequalities: geometric mean work ratio threshold, per-cell regression bound, achieved-error cap, holdout split, and all cost categories;
- WorkModel V2 remains excluded until its preregistered calibration and E1R failure are properly repaired and independently requalified;
- all pilot, rejection, allocation, setup, factorization, and fallback costs are charged.

### 6.2 R1 acceptance

- Positive-spectrum and right-half-plane Gershgorin controls must refuse certification.
- Complex spectra must be enclosed in the domain used by polynomial bounds.
- Product-polynomial defect is globally bounded over the full union, including every critical point.
- q(1) is outward-enclosed away from zero.
- Skew factors are included in the same defect proof.
- Exact/rational and floating/interval proof modes are separately identified.
- Re-run the two-interval 9.9845x counterexample as a regression.

### 6.3 R2 acceptance

- No caller boolean can create Certified status.
- Normality and the full spectrum inclusion are proof objects bound to operator identity/epoch.
- False or missing witnesses return EstimateOnly or typed refusal.
- Compare phi actions against independent high-precision and exact small-matrix functions across orders, dimensions, and adversarial spectra.
- Verify the claimed remainder enclosure with outward arithmetic.

### 6.4 R3 and R4 acceptance

- Include stable but nonnormal triangular systems, strongly transient systems, rotation, damped oscillation, stiff forced, Van der Pol, Brusselator, Robertson, and changing-Jacobian cases.
- Compare eigenvalue/secant, logarithmic norm, and resolvent indicators.
- No indicator causes a switch until a preregistered gate is passed on registered and off-registry holdouts.
- All observer modes preserve the exact default path and counters.

### 6.5 R5 acceptance

Compare at the same achieved global error:

- dense/direct RODAS5P;
- CPR assembled Jacobian + direct sparse/dense factorization;
- matrix-free GMRES/FGMRES;
- secant-Woodbury preconditioned GMRES;
- graph-Schur/path-forest;
- polynomial or Leja route only when its enclosure is valid.

Report:
- RHS evaluations;
- JVP vectors and calls;
- Jacobian builds;
- factorization/setup work;
- Krylov iterations and operator products;
- polynomial coefficient setups and actions;
- allocations/bytes and peak memory;
- rejected pilots, retries, discarded work;
- fallback count and cost;
- accepted/rejected steps and switches;
- achieved global error and dense-output error;
- wall time as a secondary diagnostic with repeated sessions and CPU affinity where relevant.

### 6.6 R6 acceptance

- Model the routing controller as an explicit finite-state automaton and test all reachable state/cause pairs.
- Hysteresis must satisfy entry/return decision preservation and no-chattering conditions on a separate holdout.
- Hermite splice endpoint values and derivatives must be derived from the actual adjacent numerical routes.
- All dense-output extension modes must satisfy the allocation contract.
- Verify event truncation, overlapping splice policy, and reentrant/concurrent evaluation.

### 6.7 R7 acceptance

Required holdouts not used for tuning:

- stiff nonlinear Robertson;
- oscillatory Prothero-Robinson/harmonic oscillator;
- nonnormal linear and nonlinear systems with transient amplification;
- changing-stiffness/reentrant systems;
- large sparse diffusion-reaction/Brusselator;
- a large sparse stiff nonnormal graph family.

Each case must have an independent high-accuracy reference, identical output grid, matched tolerances, exact source/input hashes, and a prospective stop rule. The comparison must include the protected original RODAS5P baseline.

### 6.8 R8 release gate

All proposed conditions are required:

- all theorem/certificate premises source-bound and independently reviewed;
- no Certified label on an empirical estimate;
- full workspace fmt, clippy with -D warnings, and test matrix at the exact release commit;
- all default-path bit-identity contracts pass;
- no unexplained test skip, timeout, ignored failure, or missing holdout;
- no missing work category;
- the preregistered same-error performance gate passes on calibration and holdout populations;
- at least one distinct blind referee pass and resolution of all critical findings;
- original RODAS5P direct fallback remains available and tested;
- only then may a separate explicit promotion proposal be considered.

## 7. Ordered next actions

1. R0: repair the comparison tool and provenance schema before generating any new speed claim.
2. R1: fix Gershgorin enclosure provenance and product Chebyshev defect. This blocks any Certified polynomial route.
3. R2: make Complex Leja certification depend on verified normality and spectral bounds.
4. R3: add a nonnormal observer using the weighted logarithmic norm; use contour-resolvent checks only on selected smaller audit cases until a scalable enclosure exists.
5. R4: keep WP-04 observer-only; validate WP-06 rung errors and smoothness/kink guards.
6. R5: first extend C1 coloured assembly to larger sparse and 2-D structures. Then connect secant data to actual stage history and compare against existing preconditioners. Revisit graph-Schur only on large sparse holdouts.
7. R6: complete route automaton and dense-output splice integration.
8. R7: run a preregistered same-error whole-IVP battery with independent high-accuracy references.
9. R8: independent blind referee, full CI, and production qualification.

No default or tolerance mutation is allowed during R0-R7. Any failed node remains in the ledger with its raw evidence and cannot be renamed PASS by adjusting thresholds after observing results.

## 8. Focused verification performed at HEAD 54297bb6

| Command/test | Result |
|---|---|
| cargo fmt --all -- --check | PASS |
| cargo test --locked --offline -j 1 -p rodas5p-core --test leja_total_contract | 20 passed |
| cargo test --locked --offline -j 1 -p rodas5p-integrators wp09_tests --lib | 5 passed |
| cargo test --locked --offline -j 1 -p rodas5p-krylov --test secant_preconditioner_contracts | 1 passed |
| cargo test --locked --offline -j 1 -p rodas5p-core --lib complex_leja | 3 passed |
| cargo test --locked --offline -j 1 -p vigilode --test hermite_splice_contracts | 6 passed |
| cargo test --locked --offline -j 1 -p vigilode-coupled --lib rhs_homotopy | 5 passed |
| Focused total | 40 passed, 0 failed |
| Full workspace/all-target verification at exact HEAD | NOT RUN to completion |
| Previous broad test attempt on an intermediate snapshot | SIGKILL after 324.08s; not a test verdict for HEAD 54297bb6 |
| Clippy at HEAD 54297bb6 | NOT VERIFIED |

Focused tests passing does not cancel the mathematical counterexamples in Section 3.

## 9. Known failure taxonomy

| Class | Current instance | Disposition |
|---|---|---|
| Mathematical/certificate error | WP-09 product defect and Gershgorin clipping; WP-10 caller-asserted normality/spectrum | P0 blocker for Certified use |
| Theory/model limitation | DOPRI5 secant not a global spectral bound; nonnormal transient growth | Observer only |
| Numerical-method accuracy | WP-06 high-order finite-difference smoothness and step selection | Scoped tests, wider error study required |
| Linear algebra/conditioning | Secant rank and capacitance tests use floating estimates | EST, true residual mandatory |
| Implementation/performance | Sequential dense-output at_into allocates; fixed-point map allocates above dimension 64 | Repair/charge before production |
| Work-model invalidity | E1 invalidated; E1R fails | V1/default retained, V2 not a primary oracle |
| Experiment design | G5 comparator does not enforce its stated gate | Comparator repair before rerun |
| Runtime/environment | Prior broad cargo attempt SIGKILL; current focused tests pass | Broad CI remains unverified |
| Governance/provenance | Branch 29 ahead, no PR, ledger stops at L-0214 | No production admission |

## 10. Literature and external technical anchors

- D. J. Higham and L. N. Trefethen, “Stiffness of ODEs”, BIT Numerical Mathematics 33 (1993), 285–303. DOI: https://doi.org/10.1007/BF01989751. The paper argues that eigenvalues alone do not characterize stiffness when transient behavior and pseudospectra matter.
- “Improved traditional Rosenbrock-Wanner methods for stiff ODEs and DAEs”, Journal of Computational and Applied Mathematics 286 (2015), 128–144. DOI: https://doi.org/10.1016/j.cam.2015.03.010. Relevant to ROW order conditions and order reduction on stiff ODE/DAE and semidiscretized parabolic problems.
- L. N. Trefethen and M. Embree, Spectra and Pseudospectra: The Behavior of Nonnormal Matrices and Operators, Princeton University Press, 2005.
- E. Hairer and G. Wanner, Solving Ordinary Differential Equations II: Stiff and Differential-Algebraic Problems, 2nd revised edition, Springer, 1996.
- Y. Saad, Iterative Methods for Sparse Linear Systems, 2nd edition, SIAM, 2003. Relevant to Krylov methods and preconditioning.
- S. Basu, R. Pollack, and M.-F. Roy, Algorithms in Real Algebraic Geometry, 2nd edition, Springer, 2006. Relevant to exact real-root isolation and Sturm methods.

These references motivate the theory paths. The exact RODAS5P tableau, order conditions, and implementation convention must still be checked against the exact method source and coefficient snapshot used by this repository.

## 11. Next-thread handoff

Copy this block as the next conversation's starting context.

~~~~text
PROJECT=cosmosapjw-quantum/vigilode
TRACK=PRODUCTION_SOLVER_AUDIT_AND_ROADMAP
READ_LIVE_HEAD_FIRST=true
CURRENT_BRANCH=research/switcher-v2-20261103
CURRENT_HEAD=54297bb65e3c1337e29fe98072fa4e95e32df94e
PARENT_HEAD=dc60d6f6bf372c1b866fc51a55cf912c9cd37378
REMOTE_BRANCH_HEAD=4339facc62258b5fb9a2469f16a3af8b93763e24
AHEAD_COMMITS=29
PR_FOR_BRANCH=NONE_FOUND
TRACKED_WORKTREE=CLEAN
PRODUCTION=HOLD
NO_MERGE=true
NO_DEFAULT_CHANGE=true
NO_TOLERANCE_RELAXATION=true
NO_RETROACTIVE_GATE_CHANGE=true

READ_FIRST:
- VIGILODE_PRODUCTION_SOLVER_AUDIT_ROADMAP_20261104.md
- docs/vigilode_original_mission_realign_20261101/REFOCUSED_DAG.json
- research/LEDGER.jsonl (latest row L-0214)
- tools/compare_opt_battery.py
- bench/out/opt_baseline_4339facc.json
- bench/out/opt_candidate_dc60d6f6.json
- crates/rodas5p-integrators/src/conditioned_polynomial.rs
- crates/rodas5p-core/src/leja_action.rs
- crates/rodas5p-krylov/src/secant_preconditioner.rs
- crates/vigilode/src/solution.rs
- crates/vigilode-coupled/src/rhs_homotopy.rs

CRITICAL_FINDINGS:
1. WP09 product Chebyshev defect proxy underestimates a two-interval true value by 9.9845x.
2. WP09 Gershgorin helper clips a positive eigenvalue into a negative interval and can label it Certified.
3. WP10 ComplexLejaConfig accepts caller-provided is_normal and beta without verifying normality or spectrum inclusion, yet may return Certified.
4. WP14 QContractionCertificate checks supplied scalars; it does not prove derivative/residual enclosures.
5. WP11 Solution::at_into sequential extension path allocates.
6. WP07 secant preconditioner is not connected to native RODAS5P stage solves; factor conditioning checks are estimates.
7. WP04 screen failed off-registry 1-D Brusselator agreement 0.5833 < 0.90; observer only.
8. WorkModel V2 row L-0206 invalidated by L-0207; E1R L-0211 FAIL.
9. G5 commit 54297bb6 claim is not substantiated: comparator skips stated G5 checks; 96/96 counters identical and W_base/W_candidate=1.0.
10. Latest official ledger row is L-0214; WP07/WP09/WP10/WP11/WP14/G5 have no official ledger admission rows.

CTSH:
- Archive SHA256=eddc8ad6e41e3a72e1e6a9cf1460c1d4de39c405752970d56b718cc4ad859121
- Workspace=/tmp/ctsh_vigilode_production_20261104
- decision_id=24e0f15afd76
- candidates=CTSH-VIG-LOGNORM-001, CTSH-VIG-RESOLVENT-002, CTSH-VIG-STURM-003
- all accepted for oracle evaluation but remain OPEN; no theorem promotion
- ULA disabled
- 55 CTSH tests passed when Lean 4.34.1 toolchain env was set
- Wolfram checked one encoded quadratic-form identity only

CURRENT_VERIFICATION_AT_HEAD:
- fmt PASS
- leja_total_contract 20/20
- wp09_tests 5/5
- secant_preconditioner_contracts 1/1
- complex_leja unit tests 3/3
- hermite_splice_contracts 6/6
- rhs_homotopy focused tests 5/5
- clippy not verified
- full workspace test not verified at this HEAD

NEXT_ACTION:
Start with R0 comparator/provenance repair and R1 certificate-soundness fixes. Do not wire candidate routes into production or change defaults. Every future repo task must reread live HEAD before editing. Use same achieved global error and full total work. Blind referee independently attacks all proof premises, code paths, and comparator gates.
~~~~

## 12. Stop conditions

Stop and keep production HOLD if any of the following occurs:

- any Certified label depends on a caller assertion or an unverified spectral enclosure;
- a product defect, residual, or contraction quantity is not a genuine outward bound;
- a true residual is replaced by a preconditioned residual in an acceptance decision;
- a candidate is compared at a different achieved global error;
- any setup, discarded pilot, rejected attempt, allocation, or fallback cost is uncharged;
- default output/counters are not bit-identical to the protected baseline;
- the comparator cannot reproduce its own declared gates;
- the branch/commit/test snapshot changes during the evidence run without being captured in the manifest.

Final decision for this snapshot: retain the original RODAS5P solver as protected production baseline. Continue the nine-node research DAG from R0. No default switch, merge, tolerance relaxation, or publication is authorized by this audit.
