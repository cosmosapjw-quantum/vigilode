# VigilODE R8 preregistered science/selection unit, 2026-11-01 KST

Parent Draft PR #182 and immutable HEAD 8fd2ac31d65df5e70e3bbb266d51b397562edfb5. **R8 only:** decide which exact mathematical claims and solver routing candidates to admit, reusing R5-R7; stage-local source-based algebraic experiment, one hostile review, optional OpenEvolve-MCTS four-wave exploration. No new generic solver production implementation or core changes.

Original mission G0: protect eight-stage RODAS5P matrix-free JVP method, explore lower causal critical depth and Newton-free/nonlinear homotopy, count total error/work including rejected candidates and protected fallback. Frozen official DAG 7/23, successor 0/12, production HOLD.

## Original immutable inputs

- Original raw-U source blob d68239d53977e36f98b186b627aa0c4c5c7de7cb.
- Original autonomous cooperative cubic RHS blob d78c5c5f6f476f2a5b1ea88628ed25daa6cfc482 and WP1 fixture 1bfc17ec2ed4b902cc06073807388f6e332e31d6.
- Original RODAS tableau SHA256 211255f6125b648c6955833c3da537d9c031ec78168b2918563c29fa88c8bce6.
- R6 prior source-locked eight-stage exact-rational residual and independent 90-digit first-step reference; R7 actual complete native 24/24 IVPs/three independent models and median matched-accuracy protected/transactional wall ratio 0.21069269165444293.
- Original isolated tree-search design ZIP: /home/cosmosapjw/Dropbox/tree_search_research_preimplementation_final_v1_0.zip, SHA256 bf78183ea353979fe6bf009e725fa28a7228ed55ca4593f3659eb2add26f03f3, design state-machine not a learned MCTS implementation. Genuine offline OpenEvolve ProgramDatabase 0.4.0 from previous WP9B environment, no external LLM.

## New mathematical candidate/negative controls

For admissible source coefficients k_i>0,g_i>=0,c_ij>=0, tau=h gamma>0, W=I-tau Df(y_n)=D(I-T) with diagonal d_i=1+tau(k_i+s_i+3g_i*y_i^2), and off-diagonal T_ij=tau c_ij/d_i>=0, T_ii=0. q=max_i sum_j T_ij <1.

Hypothesis A (derive algebraically): W is an invertible M-matrix, u=(I-T)^-1 D^-1 r, Neumann truncation u_m=sum_{j=0}^m T^j D^-1r has real error <=q^(m+1)/(1-q)*||D^-1 r||_infty; computed original residual gives sharper ||u-u_m||_infty <= ||Wu_m-r||_infty / min_i (1+tau(k_i+3g_i*y_i^2)). Explicit 2x2/3x3 finite Cayley-Hamilton/adjugate W^-1 with strictly positive determinant supplies algebraic alternative. This uses source-owned derivative structure, NOT unknown-RHS generic derivative-free method.

Hypothesis B (experimental): as source-specific first original raw-U stage provider, algebraic inverse avoids callback-JVP/GMRES in 2/3-dimensional cubic models, exact represented-rational residual matches original stage with zero mathematical defect. Compare to native protected source-locked stage reconstruction; work proxy != native wall speed. Do NOT claim full-native eight-stage bit parity or production speedup.

Mandatory nontrivial branching: signed non-Metzler log-norm resolvent boundary and/or algebraic-geometric Schur/Cayley inverse, with counterexamples to extension beyond admitted coefficients. Mandatory hybrid branching: original RODAS5P + algebraic-homotopy/interval residual, or fused exponential + Bernstein defect. At least one of each per search wave. Prior native work precision feeds candidate reward and next OpenEvolve mutation/UCT prior; Bayesian acquisition controls switch. Keep actual ProgramDatabase class and distinct logical status PROVED/FINITE_CHECKED/CONJECTURAL/REFUTED. Four search waves only, fixed per-op quotas 2+2, no claim statistical BO convergence.

Tested original source cohort set: asymmetric_2, asymmetric_2_refined resolution control, coupled_3_cubic, linear_exchange_control; first h=0.1/64 or 0.1/128 exactly as original; eight U stages source-conditional (4x8). Optional h half-step check ONLY when source owner and input epoch unchanged. Negative controls g<0, c<0, non-Metzler signed source, size>3, source identity swap.

Stop conditions: wrong original raw-U target, 8-stage error bound mismatch, nonexistent full IVP overlap, false proof status, no G0 relevance. At R8 close decide exactly: keep native protected as default for this size, hold default native q2, admit only candidate source-owned algebraic solver for future opt-in native integration if verified. Do not mark canonical node done or merge/change default/tolerance.

All committed changes isolated to a new research directory and append-only ledger if local checker allows. Write reproducible data, tests, actual repo source hashes, report and next-thread handoff. At each phase push a checkpoint and create authenticated standalone backup of changed artifacts; no inferred remote backup without readback.
