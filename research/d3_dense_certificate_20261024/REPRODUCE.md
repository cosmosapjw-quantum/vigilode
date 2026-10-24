# D3 reproducibility and evidence reuse

## Default next action

Read the existing results and current native owner/source/reservation/live dirty/completion receipt. This D3 campaign is complete. Do not rerun D2, D3, older benchmarks, NCP or V3 harmonic for reassurance. `BUDGET.json` is closed and `run.py` rejects attempts to reuse it. Historical ledgers are preserved separately.

The full S3 archive is the complete deliverable: source, exact source identities, commands, logs, all accepted/refused records, full `REVIEW.json`, figures, report and handoff. Git contains source and compact scientific evidence; `REVIEW_SUMMARY.json` names the exact SHA-256 of the full reviewer output. It omits only the 1484 detailed plot rows from the public summary. The complete rows and raw data remain in the archive.

## Inputs and model

- Original S0: `d702012e06eeafda5ce30b7c017e51b2c9bcbe4c`, archive26708bytes, SHA256 `feb6685e7d7fec6c2a1c09df2c8979460c2ccc6e8a46c44160e10b3834a38970`.
- D2 parent: `1c829f206ab67585191231be7f5135fd0d4c804e`; science publication `1ef6c519262560c2281a58b4cbb8ed38725ad90f`.
- D2 `admission/domain/integer_gate/stage_safe/table/trajectory` exact bytes reused. Source identities: `intake/REUSED_SOURCE_IDENTITY.json`.
- Native `directed.rs` is blob `89e2b3122b08db1849e4e56cc94f1f58c473b735` at public frontier `aaf1f834d4c589e734ca7851a56c11bb44f7047a`; no unsafe additions.
- Fixed model `z'_j=-lambda_j*z_j-alpha*z_j^2`, rates1/20/300 repeated, alpha0/4, normalized coordinates/time, exact represented initial values from `trajectory::initial`.
- Eight combinations: degree3/5 × alpha0/4 × full33 absolute curve budget2e-6/2e-8. `tf=.25,h0=.004,attemptcap20000,rtol=budget,atol=budget*1e-3`.
- Requested schedule `tf*float(k)/37.0`, k1..37, last exactlytf; one strict midpoint per accepted interval. Requested times do not force solver landing points.

## Actual environment

The supplied Rust bundle was restored locally; no Desktop Commander call occurred.

- Archive: `rust-1.94.1-x86_64-unknown-linux-gnu.tar.xz`,192287020bytes.
- SHA256: `294b3d81fa72e62581276290c60c81eb8b58498d333d422ca1dfc432877d0c40`.
- Rust1.94.1, commit `e408947bfd200af42db322daf0fadfe7e26d3bd1`, LLVM21.1.8.
- Compilation: edition2021; campaign `-O`; no fast-math override.
- Python3.12.14; standard-library Fraction and Decimal80 for the independent oracle; installed Matplotlib for figures.
- Signature validation of the supplied compiler archive was NOT_RUN; byte identity was recorded. The compiler bundle/binaries are not duplicated in the scientific archive.
- RN/gradual-underflow environment is checked by the inherited research `admission::environment` before certification and queries. The production safe FP-provider gate is separate. The dense core has a crate-level `forbid(unsafe_code)` compile receipt; the existing MXCSR assembly remains outside that safe core.

## Actual command record

The following commands describe already completed work. They are not an instruction to reopen this closed run. `BUDGET.json` stores every actual command, start time, return code, duration and log hash, including failures.

```sh
python3 run.py toolchain_restore python3 extract_rust.py
python3 run.py rustc_version toolchain/bin/rustc --version --verbose
# Initial RED compile failed because bin/ was absent; its original log remains.
# After creating bin/, the same RED compile succeeded.
python3 run.py red_compile_after_output_dir_fix toolchain/bin/rustc --edition=2021 --test --cfg d3_red source/test_dense.rs -o bin/d3_red
python3 run.py red_contracts bin/d3_red d3_contracts --nocapture
python3 run.py safe_core_forbid_compile toolchain/bin/rustc --edition=2021 --crate-type=lib --emit=metadata source/safe_core.rs -o bin/safe_core.rmeta
python3 run.py green_compile toolchain/bin/rustc --edition=2021 --test source/test_dense.rs -o bin/d3_tests
python3 run.py green_contracts bin/d3_tests d3_contracts --nocapture
python3 run.py campaign_compile toolchain/bin/rustc --edition=2021 -O source/campaign_dense.rs -o bin/d3_campaign
python3 run.py frozen_eight_new_d3_cases bin/d3_campaign
python3 run.py --timeout-seconds 85 independent_saved_review python3 review_dense.py evidence/segments.jsonl --summary evidence/SUMMARY.json --output REVIEW.json --coefficients all --precision 80
python3 run.py saved_refusal_audit_and_figures python3 plot_saved.py
```

