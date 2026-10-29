# C9 source-binding adversarial audit → distinct-author independent reviewer

$physmath-research-loop $scispace $exa $wolfram

PROJECT=cosmosapjw-quantum/vigilode
RESUME_DRAFT_PR=126
PR_URL=https://github.com/cosmosapjw-quantum/vigilode/pull/126
PARENT_PR=125
PARENT_HEAD=8afb3cef7f237b0c7ac26ec7b647694df23a7789
TESTED_HEAD=b446c8c744554a7aa8394b1f4e05aea215e76c51
TESTED_GITHUB_ACTIONS_RUN=40973472003
TESTED_GITHUB_JOB=123795691691
C9_NATIVE_ADD_ONLY_SOURCE_BLOB=b6a0f06aba6d5d48d0de056ec9eadde1467904af
C8_ORIGINAL_TEST_BLOB=e6a33b66ba80055cae9439cf683a1715ca77c5a2
NEXT_STAGE=C9_DISTINCT_AUTHOR_INDEPENDENT_REVIEW
AFTER_THAT_ONLY=C10_IF_INDEPENDENT_ACCEPTED
AUTO_NEXT_NODE=false
PRODUCTION=HOLD

OFFICIAL_VIGILODE_DAG=7/23
CENTRAL_REI_DAG=28/88
SUCCESSOR_ADMITTED=0/12
N01_C6=1/1_NATIVE_SCOPED
N01_C7=1/1_SCIENTIFIC_SCOPED
N01_C8=1/1_NUMERICAL_SCOPED
C9_ADVERSARIAL_INTERNAL_TECHNICAL=1/1_SCOPED
C9_DISTINCT_AUTHOR_INDEPENDENT=0/1_NOT_RUN

PRIMARY_ARCHITECTURE=CAUSAL_PARALLEL_RHS_ONLY_DERIVATIVE_FREE_NEWTON_FREE_HOMOTOPY
JVP_ONLY=OPTIONAL_FALLBACK
DENSE_J=SMALL_REFERENCE_ORACLE_ONLY

C9_RESULTS:
- Native C8 2 tests plus C9 forged-source negative 1 test, 3/3 PASS; exact Python Fraction source binding 512 J norm / 504 valid Neumann / 128 strictly lower causal transfer PASS; strict Clippy/fmt PASS.
- F01: original C8 test-only certified_raw_stage_bound(case.k,y,h,stages,y_out) has no opaque owner/trial binding. It can certify the wrong equation. Exact fake k=0 residual0; true k=(1/2,10,200) at y=(3/4,1/8,1/8), tau1/64 first-stage residual (7/2048,93/2048,-25/512), max25/512. Actual Rust h=.005 yields fake0 true3.3115244249e-3. BLOCK promotion until source rates/clock/epoch/trial checked.
- F02: Moderate RODAS n8 achieved global IVP error versus independent mpmath75 ~9.63953e-10, maximum one-step algebraic bound 3.30584e-12, ratio291.5908; different mathematical objects, no contradiction. BLOCK using local bound as global IVP gate.
- F03: No overlap of achieved errors between RODAS and BE on studied mild/moderate grids. No equal-achieved-accuracy cross-family work ratio. Keep within-BE same-grid map-call comparison.
- F04: mpmath 40/75-digit mild/moderate reference stable to ~1e-35/1e-34; Radau ~7e-17/2.3e-16 difference, BDF ~2.46e-13/1.73e-12 difference. Numerical cross-check, NOT rigorous interval IVP theorem.
- F05: Conditional Neumann + finite causal math valid in 504/128 exact cases, no unconditional mass-contractivity.
- F06: test-only unwrap panic risk BACKLOG, not reproduced for valid physics.

IMPORTANT_C9_INDEPENDENCE:
- The same assistant wrote this bounded adversarial review. No native distinct-author delegation was exposed, so formal independent C9 0/1 NOT_RUN. SciSpace/Exa references and Wolfram are auxiliary computational/literature sources, not independent reviewer identities.

NEXT_ACTION:
1. Independently inspect PR125 immutable original code & source manifest and PR126 modified test, preserving original snapshot.
2. Independently reproduce F01 exact fake/true RHS and actual Rust test with different physical source; recommend exact owner/trial/epoch/clock binding API, not arbitrary user-supplied delta bounds.
3. Independently challenge eight-stage Jacobian 1-norm/tube and coefficient K/U outwards, prove true-source binding, and verify a posteriori *existence*, not just residual size. Check all work/fail and event-time uncertainties.
4. Re-evaluate RODAS-vs-BE matched achieved error with comparable cost accounting only when tested ranges overlap.
5. Explicitly report independent reviewer identity, source SHA, actual command/exit, falsifiers or confirmed claims, and positive next action. Then C10 can decide scope. No default routing changes, main merge, force push, or tolerance relaxation.

RETURN=INDEPENDENT_REVIEWER_PROVENANCE_AND_ACTUAL_EVIDENCE, source identity, findings, corrected claims, PR/commit, current DAG, cloud receipts, next handoff.
