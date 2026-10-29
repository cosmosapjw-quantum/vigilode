# Next thread executable handoff: VigilODE N01 C5 → C6

$physmath-research-loop $scispace $exa $wolfram

PROJECT=cosmosapjw-quantum/vigilode
PARENT_PR=117
PARENT_HEAD=0032b7bd7a0d5092eac26ad9408b9a261df2916e
CURRENT_PR=119
BRANCH=research/n01-df-value-map-c5-20261030
NEXT_STAGE=C6_NATIVE_INTEGRATION_VERIFICATION
AUTO_NEXT_NODE=false
PRODUCTION=HOLD

DAG_VIGILODE=7/23
DAG_REI_CENTRAL=28/88
DAG_SUCCESSOR=0/12_ADMITTED
N00F_AUDIT=1/1_SCOPED
N01_R5_EXACT=1/1_SCOPED
N01_C5_INTERFACE_MIRROR=1/1_SCOPED
N01_NATIVE_RODAS8_ADMISSION=0/1

ARCHITECTURE=causal+parallelized+derivative-free+matrix-free-augmented Rosenbrock/RK; parallel accelerated homotopy Newton-free root finder
DERIVATIVE_FREE_RHS_ONLY=PRIMARY
SUPPLIED_JVP=OPT_IN_FALLBACK
EXPLICIT_JACOBIAN=INDEPENDENT_REFERENCE_ONLY

READ_FIRST:
- PR117 R5 theorem and exact Fraction oracle
- PR91 RHS-only homotopy, causal Picard, structural derivative-free raw-U
- PR115 mass-action source + opaque trial + epoch
- PR119 `crates/vigilode-coupled/src/n01_derivative_free.rs`
- PR119 `crates/vigilode-coupled/tests/n01_df_value_stage_contract.rs`
- PR119 `research/n01_df_value_map_c5_20261030/RESULTS.json`, DAG_DELTA.json, WORK_CONTRACT.md and retrospective PREREGISTRATION.md
- PR74/90/92/93/96, R-NEXT-07 epoch, original production and vectorized JF designs

ACTUAL_RESULTS:
- Tested source git blob: 7c2cd365bb830b778910e98fb29be4c36f4cfd8e
- Native test git blob: ef0d575ec40ee0e35016ad91e6522eaf6c769691
- Rust1.94.1 isolated mirror 10/10, Clippy/fmt PASS
- Independent exact Fraction 256 rows, 1280/1280 PASS
- Full native workspace / real RODAS8 / matched-error IVP / independent R6 C9 NOT_RUN
- Retrospective chronology preserved; old first CI research-process fail retained

NEXT ACTIONS (C6 only):
1. Fetch exact PR119 head, test/native source and crate dependency; run full native workspace `cargo test -p vigilode-coupled --locked --offline --test n01_df_value_stage_contract`, strict Clippy/fmt, and original PR91 regressions. Classify any failure independently.
2. Check trait exact correspondence and direct interval semantics; preserve unchanged default production route and sourced original R5 theorem. Ensure failure outputs and bounded work are retained.
3. Verify `N01MassActionOwner` read-only callbacks; PR115 legacy adapter still calls advance() for ephemeral stage probes and retains explicit Jacobian preference. Do NOT silently treat legacy adapter as derivative-free or merge.
4. If native proof-grade stage bridge passes, next separately authorized C7 links to full eight-stage RODAS raw-U original-space residual, causal lower-triangular norm transfer and accepted output; do not equate auxiliary single-stage with full 8-stage.
5. Only after separately warranted: parallelize independent physical cells/stage partitions with joined failures; implement opt-in JVP fallback for derivative-free refusal, measure same achieved-error full IVP and negative costs, different-author R6/C9.
6. Stage-by-stage GitHub checkpoints, immutable Drive+Dropbox backups with actual provider receipts/size/hash, and next copyable handoff.

NO main merge, force push, tolerance weakening, default solver route changes, production promotion or N02 until independent source/physics admission.

PACKAGE VERIFICATION: `bash VERIFY.sh` (in extracted evidence ZIP) must be run using Rust 1.94.1. The standalone mirror explicitly excludes the included native integration source from its own Cargo test discovery. Its local library 10/10 and binary64 Fraction 1280/1280 PASS are distinct from native workspace tests, which must be run in a real repo checkout. Initial packaging failure E0432/E0433/E0689 was corrected only in the mirror, see `evidence/verify_clean.log` and `evidence/verify_clean_fixed.log`.
CI: The earliest research-process check failed for missing retrospective prereg/results/ledger; these were posted in later commits; subsequently the authority-refs job still failed with historical published source identity issues (42 missing + 26 allowlisted, 19 invalid-repo,16 unavailable,261 non-commit). No CI full-green claim. Retain Q-LEDGER/Q-API/Q-GIVENS and production HOLD.
