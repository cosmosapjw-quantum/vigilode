# BASS21 source-owned Thomson eigenmode / shear-memory | next-thread handoff

## Restore anchor

PROJECT=cosmosapjw-quantum/vigilode
PARENT_PR=141
PARENT_HEAD=f14f29d4356f1723ce3efaf8ed15ad06bb955b1a
PREREG_COMMIT=ea8f5f6946614f90b414d187cbbdb5ce809179be
BRANCH=research/bass21-thomson-eigenmode-shear-memory-20261030
OWNER_REPO=cosmosapjw-quantum/bass
OWNER_COMMIT=cc602e209e7deeb511ab2a1f3b2268d55206c43c
OWNER_SOURCE_SHA256=6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8
PARENT_SOURCE_ZIP_SHA256=ce2b3507c30f5429fb93244bd7f3de7426f1f158bf52962e24734b6d1237dfe2
CURRENT_UNIT=ORIGINAL_BASS21_THOMSON_EIGENMODE_SHEAR_MEMORY
SOURCE_50=50/50
SAVED_EPOCHS=193
NEW_IVPS=0
NEW_COUPLED_OWNER_E_L_IVP=NOT_RUN
R6_DISTINCT_AUTHOR=NOT_RUN
C9_DISTINCT_AUTHOR=NOT_RUN
NATIVE_SUBAGENT=NOT_RUN
VIGILODE_DAG=7/23
REI_CENTRAL_DAG=28/88
REI_WORKING_DAG=33/88
BASS_CE1_DAG=9/15
SUCCESSOR_ADMITTED=0/12
OFFICIAL_DAG_DELTA=0
PRODUCTION=HOLD
NO_MORE_IVPS=true
NO_MAIN_MERGE=true
NO_FORCE_PUSH=true
NO_PRODUCTION_PROMOTION=true
AUTO_NEXT_NODE=false
NEXT_CANDIDATE_ONLY=SOURCE_NATIVE_SPIN_PSTF_Jl_El_FULL_HIERARCHY

## New exact result and actual source physics

For full original J2 and B_Q, `U=8J2+B_Q` and `V=4J2−3B_Q`. Collision exact `C_U=−gU`, `C_V=−3gV/10`; slow mode is physical coherency anisotropy `Mparallel−Mperp=−πV/2`. Exact source-specific shear `S_J2=bprime(-8J0/5−10J2/7+36J4/35)`, `S_BQ=bprime(10BQ/7+32c4/7)`; new J4 total/deltaJ4 reconstructed from immutable original source. Causal collision-only shear-memory `H_U=exp(−τ)∫exp(τ)S_U dx`, `H_V=exp(−3τ/10)∫exp(3τ/10)S_V dx`. The latter are prescribed-source Green diagnostics, **not** newly integrated native J/E IVPs.

50 original 193-epoch source cases generated 19,300 U/V mode entries, 19,300 shear-source entries, 19,300 causal-memory entries, 19,300 J4 total/delta entries, plus model-defect/time diagnostics and 25 matched pair endpoints. Full source, saved arrays and runnable code inside final ZIP; Git text mirror is not a binary substitute.

Observed max |H_U|≈1.8326510975, max |H_V|≈0.9554962441; mean paired ΔU≈−1.700933, ΔV≈−0.887352. Full casewise matched **initial** differences are nonzero; use ΔP initial transmitted term before attributing to shear. Max exact collision eigengap <3e−17, tensor tie <4e−15, GL64/96 J4 <2e−15, full saved-grid Duhamel defect U≈1.07898e−4, V≈5.64163e−5, Simpson/trapezoid gap up to 4.472e−5. The latter do not certify source BDF global error.

## Verification and publications

Run `OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 bash VERIFY.sh` in fresh extracted source ZIP to check exact immutable owner 50/50, full manifest, actual 10/10 tests and 6/6 output raw replay. On GitHub re-read the **live PR/head and scoped/global CI** before claiming present PASS. Source ZIP SHA/bytes and Drive/Library object IDs are bound in an external final receipt rather than written into self-referential source manifest. Dropbox requires a true provider ACK and has not been promised.

Required evidence states: `SOURCE_OWNED_PHYSICS_SCOPED`, `GLOBAL_AUTHORITY_CI=HISTORICAL_FAIL_UNLESS_VERIFIED`, `NO_OFFICIAL_DAG_DELTA`, `R6/C9=NOT_RUN`. Next science: owner-native spin PSTF E_l transport and spectral/observer closure before any new IVP; no automatic start.
