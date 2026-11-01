# VigilODE R6 / M2 original eight-stage causal-frontier preregistration

Date: 2026-11-01 KST. Selected theory stage R6, with stage-local R7 numerical counterexample check. Parent R5 Draft PR #177 immutable HEAD 7af490762873387c992edfbbbd66d0f8e6198607. This contract is committed and pushed BEFORE running the new q-frontier source experiment.

Original mission: source-preserving eight-stage raw-U RODAS5P, Newton-free causal critical-depth reductions, source-owned homotopy/algebraic alternatives when cheaper, protected R-JF fallback otherwise. Distinguish source-conditioned linear Rosenbrock stage from nonlinear BE/RK root. Only standalone VigilODE, no applied-project branch.

Inputs:
- Original eight-stage native raw-U semantics: crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs Git blob d68239d53977e36f98b186b627aa0c4c5c7de7cb
- Native frozen RODAS5P coefficient snapshot fixtures/rodas5p_coefficients_snapshot.json SHA256 211255f6125b648c6955833c3da537d9c031ec78168b2918563c29fa88c8bce6, original h gamma, A, C, b_code.
- Original cooperative cubic RHS source crates/vigilode-coupled/src/coupled_inverse_be.rs Git blob d78c5c5f6f476f2a5b1ea88628ed25daa6cfc482.
- Original case fixture crates/vigilode-coupled/examples/coupled_achieved_wp.rs Git blob 1bfc17ec2ed4b902cc06073807388f6e332e31d6, with original four labels: asymmetric_2, asymmetric_2_refined (only resolution control, same source), coupled_3_cubic and linear_exchange_control.
- Original horizon=0.1, first h=0.1/steps using steps 64,128,128,128. Start t=0 at original frozen y0.
- Original autonomous raw-U formula W U_i = tau*f(y+sum_(j<i) Aij U_j) + gamma sum_(j<i) Cij U_j, W=I-tau J(y), tau=float(h*gamma). All calculations source-conditional, no mass/nonautonomous f_t.
- Strict posterior certification: exact Fraction residual of binary64 represented constants/source inputs and actual proposed stage vectors; source-specific M-matrix inverse row-sum bound plus a finite triangular *a posteriori* nonlinear stage-error recursion with exact-real arithmetic and explicitly outward-CEIL output. No post hoc relaxation.

Preselected sweep policies:
- protected serial baseline: 8 sequential true source-owned linear solves of original W, causal span 8.
- q0=one simultaneous sweep from zero U (all 8 solves); q1=two sweeps, q2=three; q3..q7=4..8 sweeps, preserving already-correct prefix and charging all solved future stages, so upper work counts 8,15,21,26,30,33,35,36.
- independent high-precision mpmath protected sequential source stage reference with 100 decimal digits.
- analytic source-owned Jacobi diagonal inner W inverse: no runtime JVP/AD/FD/Newton callback; cost counted separately, tested against direct W solve; this is a source-specialized linear inverse, NOT generic derivative-free unknown-RHS solver.
- original stage residual exact on candidate; output weighted error upper sum_i |b_code_i| e_i; no physical IVP or native speed gate unless explicitly measured.

Adversarial cases:
- q2 may FAIL the original stage/output error despite reduced causal span; preserve as negative. Non-normal/sign-structured RODAS coefficients A/C can amplify e_i even if source W is M-matrix. No general global contraction assumption for the stage map.
- An 8-stage block iteration is exact in 8 passes in exact arithmetic only if each W-solve is exact and the source callbacks/times/epoch remain fixed. Floating point results are approximate; do not promote exact equality or order-five from the finite dependency theorem.
- Source original R5 nonlinear BE homotopy can be slower; keep native protected fallback and record it as part of total candidate work, never silently lower tolerance.
- Distinguish expression-level exact-real polynomial source from native Rust callback rounding and step clock.

Success evidence:
  4 frozen case rows; 8 sweep-depth rows each; at least original q2 and q7; 8-stage original stage residual max, finite a-posteriori output bound, high-precision protected source reference and work/span tradeoff, negative controls; exact source identity lock and deterministic replay tests.
  Failures preserved without claim of matched-achieved-error whole-IVP or production speedup.

Authority CI parallel:
  Check current PR177 strict CI; locate exact historical missing objects in local Git, backups and GitHub PR synthetic merge refs. Publish only exact original recoverable commit objects to archive refs. Never change checker, allowlist, reference relation, historical identities, main, default or release gate.

Frozen official DAG 7/23, successor admitted 0/12, production HOLD. This R6 result is a stage-scoped theory/verification node only and does not close a pre-existing canonical 23-node release dependency. No automatic next node, merge, or tolerance relaxation.
