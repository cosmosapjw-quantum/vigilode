# VigilODE derivative-free N01 C7 final handoff (2026-10-30 KST)

$physmath-research-loop $scispace $exa $wolfram

PROJECT=cosmosapjw-quantum/vigilode
RESUME_DRAFT_PR=123
PR_URL=https://github.com/cosmosapjw-quantum/vigilode/pull/123
BASE_PR=120
BASE_HEAD=f022c4c5593b8d3f1f4ee7ba3efa56fd800dd413
VERIFIED_PUBLISHED_HEAD=0a32803ac2ed83f62b71f62add84cfa70e997f6a
WORK_BRANCH=research/n01-df-c7-raw-u-physics-20261030
FINISHED_STAGE=C7_SCIENTIFIC_SCOPED_NATIVE_PASS
NEXT_STAGE=C8_NUMERICAL_STABILITY_ACCURACY_AND_PERFORMANCE
AUTO_NEXT_NODE=false
PRODUCTION=HOLD

DAG_VIGILODE=7/23
DAG_REI_CENTRAL=28/88
SUCCESSOR_ADMITTED=0/12
N01_C6=1/1_NATIVE_SCOPED
N01_C7=1/1_SCIENTIFIC_SCOPED_NATIVE_PASS
N01_GENERAL_PRODUCTION=HOLD

ARCHIVE=VIGILODE_N01_DF_C7_20261030_EVIDENCE.zip
ZIP_BYTES=150430
ZIP_SHA256=efdbb3581479524b0d3d888b759137b0514b1639ddb5e8313131d7467fb0474a
ZIP_ENTRIES=18
ZIP_PAYLOADS=17
DRIVE_ZIP_ID=1I2u7SShIemd37vUkW78T37C9medLROSF
DRIVE_RAW_READBACK=BYTE_IDENTICAL_SHA256_CRC_PASS
DROPBOX_PAPER_ID=id:BSpOijBcT10AAAAAAEHV8Q
DROPBOX_TEXT_BACKUP=ACK_REVISION_2
DROPBOX_BINARY_BACKUP=NOT_RUN

METHOD_PRIORITY:
Causal, parallelized, RHS-value-only derivative-free Rosenbrock/RK
with Newton-free accelerated homotopy; JVP optional secondary fallback;
dense original Jacobian only a small independent oracle.

SOURCE_IDENTITY:
Native test Git blob 1156ea9e67562b8177cebbead92635e7e8a697a6
Original RODAS5P coefficient snapshot SHA256
211255f6125b648c6955833c3da537d9c031ec78168b2918563c29fa88c8bce6
Successful final Green GitHub Actions run 40914675878
Tested code HEAD b80617f6eb0d38d42dbf8ee5552b3d94b5a0a785
Job 123591615978 SUCCESS; later reporting commits do not modify native source.

C7_SCIENCE:
1. Exactly solved BE scalar auxiliary stage must not be assumed a RODAS5P raw U0 solution.
2. For Robertson f and BE displacement d=tau f(y+d), original U0 defect is
   tau*(k2*db*dc, -k2*db*dc-k3*db^2, k3*db^2).
3. Exact manufactured witness: BE residual=0 but original U0 residual
   (5/65536, -35/131072, 25/131072), max=35/131072.
4. Exact Fraction oracle: 256 cases, 1799 assertions PASS.
5. Fixed endpoint with y(tau)=z-tau f(z): exact cubic tau^3 scaling,
   ten dyadic checks PASS, doubling tau yields factor eight.
6. Actual n3 original 8-stage RODAS direct comparator: max raw-U
   residual 7.69783542464708148e-18; stage mass defect
   2.18026280623256907e-18; endpoint mass defect recorded0;
   raw-U/native-K endpoint difference 2.77555756156289135e-17.
7. Native focused CI: C7 3/3 + C6 6/6 + PR91 29/29 =38/38 PASS,
   strict Clippy, focused rustfmt, exact Python replay PASS.
8. Preserve RED run40913125750 (test-only Clippy 5 cases) and
   RED run40913805580 (relative Python path); repaired without model change.

NEXT_STAGE_C8_ONLY:
A. Read PR91 RHS-only homotopy/causal source and current PR123.
B. Use BE scalar root only as candidate for original U0 after original-target residual screen.
C. Construct source-owned original eight-stage outward residual and stage-error
   bounds, including representation, tableau raw/U/K discrepancy, time/epoch,
   allowed stage-domain tube, RHS/JVP application errors and rounding.
D. Prove bounded resolvent and nonnegative causal triangular transfer
   (I+L+...+L^7), record actual physical output bound.
E. Evaluate matched achieved-error full-IVP and stiff/nonnormal/negative
   controls; include all failed callbacks, rejected speculative work, and
   external reference uncertainty. Derivative-free RHS-only remains primary;
   use JVP/Krylov only opt-in and record all work.
F. Independent-author R6/C9, original source BASS physical33, whole-slab
   CMB/chemical integration, product release still NOT_RUN.
G. Post isolated Draft PR with real GitHub/Drive and permitted Dropbox
   backup acknowledgements. Never merge main, force push, weaken tolerances,
   silently change defaults, or claim full solver production from scoped C7.

RETURN:
Actual source and archive identities; exact native commands/jobs, mathematical
bounds, measurement/negative controls, original 7/23 REI28/88 successor0/12
DAG and scoped progress, provider receipts, and next executable handoff.
