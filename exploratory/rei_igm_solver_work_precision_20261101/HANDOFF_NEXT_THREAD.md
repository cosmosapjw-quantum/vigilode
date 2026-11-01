# VigilODE / REI Bianchi IGM scoped optimization handoff, 2026-11-01

PROJECT=cosmosapjw-quantum/vigilode
CENTRAL_OWNER=cosmosapjw-quantum/rei_bianchi
TRACK=REI_IGM_VIGILODE_SOURCE_OPTIMIZATION
SOURCE_VIGILODE_PR=175
SOURCE_VIGILODE_HEAD=d99c3f06d46e233a7848e5b4562b4a59e2b5d900
SOURCE_REI_PR=348
SOURCE_REI_HEAD=fcc90939b6b7cc842fae65d5a0ffbb69a3ec3120
ORIGINAL_ARCHIVE_SHA256=a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b
ORIGINAL_STATE_ARRAY_SHA256=cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01
ORIGINAL_26EVENT_KERNEL_SHA256=579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644
ORIGINAL_ACCEPTED_TICK=160/131072
ORIGINAL_NODES=46080
PHOTON_GROUPS=4
GLOBAL_NEUTRAL_MOMENTS=3
PHOTON_NONLOCAL_JACOBIAN_RANK_UPPER=3
INITIAL_F64_HI_ENCLOSURE=0/46080_FAILED_PRESERVED
HYBRID_SPECIES_OWNER_ENCLOSURE=46080/46080_EACH_FOR_HI_HEI_HEII
HYBRID_HEAT_OWNER_ENCLOSURE=46080/46080
HEII_LONGDOUBLE_POSITIVE_CELLS=46080
HEII_F64_ZERO_UNDERFLOW_CELLS=46080
HEII_TRACE_LONGDOUBLE_TOTAL_RATE_PER_S=4.9266637335576802504e-324
JVP_FD_MAX_RELATIVE_ERROR_APPROX=7.95e-13
PR175_NATIVE_SOLVER_ROUTES=4
PR175_SOURCE_CASES=2
RUST_TESTS=5/5_PASS
PYTHON_NORMAL=5/5_PASS
PYTHON_OPTIMIZED=5/5_PASS
CENTRAL_REI_DAG=28/88
WORKING_REI_DAG=33/88
REI_PHYSICAL_FAST=4/7
VIGILODE_CORE_DAG=7/23
VIGILODE_SUCCESSOR_DAG=0/12
CANONICAL_DELTA=0
FULL_26EVENT_DH_JACOBIAN=NOT_RUN
FULL_26EVENT_DH_ALLX_UNIFORM_REMAINDER=NOT_RUN
PRODUCTION=HOLD
ORIGINAL_TICK161=NOT_ADMITTED
NO_MERGE=true
NO_FORCE_PUSH=true
NO_TOLERANCE_RELAXATION=true
AUTO_NEXT_NODE=false

REPRODUCE:
  cargo test --offline --locked --release -p rodas5p-integrators --example rei_igm_native_solver_comparison
  cargo run --offline --locked --release -p rodas5p-integrators --example rei_igm_native_solver_comparison
  python3 -m unittest discover -s exploratory/rei_igm_solver_work_precision_20261101/tests -v
  python3 -O -m unittest discover -s exploratory/rei_igm_solver_work_precision_20261101/tests -v
  python3 exploratory/rei_igm_solver_work_precision_20261101/original26_photon_lowrank_check.py --owner-root "$HOME/vigilode-rei-owner-f05-readonly-20261101" --lane LOCAL_NEUTRAL_HAZARD_PRIMARY --epsilon 0.0002 --out exploratory/rei_igm_solver_work_precision_20261101/ORIGINAL26_PHOTON_JVP_RESULT.json

EVIDENCE:
  exploratory/rei_igm_solver_work_precision_20261101/WORK_PRECISION.json
  exploratory/rei_igm_solver_work_precision_20261101/ORIGINAL26_INITIAL_INTERVAL_FAIL.json
  exploratory/rei_igm_solver_work_precision_20261101/ORIGINAL26_PHOTON_JVP_RESULT.json
  exploratory/rei_igm_solver_work_precision_20261101/ADVERSARIAL_REVIEW_KO.md

NEXT_ACTION: source-owning full 26-event same-parent solver/low-rank operator experiment. Preserve original four source evaluation sites, positivity, original MPRK22/SDIRK2 coefficients, implicit thermal roots, Hummer-Seaton event boundaries, conserved H/He nuclei, photon and thermal ledgers and unchanged original error gate. Integrate new Jacobian only as an opt-in, residual-checked operator. Stop standalone photon atlases. This result does not certify whole D_h or original interval admission.
