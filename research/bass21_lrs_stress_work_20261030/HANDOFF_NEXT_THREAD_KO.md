# VigilODE × BASS21 PR133 | 2026-10-30 source-owned radiation stress handoff

$physmath-research-loop $scispace $exa $wolfram

```makefile
PROJECT=cosmosapjw-quantum/vigilode
CURRENT_PR=133
CURRENT_PR_URL=https://github.com/cosmosapjw-quantum/vigilode/pull/133
PARENT_PR=132
IMMUTABLE_PARENT_HEAD=f97b0df6e13e09a67db3050a0e8e925e2e009022
BRANCH=research/bass21-lrs-source-owned-stress-work-20261030
PREREG_COMMIT=3eb9bd535e3aaaf6b63b4048ced60a8c8ba91342
SCIENCE_SOURCE_HEAD=7298ea4c08bdb4842d03b0f7146f8139fd428372
READ_LIVE_HEAD_FIRST=true
SOURCE_OWNER=cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c
BASS_LRS_LIOUVILLE_GIT_BLOB=e15e1875fba7fcc4a42e257e291c54d9b540a3f0
ORIGINAL_BASS_FULL50_SHA256=6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8
PARENT_PR132_ZIP_SHA256=ff17ce0c5883de8363898e83ebcb8b30322d35402d0cdbe4c678437493f6103a
UNIT=BASS21_LRS_SOURCE_OWNED_RADIATION_STRESS_WORK_20261030
CASES=50
EPOCHS=193
EVEN_L=0,2,4,6,8,10
NEW_JL_ROWS=57900
PAIRED_JL_ROWS=28950
STRESS_WORK_ROWS=9650
NEW_IVPS=0
VIGILODE_DAG=7/23
REI_CENTRAL_DAG=28/88
REI_WORKING_DAG=33/88
BASS_CE1_DAG=9/15
SUCCESSOR_DAG=0/12
OFFICIAL_DAG_DELTA=0
AUXILIARY=1/1_SCOPED_MODEL_PHYSICAL_HOLD
NATIVE_SUBAGENT=NOT_RUN
R6_DISTINCT_AUTHOR=NOT_RUN
C9_DISTINCT_AUTHOR=NOT_RUN
PRODUCTION=HOLD
NO_MAIN_MERGE=true
NO_FORCE_PUSH=true
NO_MORE_IVPS=true
NO_PRODUCTION_PROMOTION=true
NO_TOLERANCE_WEAKENING=true
AUTO_NEXT_NODE=false
```

## Actual new physical result

50 original same-source full62 BASS21 histories are byte-authenticated. Source-conditional PR132 coupled Thomson `deltaI,Q` response re-evaluated at GL48 internal and GL64/96 observer angles; six even Legendre corrections δJ_l and diagonal stress corrections generated. No original chemistry/geometry or polarization IVP. `x=ln(a/a_ref)`, metric (-,+,+,+), cold normal-comoving electrons, source photon F=a⁴I/(a_ref⁴I_ref). The physical perturbation is `delta T^a_b = I_ref(a_ref/a)^4 diag(-δJ0,(δJ0−δJ2)/3,(δJ0−δJ2)/3,(δJ0+2δJ2)/3)`; Q is not a separate energy reservoir.

Exact continuum identity `dδJ0/dx = −2 b'δJ2`, collision monopole 0; quadrupole `dδJ2/dx=b'(-8/5 δJ0−10/7 δJ2+36/35 δJ4)−g(9/10 δJ2+3/40 B_Q)`. At final saved x=z2, mean δJ0 +4.0251654634559156e-7, mean δJ2 −1.2351230138784083e-5; paired means ΔδJ0 +8.065507552213502e-7, ΔδJ2 −2.4696334720767026e-5, all 25 pair signs verified; paired shear work integral +8.065818082026581e-7. Maximum saved-grid relative work defect 5.7659799220187526e-5. GL64/96 max moment difference 1.9893362859655294e-19, tensor difference 7.623296525288703e-21, trace 3.3881317890172014e-21. Differentiated quadrupole budget max 2.7466652386423947e-8, **not BDF global error certificate**.

Operator-only candidate `E2_candidate = 3 B_Q/(4 sqrt(6))` yields symmetric homogeneous collision 2x2 `[-9/10,-sqrt6/10;-sqrt6/10,-2/5]`, eigenvalues -1,-3/10, Wolfram checked. This is NOT yet a full source-native E2 basis/transport equivalence or a new full Einstein-Boltzmann solution.

## Exact replay

Source-full ZIP is the primary reproducibility artifact; original 29,752,361-byte parent archive remains nested byte-exact. Extract to an **empty directory**, then `bash VERIFY.sh` (9 new unit tests + complete SHA source + 50-case reproduction and output comparison). Full numeric NPZ: `results/STRESS_50x193x6_SAME_OWNER.npz`; accompanying CSV: 57,900 deltaJ rows; 28,950 paired; 9,650 time/stress work; 50/25 endpoints. Git PR133 is a text mirror, and does not itself contain the nested source binary. The source ZIP, CSV, checksum manifest and provider locations should be inspected via final receipt before reporting dual backups.

## Blind referee, unclosed controls and next path

Same-assistant hostile check + Wolfram/independent tensor quadrature performed. Distinct-author R6/C9 **NOT_RUN**. Historical parent PR132 ledger/authority CI and PR127/PR130 records are not rewritten. New node is auxiliary 1/1 scoped with official delta0; no merge, no production promotion. New science only after explicit new authorization: bind candidate E2 to exact source-owner definitions and perform genuine same-event J_l/E2 co-evolution, then source-preserving radiation Einstein/shear feedback with chemistry/opacity changes; do NOT import disjoint CE1/KM2E result as same physical event. Literature: Challinor PRD62 (2000) DOI10.1103/PhysRevD.62.043004; Pontzen & Challinor MNRAS380 (2007) DOI10.1111/j.1365-2966.2007.12221.x; Anile ApSS29 (1974) DOI10.1007/BF02639073; Kamionkowski PRD104 (2021) DOI10.1103/PhysRevD.104.063512. No additional user-supplied literature is required for this bounded result.