RED used the feature-absent scaffold and its14 tests at that snapshot. The current file has15 tests after the isolated query-rounding refusal contract was added; do not relabel a new current RED run as the original14-test result. The S1 archive preserves its fourteen-test source checkpoint; the original RED execution is identified separately by its retained log. Later source edits are not inferred to be byte-identical to the original compiler input. GREEN used the actual final15-test source. One imported old test was filtered, never executed.

The per-command timeout parameter was added before the first saved-output audit to allow up to85seconds for the full coefficient corpus within the unchanged aggregate120seconds. The audit used10.280914869999833seconds. `RUNNER_HISTORY.json` and `evidence/run_before_timeout_option.py` preserve that representation change. No scientific tolerance or target changed.

## Expected already-saved results

- Eight statuses COMPLETE;1188accepted segments;2360attempts;18880stage RHS;5266curve RHS;594analytic Jf;forbidden RHS0.
-15/15 new GREEN tests PASS; safe core and campaign compile exit0.
- Full oracle status `PASS_ALL_SAVED_DEFECTS_BOUNDS_AND_QUERIES`;285120 exact coefficients;1484queries;35400 inherited component bit checks.
- Saved refusal review1172records PASS; local norm24, certificate1148.
- Max query actual-error/radius0.4774119143317735; max endpoint component ratio0.7774548186902503; exact query rounding ratio0.875.
- New local subprocess total35.897308696992695seconds;closed,cap120,reset=false.

## Archive and publication verification

`checkpoint.py` creates each archive once and verifies ZIP CRC plus every payload size/SHA256. `ARCHIVE_MANIFEST.json` inside the archive contains per-file size/SHA256/Git blob. The archive's own size/SHA256 and provider receipts are necessarily external to that immutable archive.

S0 restored original bytes and all15payloads were checked. New S1/S2/S3 delivery uses local complete payload verification, Drive/Dropbox successful ACK and metadata size. A new remote whole-byte download/readback was not performed; do not call the provider ACK a full restore test.

Source publication is a non-force expected-SHA update on the existing D3 branch. Draft PR86 remains based on the preserved D2 branch. Science files, immutable raw archives, delivery metadata and branch HEAD are distinct identities; final receipts specify each.

## Scope of reproducibility and remaining obligations

The exact Fraction oracle is independent of the Rust Bernstein coefficient implementation. Decimal80 Riccati/contraction comparisons are numerical audit evidence with1e-68comparison slack, not formal interval proofs. Continuous certification follows the stated theorem and outward arithmetic under the checked floating-point/compiler assumptions. Generic callbacks, coupled BASS models, arbitrary uncertain initial data and external side-effect rollback are not established.

The build/source binding is `SOURCE_EXECUTION_IDENTITY.json`; it captured the frozen scientific source/binaries after the campaign. Later report additions and the runner timeout option are recorded separately. Final payload identities are in the S3 manifest.

For future genuinely changed work, first establish the native source/reservation/dirty/completion input and the exact new acceptance contract. Any separately authorized new computation needs a new linked ledger; never edit this closed ledger to reset time. No automatic next research node is created by this handoff.

## Remote CI disposition

Automatic GitHub process CI ran twice, separately from local scientific execution. The first exposed missing canonical registration/ledger linkage. The second (run40093427748 on science commit96aee1829d72ce1cbd6bcd0c0e48ebf3b1b37455) passed the D3 node/167-row append-only ledger and24 validator tests, then failed the whole-repository authority check on32 pre-existing missing references/143occurrences outside D3. All failed paths are unchanged by PR86. No checker or allowlist was weakened. The final delivery-only metadata commit does not repeat that same global failure; source CI remains explicitly FAIL, and production HOLD persists. See delivery/CI_FINAL_RECEIPT.json.
