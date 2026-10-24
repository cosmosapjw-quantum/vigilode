# Reproduction and preservation

The completed D4 runs are closed. Do not rerun completed D2/D3/D4 contracts, campaigns, benchmarks, NCP or V3 harmonic when resuming. Reuse source identity, logs, REVIEW.json and the raw phase archive. run.py refuses closed BUDGET.json. Do not reset or reopen that ledger.

## Exact executed source

D4 S1 commit555a93e5bffbed4b488ee0496e73d8095b08eab4 contains driver/fixture/tests/trace generator and independent oracle used. S2 result publication8dc905eb56682be5fd59ee50971704abd5a150f0 adds exact logs/results. SOURCE_IDENTITY.json and REUSED_SOURCE_PINS.json bind contents; runtime recovery retained original code hashes. Plot generation is later saved-data processing, not a solver rerun.

Toolchain: Rust1.94.1 e408947bfd200af42db322daf0fadfe7e26d3bd1, LLVM21.1.8, x86_64-unknown-linux-gnu; Python3.12.14. No Cargo/network dependency resolution. Official archive matches provided historical SHA256294b3d81fa72e62581276290c60c81eb8b58498d333d422ca1dfc432877d0c40; PGP signature NOT_RUN.

## Actual command order (documentation, not an instruction to rerun)

1. python3 restore_toolchain.py
2. toolchain/bin/rustc --version --verbose
3. toolchain/bin/rustc --edition=2024 --crate-type=rlib source/fixture.rs -o bin/libd3_fixture.rlib
4. toolchain/bin/rustc --edition=2024 --crate-type=rlib -F unsafe-code source/driver.rs --extern d3_fixture=bin/libd3_fixture.rlib -o bin/libd4_effects.rlib
5. toolchain/bin/rustc --edition=2024 --test -F unsafe-code source/tests.rs --extern d3_fixture=bin/libd3_fixture.rlib --extern d4_effects=bin/libd4_effects.rlib -L dependency=bin -o bin/d4_contracts
6. bin/d4_contracts --nocapture
7. toolchain/bin/rustc --edition=2024 --crate-type=rlib --emit=metadata -F unsafe-code source/fixture.rs -o bin/forbidden_fixture.rmeta (EXPECTED exit1 at inherited MXCSR)
8. toolchain/bin/rustc --edition=2024 -O -F unsafe-code source/trace_corpus.rs --extern d3_fixture=bin/libd3_fixture.rlib --extern d4_effects=bin/libd4_effects.rlib -L dependency=bin -o bin/d4_trace_corpus
9. bin/d4_trace_corpus evidence/traces.jsonl
10. python3 review_traces.py evidence/traces.jsonl REVIEW.json
11. python3 plot_saved.py

All were wrapped by run.py with unique log names and measured in BUDGET.json. Ordinary commands had45s child timeout; toolchain restore90s, within120s aggregate. No command timed out. Expected unsafe rejection and intentional panic stderr are preserved. Total25.906088458010345s/11entries, closed.

## Expected results and raw evidence

22/22 contracts; two new stage hook trials10RHS/9solves/10checks; no trajectory campaign. All16,384 words from8^4 actions/caps0–3:65,536rows; independent3,407,872comparisons; failure0; three semantic mutants distinguished. Byte-exact trace68,074,488B SHA2563f9948f68c3da234859519788b34806145ee1f95dc69aa3302e74ba17fc9273b is in the S2/S3 archives. REVIEW.json is fully published. Excluded compiler/binaries are regenerable and not science evidence.

## Delivery and review

Read delivery/S0–S3_PUBLICATION.json and corresponding PROVIDER_ACK.json for real commit/archive/provider identity. The immutable archive predates its own upload ACK; later Git/final receipts record that ACK separately, never by rewriting an archive. Local CRC/all payload hashes and provider ACK/size are verified; whole remote-byte restore NOT_RUN.

Research node CI runs only metadata validators and authority-reference checks. Existing global authority failure is preserved; no checker/allowlist changes and no old science workflow reruns. Final source CI status is recorded after actual observation. Final delivery-only metadata can skip the identical global check while keeping the earlier failure visible.

## New work resume prerequisite

Current native owner exact source/reservation/dirty/completion input remains UNRESOLVED_INPUT_DEPENDENCY. Production HOLD/global completion null. Reuse its existing Transaction/Session/output hooks and integrate only an identified missing behavior. New mathematical/producer/sink extensions need their own scoped contract; do not create another norm-only child automatically.

Actual S3 metadata CI: run40138539924, validator24 PASS and node/append-only ledger168 PASS. Authority FAIL38distinct/152occurrences = inherited32/143 plus D4 external repository objects6/9. All six external identities are bound to their actual repositories in EXTERNAL_SOURCE_BINDINGS.json. Checker/allowlist unchanged; no claim of CI green. Delivery-only final metadata commit skips repeat CI and retains this exact failure.
