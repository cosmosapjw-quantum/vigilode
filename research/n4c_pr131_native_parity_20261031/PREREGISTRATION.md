# N4C exact PR131 source-matched native parity: preregistration

Research-only sibling to [N4B #146](https://github.com/cosmosapjw-quantum/vigilode/pull/146) and [offline N4C #147](https://github.com/cosmosapjw-quantum/vigilode/pull/147). This separate draft branch is based on exact **PR131 HEAD b289d8e8165bf7d5b1d3255c9a02e89bc4a20b9e**, not on an inferred snapshot. The tests are reused **bit-for-bit** from PR147 existing blobs (rank2 source `f0903a58a551013e4c6ea51453919cb411b61602`, tableau `97ac7f3e261d6bd8d6d7169b17e4f96d8c56c229`).

**Hypothesis:** Original source-owned N4C rank2 RODAS5P eight-stage raw-U and failed-step transaction tests compile and pass against actual PR131 native `sequential.rs` blob `4153e286a2ac756252978d5d77cf1abe0e6a4e0c` and `problem.rs` blob `cbb0bf22480541ee4c92eca32bbe41574e0eee7e`. Test 13/13 is **historical offline-only** until GitHub CI checks THIS checkout.

**Observable pass:** `cargo test --locked --offline -p rodas5p-integrators --test n4c_native_rank2_source_owned -- --nocapture` returns exit 0 and 13 tests pass. Second separate original PR131 frozen-source regression must pass. Byte-identity gate checks original native source, coefficient snapshot, Cargo.lock, unchanged production, and inherited test blobs. Strict Clippy and rustfmt remain separately reported; no leniency for errors.

**Scientific limit:** Real original all-eight residuals and refusal are test-only, not a rigorous stage forward-error bound, global physical two-window error, matched achieved-error cost comparison, production authorization, default solver, or independent C9 reviewer. Do not amend N4B receipts; no merging, tolerance changes, C10 promotion, or DAG advancement.

Statuses before fresh CI: N4C=PR131_SOURCE_MATCHED_NOT_RUN; official VigilODE 7/23; REI 28/88; successor 0/12; RVJ 3/8 + N4 PARTIAL; C9 distinct author 0/1; production HOLD.

## FAIL classification, initial run 41101087887 (immutable)

The first CI attempt at commit `d4f749bb948883559001d1c4a36277bf020b51c1` **FAILED** before any Rust scientific test: `fatal: Not a valid commit name b289d8e8165bf7d5b1d3255c9a02e89bc4a20b9e`. Root cause was `actions/checkout@v4` default `fetch-depth: 1`, making the parent-in-commit-graph test impossible locally. The same runner log also confirmed `rust-toolchain.toml` overrides the default to Rust 1.94.1, despite installing 1.99.1, so this workflow is corrected to use explicit `cargo +1.99.1`/`rustfmt +1.99.1` and `fetch-depth: 0`. This is **CI/provenance setup failure**, not source-matched native success, and no code/tolerance/physics criterion was relaxed. The former run remains failed and is never relabeled PASS.
