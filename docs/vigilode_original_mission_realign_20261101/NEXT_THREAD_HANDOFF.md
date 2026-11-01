# VigilODE standalone G0 science-first audit → next thread handoff

PROJECT=cosmosapjw-quantum/vigilode
TRACK=ORIGINAL_GENERIC_CAUSAL_JACOBIAN_FREE_STIFF_OSCILLATORY_POLYALGORITHM
EXCLUDE_APPLIED_PROJECT_BRANCHES=true
READ_LIVE_HEAD_FIRST=true

BASE_PARENT_HEAD=7e8d7f5cdf2b82c4984528a6a2df1f9647e28fde
CURRENT_BRANCH=governance/vigilode-original-mission-realignment-20261101
CURRENT_HEAD=READ_REMOTE_BRANCH_FIRST
PR=READ_NEW_DRAFT_PR
NO_MERGE=true
NO_DEFAULT_CHANGE=true
NO_TOLERANCE_RELAXATION=true
AUTO_NEXT_NODE=false

INITIAL_G0_SOURCE=PREIMPORT_HISTORY_RESEARCH_GENERIC_VECTORIZED_JF_V1
SOURCE_PLAN=research/polyalgorithm_alternatives_20261028/LOCAL_DEVELOPMENT_PLAN.json
SOURCE_PLAN_SHA256=8a05bdf42d999f724662d64b5fc9a36bc5bb0caa3a9f5022b7333dcc914bc3f1
SOURCE_PLAN_23_NODES=7_COMPLETE_SCOPED+15_PENDING_LOCAL+1_AUTHORITY_PENDING
OTHER_A01_A12_SCOPED_IMPLEMENTATION=SEPARATE_DAG_NOT_PRODUCTION_12_PASS
OFFICIAL_DAG=7/23_UNCHANGED
SUCCESSOR_ADMITTED=0/12_UNCHANGED
PRODUCTION=HOLD

DOCS=docs/vigilode_original_mission_realign_20261101/
INVENTORY=INTEGRATOR_INVENTORY.json
INVENTORY_ENTRIES=28_INCLUDING_STEPS_COMPONENTS_CONTROLS
REFRESHED_DAG=REFOCUSED_DAG.json
REFRESHED_DAG_AUTHORITY=NONCANONICAL_RESEARCH_START_OVERLAY
ORIGINAL_RELEASE_DAG_EDGES=UNCHANGED
RESEARCH_START_EDGES_ADJUSTED=7
VALIDATOR=python3 tools/validate_vigilode_core_realign.py --check
NATIVE_SUBAGENT=NOT_RUN

ORIGINAL_SCIENCE_G0:
  Protect original 8-stage sequential JVP Rosenbrock R-JF;
  first attempt lower causal critical depth without explicit assembled Jacobian or Newton;
  prefer derivative-free source-permitted fast proposals when mathematically defensible;
  select admissible E-K or q1/q2 candidate without changing original raw-U/root;
  count rejected, aborted, diagnostic and fallback costs;
  compare full actual generic IVPs on matched achieved global error and identical output grid;
  if no benefit, refuse candidate and retain protected R-JF and RHS-only baseline.

PRIMARY_M1=L-HARDWARE_SCOPED_SAME_RUNNER_MATCHED_GLOBAL_ERROR
PRIMARY_M2=PR-N04_AND_PR-N05_ORIGINAL_STAGE_CAUSAL_DEPTH
M3=PR-N06_CONDITIONAL_SWITCH_REENTRY_AFTER_M1_M2
M4=STRICT_RELEASE_AFTER_FULL_CI_AND_SOURCE_AUTHORITIES

NEXT_EXECUTABLE_SCIENCE:
  1. Pin clean source and generic-only Robertson/Prothero-Robinson/nonnormal/reentrant IVPs.
  2. Run protected R-JF, adaptive pexprb54s4, RHS-only DOPRI5 with same-machine clock/hardware, identical source and requested outputs; compute achieved global errors against independent high-accuracy reference before wall ranking.
  3. Profile actual 8-stage raw-U causal depth and full JVP/RHS/callback+allocation+speculation+fallback work for q1/q2 and E-K.
  4. Adverse source epoch/clock and original residual attacks, then at most one fresh adversarial review.
  5. Record Pareto and failed cases, never promote on microkernels or work proxy alone.

PRIMARY_STOP=FALSE_PASS_OR_ORIGINAL_RAW_U_MISMATCH
SCIENCE_SUCCESS=SCOPED_MATCHED_ACHIEVED_GLOBAL_ERROR_AND_TOTAL_WORK_RATIO
TARGET_ORDER>=4.8
TARGET_MEDIAN_SPEEDUP>=1.15
TARGET_ACCEPTED_STEPS_RATIO<=1.2
TARGET_FALLBACK_FRACTION<=0.10
MAX_FAST_LANE_EXPLICIT_JACOBIAN=0
MAX_FAST_LANE_NEWTON_ITERATIONS=0
SOURCE_CLIENT_MODELS=VIGILODE_GENERIC_ONLY
NO_COSMOLOGICAL_APPLICATION_BRANCHES=true

CI_GLOBAL=CHECK_LIVE_NOT_ASSUMED_GREEN
RELEASE_STATUS=HOLD
ALL_PREVIOUS_RUST_SOLVER_SOURCES=UNCHANGED_BY_AUDIT
