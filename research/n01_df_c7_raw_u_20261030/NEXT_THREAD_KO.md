# VigilODE N01 C7 → 다음 C8 인계

$physmath-research-loop $scispace $exa $wolfram

PROJECT=cosmosapjw-quantum/vigilode
CURRENT_STAGE=C7_SCIENTIFIC_VALIDATION
NEXT_STAGE=C8_BOUNDED_NUMERICAL_VALIDATION
BASE_PR=120
C7_WORKFLOW_RUN=40914675878
C7_NATIVE_TEST_BLOB=1156ea9e67562b8177cebbead92635e7e8a697a6
WORK_BRANCH=research/n01-df-c7-raw-u-physics-20261030
AUTO_NEXT_NODE=false
PRODUCTION=HOLD

DAG_VIGILODE=7/23
DAG_REI_CENTRAL=28/88
SUCCESSOR_ADMITTED=0/12
N01_C6=1/1_NATIVE_SCOPED
N01_C7=1/1_SCIENTIFIC_SCOPED_NATIVE_PASS

MOTTO=causal_parallelized_derivative_free_first_Rosenbrock_RK_homotopy_Newton_free
JVP=CONDITIONAL_FALLBACK_ONLY
DENSE_J=INDEPENDENT_SMALL_REFERENCE_ONLY

READ_FIRST:
- PR91 derivative-free value-only homotopy and causal structure
- PR117 R5 exact nonlinear scalar-stage theorem
- PR119 C5 actual source-bound value-only N01 bridge
- PR120 C6 native 35/35 scoped
- current research/n01_df_c7_raw_u_20261030/* on new Draft PR

C7_PROVED:
- RODAS5P raw-U0 residual of exact BE displacement equals tau times exact quadratic Taylor remainder.
- Specific dyadic r_U0=(5/65536,-35/131072,25/131072) while BE residual exactly zero.
- Under fixed manufactured endpoint z, r_U0(tau) ∝ tau^3, 10 exact cases, doubling factor8.
- Exact Robertson mass: 1^T f=0, 1^T J=0, hence exact original raw-U stage and endpoint mass neutrality.
- 256 exact rational cases reject false BE→raw-U admission.

C7_NATIVELY_TESTED:
- 3 test-only scientific source-contract tests with existing 8-stage RODAS5P direct small oracle.
- C6 6 tests and PR91 RHS-only 29 regressions preserved.
- Test-only dense 3x3 reference not in main solver route.
- Final focused C7 GitHub run ID=40914675878, HEAD=b80617f6eb0d38d42dbf8ee5552b3d94b5a0a785, job123591615978: native C7 3/3, C6 6/6, PR91 causal/homotopy 29/29, strict Clippy+rustfmt PASS; independent Fraction 256 cases/1799 assertions, tau^3 10 cases PASS. Source Git blob 1156ea9e67562b8177cebbead92635e7e8a697a6. Full-IVP and BASS physical33 still NOT_RUN.

C8_OBJECTIVE:
1. Use BE value-only root as optional proposed first U-stage and pre-screen its exact source-specific quadratic defect. Do not directly admit.
2. Derive/report actual 8-stage verified source-owned residual bounds with clock/epoch, interval arithmetic, normalizations, frozen coefficient allowances.
3. Check invariant tube for all intermediate stage states, prove/source-certify resolvent and Lipschitz bounds; combine strict causal finite path sum (I+L+...+L^7) into output stage-error budget.
4. Make genuinely derivative-free RHS-only candidate primary; JVP/Krylov fallback opt-in after source/physics gate.
5. Run matched-accuracy native whole-IVP and adversarial stiff/nonnormal/negative controls, keep all failures and partial work.
6. Separate physics result, code correctness, scope and timing. No main merge/force push or tolerance relaxation.
7. Publish per-stage GitHub checkpoint, Google Drive and Dropbox with real ACK and source hashes; don't assert binary Dropbox if only Paper text exists.

RETURN:
actual PR/head, source/coeff SHA, commands and CI job IDs, exact oracle receipts, DAG official and scoped, backup provider ACK, next handoff.
