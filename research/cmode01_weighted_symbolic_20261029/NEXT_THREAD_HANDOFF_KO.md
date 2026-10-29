# Next thread / local Codex handoff: VIGILODE CMODE01 -> native-qualification
DATE=2026-10-29
REPO=cosmosapjw-quantum/vigilode
PARENT=PR106 c6cba2b62629e819dd163325c7c9fb9bb5e3080c
BRANCH=research/cmode01-weighted-symbolic-certificate-20261029
WORK=CMODE01 weighted-lognorm certificate with affine symbolic-cell reuse
DAG_ORIGINAL=7/23 inherited scoped, 34 edges; DELTA=0
REI=28/88 unchanged
AUXILIARY=8/10 scoped; native independent admission=0/1
PROPOSED_SUCCESSOR=0/12 admitted
PRODUCTION=HOLD

1. Read CMODE01 PREREGISTRATION.md, THEORY_KO.md, REPORT_KO.md, PRODUCTION_DAG.json, STAGE_LEDGER.json, this handoff, and source TAR/ZIP hashes. Distinguish parent PR106's standalone preconditioned GMRES from CMODE01's separately executed synthetic midpoint IVP; neither is native RODAS production.
2. Reproduce with exact source archive in a clean working tree using Rust 1.94.1: cargo fmt --all -- --check; cargo test --locked --offline --all-targets; cargo clippy --locked --offline --all-targets -- -D warnings; cargo run --locked --offline --release --bin probe > evidence/probe.csv; cargo run --locked --offline --release --bin trajectory > evidence/trajectory.csv; cargo run --locked --offline --release --bin benchmark > evidence/benchmark_raw.csv; python evidence/oracle.py; python evidence/plots.py. Archived results: 8 Rust tests, 560 Fraction inequalities, 256 analytic trajectories, max error 2.0699365877591136e-8, bit-identical arms, 7 paired blocks per n; ratios n128 1.1823386213, n512 1.1787810814. Native full-IVP performance not established.
3. THEORY FIRST: validate resolvent bound in physical weighted norm and interval-inclusion proof for J(theta)=J0+theta E. Counterexample nonlinear theta(t)=.02+.25 sin²(pi t), endpoints safe and interior escapes. Prove or explicitly reject whole-slab confinement; never use mere endpoint membership. Treat itinerary as verified finite-state guard, NOT topological conjugacy.
4. CODE SECOND: inspect native crates/rodas5p-krylov/src/gmres_into.rs, crates/rodas5p-integrators/src/rodas5p_fast_dense.rs, adaptive controller, physical BASS/REI source contract, epoch/clock ownership. Introduce opt-in OPAQUE, source-bound certified MuUpper not caller-supplied untrusted f64. Use actual physical norm conversion; original residual authority and backward/forward budget; preserve left/right preconditioner semantics, Q-GIVENS, Q-API, warm starts, rollback and work counters. Do not rewrite standard methods or default routing.
5. Validate with PHYS-MATH audit THEN PHYS-MATH-CODE audit, negative controls including source mutation/epoch switch, zero/infinite weight, nonnormal, failed JVP, nonlinear interior excursion, stale cell, wrong projected-only trigger and output unchanged on refusal. Independently bound matrix-free callback error or decline rigorous claim. Preserve negative evidence.
6. Same-physics, same time grid, same controller/tolerances, same physical error and output, matched achieved-error external comparator. Use at most THREE total code/benchmark adjustment loops PER NODE (CMODE01 has already used ONE). Record all timings and row scans, JVP, MGS, certificates, interval work, allocation and fallback losses; no claim of full production speedup from component speed.
7. Original VigilODE 23-node DAG and independent PR103-106 source pointers must remain immutable unless explicit admission. Avoid merge, force push, production release, changing source-of-truth. Per stage: commit & read back GitHub plus actual Drive+Dropbox ACK; absent provider ACK mark NOT_DONE. Update counts separately: original completed/23, auxiliary accepted/1, proposed successor admitted/12, and independent REI completed/88. Return actual SHAs, exit codes, reproducible evidence, close outstanding Q-LEDGER/Q-API/Q-GIVENS and 8-suite/10-interaction gates only when independently qualified.

Expected next minimum DAG-correct node: N03, native opaque weighted certificate preceded by N01 source and point-J original-residual verification. Not production admission.
