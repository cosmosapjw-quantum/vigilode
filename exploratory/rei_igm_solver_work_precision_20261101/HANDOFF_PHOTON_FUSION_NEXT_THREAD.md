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


---

## SUPERSEDING NEXT-THREAD HANDOFF: source-native 26-event root shadow (2026-11-01)

```text
PROJECT=cosmosapjw-quantum/vigilode
RESEARCH_TRACK=REI_IGM_VIGILODE_REAL_26EVENT_OPTIMIZATION
PR=183
BRANCH=research/vigilode-rei-igm-photon-fusion-20261101
READ_LIVE_HEAD_FIRST=true
ORIGINAL_VIGILODE_PR178_HEAD=ab52c102cb7d30e1e0f030d05a9a04950001a494
FROZEN_REI_OWNER_HEAD=fcc90939b6b7cc842fae65d5a0ffbb69a3ec3120
FROZEN_REI_SOURCE_SHA256=a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b
FROZEN_REI_STATE_SHA256=cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01
FROZEN_REI_KERNEL_SHA256=579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644
REI_ORIGINAL_ACCEPTED_TICK=160/131072
REI_ORIGINAL_NODES=46080
REI_ORIGINAL_26EVENT_FULL_HALF1_HALF2=SCOPED_PASS
REI_ORIGINAL_14_NATIVE_LEDGER=14/14_PASS
REI_ORIGINAL_HISTORICAL_CROSSBOX_LOGT=2.1245050576368385e-4
REI_ORIGINAL_LOCAL_GATE=2e-4
ORIGINAL_HISTORICAL_LOGT_STATUS=FAIL_RETAINED
EXPERIMENT=READONLY_THERMAL_ROOT_DUPLICATE_BRACKET_EVAL_REMOVAL
ORIGINAL_THERMAL_RHS_CALLS_PER_THREE_MAPS=90
OPTIN_SHADOW_THERMAL_RHS_CALLS_PER_THREE_MAPS=72
ORIGINAL_STATE_BYTES_ALL_A_B_B_A=IDENTICAL
ROOT_SHADOW_ABBA_DIAGNOSTIC_WALL_SPEEDUP=1.0551085595375107
VIGILODE_PR181_NONAUTON_NATIVE_MODULE=REUSED_OPTIN
VIGILODE_PR181_SOURCE_SCOPED_NATIVE_METHODS=8/8_PASS
VIGILODE_NATIVE_NONAUTON_TESTS=6/6_PASS
PYTHON_NORMAL=14/14_PASS
PYTHON_OPTIMIZED=14/14_PASS
VIGILODE_OFFICIAL_DAG=7/23
VIGILODE_SUCCESSOR_DAG=0/12
REI_CENTRAL_DAG=28/88
REI_WORKING_DAG=33/88
REI_PHYSICAL_FAST_DAG=4/7
CANONICAL_DELTA=0
SOURCE_OWNER_MUTATED=false
PRODUCTION=HOLD
ORIGINAL_TICK161_ADMITTED=false
NO_MERGE=true
NO_FORCE_PUSH=true
NO_TOLERANCE_RELAXATION=true
AUTO_NEXT_NODE=false
BLIND_REFEREE=SAME_AUTHOR_ONLY
NATIVE_SUBAGENT=NOT_RUN_CAPABILITY_UNAVAILABLE
```

The previous photon cached-JVP result remains historical source-scoped research, not original 26-event performance. The actual whole original full/half/half bracket candidate has a new real result (5.5% exploratory timing improvement, exact 46k population and thermal intervals). Read
`ORIGINAL26_OWNER_MAP_PROFILE.json`, `ORIGINAL26_ROOT_ABBA.json`,
`NATIVE_PR181_NONAUTON_REPLAY.json` and `THEORY_AND_REVIEW_FUSION_KO.md`
in the same directory. The author code is
`original26_owner_map_profile.py`, `original26_root_duplicate_eval_shadow.py`
and the original unmodified `original26_photon_group_fusion.py`.

REPRODUCE (on the authorized Linux host with exact source snapshot):

```bash
cd /home/cosmosapjw/worktrees/vigilode-rei-igm-photon-fusion-20261101
python3 -m unittest discover -s exploratory/rei_igm_solver_work_precision_20261101/tests -v
python3 -O -m unittest discover -s exploratory/rei_igm_solver_work_precision_20261101/tests -v
cargo test --offline --locked -p rodas5p-integrators --example rei_igm_five_clock_source
cargo run --offline --locked -p rodas5p-integrators --example rei_igm_five_clock_source -- --nonauton-study
python3 exploratory/rei_igm_solver_work_precision_20261101/original26_root_duplicate_eval_shadow.py --owner /home/cosmosapjw/vigilode-rei-owner-f05-readonly-20261101 --out /tmp/rei_vigilode_root_abba_replay.json
```

NEXT_SCIENCE_ACTION: do an **owner-controlled versioned opt-in** of this exact thermal-root bracket-elimination, test across all three original physical shape lanes, original 14 raw ledgers, Hummer-Seaton events, historical source owner V1/V2 authority, prior accepted 0-64 parity, and independent clean pinned-environment numerical readback. Do not promote tick161 or replace default owner without original scientific admission. No additional surrogate photon-source atlas. Original source owner is NOT edited by this VigilODE research PR.

BACKUP_LOCATION=/home/cosmosapjw/archives/vigilode_rei_igm_photon_fusion_20261101/
AUTHORITATIVE_GITHUB_PR=https://github.com/cosmosapjw-quantum/vigilode/pull/183
```
