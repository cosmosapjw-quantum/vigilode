# VigilODE REI IGM optimization handoff: actual 46k source + cached JVP

PROJECT=cosmosapjw-quantum/vigilode
TRACK=VIGILODE_REI_IGM_PHOTON_COEFFICIENT_FUSION_OPTIN
BASE_PR=178
BASE_HEAD=ab52c102cb7d30e1e0f030d05a9a04950001a494
APPLIED_REUSE=VIGILODE_PR180_TEST_LITERAL_FIX_ONLY
PARENT_REI_SOURCE_PR=348
PARENT_REI_HEAD=fcc90939b6b7cc842fae65d5a0ffbb69a3ec3120
NEWER_REI_PHYSICAL_SHADOW=PR354_HEAD_6574db945a1b96fa5e0b55923e36781b5db0b03b
ORIGINAL_FROZEN_26_EVENT_KERNEL_SHA256=579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644
ORIGINAL_FROZEN_ARCHIVE_SHA256=a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b
ORIGINAL_ACCEPTED_ARRAY_SHA256=cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01
ACCEPTED_TICK=160/131072
ORIGINAL_NODES=46080
GROUPS=4
SHARED_GLOBAL_NEUTRAL_MOMENTS=3
NONLOCAL_PHOTON_ONLY_RANK_UPPER=3
CANONICAL_DAG_VIGILODE=7/23
VIGILODE_SUCCESSOR_DAG=0/12
CENTRAL_REI_DAG=28/88
WORKING_REI_DAG=33/88
PHYSICAL_FAST_REI_DAG=4/7
SCIENTIFIC_DELTA=0
STATUS=SOURCE_SCOPED_NUMERICALLY_CHECKED_IMPLEMENTATION_VERIFIED_ONLY
PRODUCTION=HOLD
NO_MERGE=true
NO_FORCE_PUSH=true
NO_DEFAULT_CHANGE=true
NO_TOLERANCE_CHANGE=true
AUTO_NEXT_NODE=false

ACTUAL_NEW_CODE:
  exploratory/rei_igm_solver_work_precision_20261101/original26_photon_group_fusion.py
  exploratory/rei_igm_solver_work_precision_20261101/tests/test_photon_group_fusion.py
ACTUAL_NEW_RESULTS:
  exploratory/rei_igm_solver_work_precision_20261101/GROUP_FUSION_FIRST_RUN.json
  exploratory/rei_igm_solver_work_precision_20261101/ORIGINAL26_GROUP_FUSION_CACHED_RESULT.json
  exploratory/rei_igm_solver_work_precision_20261101/THEORY_AND_REVIEW_FUSION_KO.md

REPRODUCE:
  python3 exploratory/rei_igm_solver_work_precision_20261101/original26_photon_group_fusion.py --owner-root /home/cosmosapjw/vigilode-rei-owner-f05-readonly-20261101 --out /tmp/vigilode_rei_photon_check.json --repetitions 9
  python3 -m unittest discover -s exploratory/rei_igm_solver_work_precision_20261101/tests -v
  python3 -O -m unittest discover -s exploratory/rei_igm_solver_work_precision_20261101/tests -v
  cargo test --offline --locked -p rodas5p-integrators --example rei_igm_five_clock_source
  cargo test --offline --locked -p rodas5p-integrators --example rei_igm_native_solver_comparison

OBSERVED:
  original_owner_species_include=46080/46080_each
  original_owner_heat_include=46080/46080
  source_rhs_fusion=REGRESSION_0.769x
  cached_source_only_JVP=DIAGNOSTIC_1.939x_EXCLUDING_SETUP
  cache_invalidation=NEW_PER_STAGE_STATE_CLOCK_SOURCE
  owner_heii_longdouble_to_f64_underflow=UNRESOLVED
  full26event_timemap=NOT_RUN
  real_original_tick161_admission=NOT_RUN

NEXT_PRODUCTIVE_UNIT:
  Opt-in rust-source adapter for fixed-(state,clock) rank-3 photon
  partial JVP with analytic owner coefficient and full 26-event
  MPRK22(1)/Alexander SDIRK2 *shadow* step. Reuse existing native
  matrix-free GMRES, transactional q1/q2, low_rank_bordered where
  mathematically appropriate. Run original full/half/half source with
  untouched 14 ledgers, fallback, actual speed and unchanged gate.
  DO NOT repeat a fourth isolated photon atlas/metadata audit.
