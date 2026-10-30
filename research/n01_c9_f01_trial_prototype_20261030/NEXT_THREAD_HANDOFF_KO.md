# VigilODE C9 F01 scoped source-trial prototype → 실제 독립 C9 reviewer 인계

$physmath-research-loop $scispace $exa $wolfram

PROJECT=cosmosapjw-quantum/vigilode
PARENT_PR=126
PARENT_HEAD=2688a59e0b8ab88120105a6949db68d61a378e71
NEW_DRAFT_PR=128
NEW_BRANCH=research/n01-c9-f01-source-trial-prototype-20261030
VERIFIED_NATIVE_RUN=40989982863
VERIFIED_NATIVE_JOB=123849770277
VERIFIED_NATIVE_HEAD=1b98ec58353e80e553d03df24d2afaf12aeaef39
NEW_STAGE=C9_F01_SOURCE_TRIAL_REPAIR_PROTOTYPE_1/1_SCOPED
NEXT_REQUIRED_STAGE=C9_DISTINCT_AUTHOR_INDEPENDENT_REVIEW
AUTO_NEXT_NODE=false
AFTER_INDEPENDENT_ACCEPTANCE_ONLY=C10
PRODUCTION=HOLD

DAG_VIGILODE=7/23
DAG_REI_CENTRAL=28/88
DAG_SUCCESSOR_ADMITTED=0/12
N01_C6=1/1_SCOPED
N01_C7=1/1_SCOPED
N01_C8=1/1_SCOPED
N01_C9_TECHNICAL=1/1_SCOPED
N01_C9_F01_OWNER_TRIAL_PROTOTYPE=1/1_SCOPED
N01_C9_INDEPENDENT_AUTHOR=0/1_NOT_RUN
N01_C10=NOT_STARTED

FROZEN_SOURCES:
- Original C8 baseline blob: e6a33b66ba80055cae9439cf683a1715ca77c5a2
- PR126 C9 test blob: b6a0f06aba6d5d48d0de056ec9eadde1467904af
- New owner-trial source blob: ffe079e4fec4a8569f0661e122a97b2ac8ae4680
- New C8+C9+F01 test blob: bb4637b39b213e47941b319f63f042e7adb86359
- Tableau snapshot SHA256: 211255f6125b648c6955833c3da537d9c031ec78168b2918563c29fa88c8bce6

SCIENCE:
- Existing C8 test-only certificate accepted free caller case.k; C9 exact fake k=0 and true source differed (tau=1/64 true original raw-U0 max=25/512).
- New opt-in guard checks same opaque owner, epoch, clock bits, carrier bits, plus immutable owner-supplied rates (before/after). No default route changed.
- h=1e-4, k=(.5,10,200), y=(.75,.125,.125), fake k=0 zero stages gives fake residual0/bound0.
- Actual source with zero stages: original residual 1.32460976996431364e-4, POSITIVE_TUBE_REFUSED, bound None. Correct refusal.
- Actual native 8-stage RODAS small direct reference: original raw-U residual 5.73949525059513910e-18, conditional algebraic upper 3.0599993684558566e-15, PROVED_CONDITIONAL_TUBE, 8 direct linear solves.
- Negative trials: different owner, stale epoch, changed time/carrier including -0.0, nonfinite input, wrong shape all refused.
- Wolfram exact F01: fake zero; true (7/2048,93/2048,-25/512) at tau1/64, source mass identity 0.

REPRO_COMMANDS:
cargo test --locked --offline -p vigilode-coupled --test n01_df_c8_work_precision -- --nocapture
cargo test --locked --offline -p vigilode-coupled --test n01_df_value_stage_contract --test n01_df_c7_original_raw_u_contract
cargo test --locked --offline -p vigilode-coupled --test rhs_homotopy_contracts --test causal_contracts --test causal_reuse_contracts
cargo clippy --locked --offline -p rodas5p-core -p vigilode-coupled --test n01_df_c8_work_precision -- -D warnings
rustfmt --check --edition 2024 crates/rodas5p-core/src/n01_mass_action_owner.rs
rustfmt --check --edition 2024 crates/vigilode-coupled/tests/n01_df_c8_work_precision.rs

TEST_VERDICT=44/44_NATIVE_PASS_STRICT_CLIPPY_AND_FMT_PASS
PREVIOUS_RED_RUNS=40989229211,40989578203

IMPORTANT_UNRESOLVED:
1. This is still a research-only wrapper around C8 test checker, NOT a bound physical production adapter. A caller can present a different, equally valid owner that is NOT the owner of the actual OdeProblem callback. The wrapper has no proof of arbitrary native callback ownership or step-size identity.
2. Existing n01_mass_action_owned_problem callbacks call advance(t,state) at ephemeral probes, changing model epoch and invalidating trial/cache. Determine source model-generation versus temporary stage-evaluation epochs before production integration.
3. Exact stage algebraic bound is NOT global-IVP truncation/stability/error bound. BASS physical33 and full 8-stage derivative-free production implementation remain unverified.
4. C9 independent different-author reviewer/agent NOT_RUN (no delegated agent facility in this chat). SciSpace, Exa and Wolfram are supporting reference/computation services, not a separately accountable author. Never claim C9 independent 1/1 without actual reviewer identity and readback.
5. Original PR91 RHS-only derivative-free causal homotopy remains PRIMARY, optional JVP fallback only, dense n3 reference oracle only. No main merge, force push or tolerance change.

NEXT_ACTION_C9_ONLY:
- An actual distinct author reviews both frozen PR126 and PR128 diff with exact source bytes.
- Reproduce F01 forged-source and true-source counterexamples; audit trial/clock/epoch and general OdeProblem callback identity.
- Verify original 8-stage raw-U source-domain tube, interval residual/roundoff, coefficient K/U transport, time discretization vs algebraic error, and work accounting.
- Produce reviewer identity, independent machine commands+exit, code hashes, findings or actual counterexamples. Return accepted/rejected/corrected claims. Do NOT start C10 before this evidence.
- Preserve a new immutable checkpoint and actual GitHub/Drive/Dropbox ACKs.

RETURN=REVIEWER_ID_SOURCE_SHA_ACTUAL_TESTS_AND_FINDINGS_DAG_BACKUP_RECEIPTS_NEXT_HANDOFF
