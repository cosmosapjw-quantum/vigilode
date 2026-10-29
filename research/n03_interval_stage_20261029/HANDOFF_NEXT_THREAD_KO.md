# VigilODE next-thread handoff, N02 recovery → N03 conditional interval source stage

$physmath-research-loop $scispace $exa $wolfram

REPO=cosmosapjw-quantum/vigilode
PARENT_PR110_HEAD=6c99864b9826070adfaf4da759efa5328f54f45a
N03_PR=112
N03_BRANCH=research/n03-physical-interval-stage-certificate-20261029
N03_PREREG=d90f28dc3f6465882e8ce294a78adfdf9cfb96f4
N02=16/20 scoped; R6/C9 native independent NOT_RUN
VIGILODE_ORIGINAL=7/23 inherited scoped; REI_CENTRAL=28/88
PROPOSED_SUCCESSOR=0/12 admitted
PRODUCTION=HOLD

Read N03 PR source, THEORY_KO.md, REPORT_KO.md, RESULTS.json, stage ledger, receipts and full local SOURCE ZIP; verify SHA/CRC. Compare PR111 `2357135220...`, PR108 `611e697a...`, PR109 N01 `2d430a7a...`, parent PR110 terminal `6c99864b...`. Never conflate source identities or mark 0/12 as admitted from scoped tests.

Native code: `crates/rodas5p-krylov/src/stage_interval_certificate.rs`, two new native integration test files, additive `lib.rs` export and N03 focused workflow. The API takes user-provided entrywise interval J/b and token; it proves conditional original-residual / p-weighted error for ANY enclosed represented stage only. In particular it **does not authenticate** owner-specific J, electron feedback, rates, temperature/ion inventory, actual physical33, physical clock, JVP evaluation errors, or global IVP.

Theorem: mu_up=max_i(U_ii+sum_{j!=i}max(|Lij|,|Uij|)*w_j/w_i). If sigma>=0,1-sigma*mu_up>0, positive fixed w,p: rho_up from `b-x+sigma Jx` outward interval; C=max(w/p), K=1/(1-sigma mu_up), stage error <=C K rho_up. Admit only if original rho_up<=tau_orig AND forward upper<=epsilon_stage, same source/clock/norm/epoch before and at output commit; owner must serialize mutation. Projected Givens only candidate trigger.

Reproduction:
```
export PATH=/mnt/data/rust-1.94.1-prefix/bin:$PATH
cd standalone
cargo test --locked --offline --all-targets
cargo fmt --all -- --check
cargo clippy --locked --offline --all-targets -- -D warnings
cargo run --locked --offline --bin probe > ../probe.csv
python3 ../oracle_fraction.py
```
Expected local 11/11 tests and Fraction 768/768; numerical external source is synthetic only. For real native checkout use:
```
cargo +1.94.1 fmt --check -p rodas5p-krylov
cargo +1.94.1 test --locked -p rodas5p-krylov --test n03_stage_interval_contracts --test n03_n02_guard_integration -- --nocapture
cargo +1.94.1 clippy --locked -p rodas5p-krylov --all-targets -- -D warnings
```
Native focused run IDs 40853674030 initial fmt fail, repaired 40854021844; check current HEAD outcome not report text alone. Broad authority and MPI.h benchmark failures remain separate, do not suppress. Independent-author R6/C9 still required.

Next meaningful scientific step: establish one *actual owner* frozen source/inventory/clock/positive physical weight and enclosure over the whole stage, with independent native JVP/source error bound. PR265 refuses direct BASS/REI pilot mapping due initial T+ and He4/H1 source identity mismatch. Do not synthesize an owner certificate, merge, force push, or promote to production. Only then repeat N03 interval membership and attempt physically matched full-IVP ABBA/BAAB, N04 whole-slab cells, 8 suites/10 interactions. No further optional meta-audit loops before genuine source construction.

At the end return exact PR head/tree/source SHA, commands/exits, native CI, standalone fraction oracle, source membership proof and missing input, DAG counts, provider ACKs, next minimum action. Per-stage GitHub receipts versus aggregate binary backup are separate.
