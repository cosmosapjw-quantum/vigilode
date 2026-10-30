# VigilODE next-thread handoff: C9 H1/H2 trusted immutable source bundle

$physmath-research-loop $scispace $exa $wolfram

PROJECT=cosmosapjw-quantum/vigilode
PARENT_PR=129
PARENT_HEAD=1771fe51f1dfbc2395ca575a99a9f749918e08d3
CANDIDATE_DRAFT_PR=131
CANDIDATE_BRANCH=research/n01-c9-trusted-frozen-probe-bundle-20261030
VERIFIED_NATIVE_HEAD=6c060d963fa3257dd982d09a562c639bbd63fa3e
VERIFIED_NATIVE_RUN=41006566540
VERIFIED_NATIVE_JOB=123902686470
VERIFIED_NATIVE_ARTIFACT=12955522545
STAGE_EXECUTED=C9_H1_H2_SOURCE_BUNDLE_SCOPED_REPAIR
NEXT_REQUIRED=C9_REAL_DIFFERENT_AUTHOR_INDEPENDENT_REVIEW
AUTO_NEXT_NODE=false
C10=NOT_STARTED
PRODUCTION=HOLD

DAG_VIGILODE=7/23
DAG_REI=28/88
DAG_SUCCESSOR_ADMITTED=0/12
C9_SAME_ASSISTANT_TECHNICAL=1/1_SCOPED
C9_TRUSTED_BUNDLE=1/1_NATIVE_SCOPED
C9_DIFFERENT_AUTHOR=0/1_NOT_RUN

PROTECTED_ARCHITECTURE:
- Causal parallel RHS-only derivative-free Newton-free homotopy PRIMARY
- Supplied JVP only OPTIONAL secondary fallback
- Dense n3 direct original RODAS5P SMALL REFERENCE ONLY
- No modification to existing adapter/default, tableau or tolerances

RUNTIME_CURRENT:
- Container Python 3.13.5 PYTHON_PASS; no local Rust compiler
- Actual GitHub Actions Rust1.94.1 Ubuntu24.04 native SUCCESS

NEW_SOURCE:
- crates/rodas5p-core/src/n01_mass_action_owner.rs: read-only box probe
- crates/rodas5p-integrators/src/n01_frozen_source_bundle.rs: private single-source Arc, callback bundle and frozen one-step session
- crates/rodas5p-integrators/src/lib.rs: opt-in export
- crates/rodas5p-integrators/tests/n01_frozen_source_bundle_contract.rs: new owner/epoch and original raw-U tests
- Full exact tested bytes in this ZIP sources/ and GitHub artifact 12955522545; compare native SOURCE_HEAD and Git blob/SHA files.

RUN_NATIVE:
cargo test --locked --offline -p rodas5p-integrators --test n01_frozen_source_bundle_contract -- --nocapture
cargo test --locked --offline -p vigilode-coupled --test n01_df_c8_work_precision
cargo test --locked --offline -p vigilode-coupled --test n01_df_value_stage_contract --test n01_df_c7_original_raw_u_contract --test rhs_homotopy_contracts --test causal_contracts --test causal_reuse_contracts
cargo clippy --locked --offline -p rodas5p-core -p rodas5p-integrators --test n01_frozen_source_bundle_contract -- -D warnings
rustfmt --check --edition 2024 crates/rodas5p-integrators/src/n01_frozen_source_bundle.rs
rustfmt --check --edition 2024 crates/rodas5p-integrators/tests/n01_frozen_source_bundle_contract.rs

VERIFIED:
- Original 8-stage small direct ref: first original raw-U0 residual=1.35525271560688054e-20, mass defect=0, direct linear solves=8
- RHS stage callbacks at t=0 and h/2: same exact source RHS, model_epoch 0->0
- Different fake source cannot be substituted via this bundle API
- All native tests 49/49, Clippy/fmt PASS
- Two historical formatting RED runs preserved

STILL_BLOCKING:
1. Frozen bundle cannot advance accepted model state; source generation remains immutable throughout its lifetime. Do NOT claim whole-IVP integration or correct accepted/rejected state transition.
2. This private callback identity does NOT attest generic users' OdeProblem or BASS physical33 source/clock. Legacy n01_mass_action_owned_problem has stage-probe epoch churn, unchanged.
3. No true derivative-free full RODAS8 algorithm or eight-stage outward physical error certificate/global IVP proof. Tiny direct reference does not imply performance speedup.
4. Formal different-author C9 review NOT_RUN. Existing ChatGPT same-assistant adversarial calculations and SciSpace/Exa/Wolfram are not such a reviewer.

NEXT_C9_ONLY:
- Real distinct author/hosted agent must independently read frozen PR129 and new PR131 source bytes, reproduce H1/H2 and new bundle test, check Rust ownership and private bundle invariant.
- Independently attack temporary stage domain, source model generation, accepted-step commit, rollback and true 8-stage physical residual/roundoff.
- If accepted, return actual reviewer identity, code SHA, commands/exits, real negative/positive evidence and signed scoped opinion. Only afterward C10 may run.
- Never merge main, force push, widen source claims or relax tolerances.

RETURN=REAL_REVIEWER_ID, PR_HEAD, SOURCE_BLOBS, COMMANDS_EXITS, NEGATIVE_AND_POSITIVE_EVIDENCE, PROVIDER_RECEIPTS, DAG, NEXT_HANDOFF
