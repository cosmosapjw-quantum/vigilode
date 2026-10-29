# N01 derivative-free causal research R5 -> next run handoff
2026-10-29. COPY BELOW INTO NEXT THREAD.

$physmath-research-loop $scispace $exa $wolfram
PROJECT=cosmosapjw-quantum/vigilode
BASE_PR=115
BASE_HEAD=8555b68a8053fc081be1195b0a40024d994d5798
RESEARCH_BRANCH=research/native-n01-jf-epoch-realign-20261029
NEXT_NODE=N01_DF_CAUSAL_HOMOTOPY_VALUE_ONLY_C5
AUTO_NEXT_NODE=false
PRODUCTION=HOLD
DAG_VIGILODE=7/23
DAG_REI=28/88
PROPOSED_SUCCESSOR=0/12 admitted
N00F=1/1 scoped
N01_DF_R5=1/1 scoped
N01_NATIVE=0/1 admitted

## Most important user-authored correction
Original solver motto: causal, parallelized, derivative/matrix-free augmentation of Rosenbrock/Runge-Kutta method with parallelized accelerated homotopy Newton-free root finder.
Preserve a hierarchy: (1) **RHS-only derivative-free original-stage proposal** with causal dependencies and parallel independent stage work, (2) structure/exact algebra where known, (3) *only when needed* JVP matrix-free Krylov fallback, (4) explicit dense oracle only for small independent validation. Do not mistakenly make a JVP-first solver.

## Directly inspect
- GitHub PR91 (derivative-free causal homotopy), PR92 (RHS-first default production facade), PR90 (accounting), PR74 (JVP-only coupled benchmark), PR93 (polyalgorithm), PR96 (negative authority), PR115 (current nonlinear owner).
- docs/superpowers/specs/2026-08-07-rust-parity-port-design.md
- docs/superpowers/specs/2026-08-10-vectorized-jf-path-controller-design.md
- docs/superpowers/specs/2026-08-08-unified-candidate-harness-design.md
- research/derivative_free_causal_homotopy_20261027/README_KO.md
- research/production_solver_closure_20261027/PRODUCTION_SETUP.md
- research/rnext07_model_epoch_20261003/PREREGISTRATION.md
- crates/vigilode-coupled/src/rhs_homotopy.rs, causal.rs, structural_mean_field.rs
- crates/vigilode/src/production.rs
- current branch DERIVATIVE_FREE_DESIGN_CORRECTION.md, R5_THEOREM_KO.md, exact_monotone_oracle.py, R5_RESULTS.json, BACKUP_RECEIPTS.json.

## R5 completed (narrow)
For nonnegative y, k_i>=0, tau>=0 and mass-action three-state model, diagonal-implicit-RK/BE stage z=y+tau*f(z) reduces exactly to a unique positive scalar b root F(b), F'(b)>=1. This is Newton-free, derivative-free at runtime, with |b-b*|<=|F(b)| and a source-specific physical p-error Lipschitz factor. All three original stage residual components equal (0,F,0), not (0,-F,0). Fraction exact 384 deterministic dyadic cases x9 = 3456/3456 PASS, fixed rational example exact. First oracle test sign bug exit1 preserved and corrected. Wolfram verified symbolic identities, exact source stage only. NO native Rust RODAS 8-stage performance/physical claim.

## Next selected C5 task, dependency
Build an opt-in source-owned value-only map implementing existing RHS-only homotopy/casual API **before** modifying PR115's supplied-JVP fallback. A callback at transient stage points must not change a *model generation* epoch. Model generation, time, state, parameter, physical weights, and acceptance clock each get explicit identity; no fake physical source hash from a caller.
For RODAS5P acceptance, compute **the actual 8-stage raw-U original residual**, physical output/stage bound, embedded time error and source-owned outward arithmetic. The scalar BE/diagonal-RK R5 lemma is a separate auxiliary comparator; do not transpose its positive carrier theorem to arbitrary negative SDIRK stages.
Bounded independent stage/cell parallelism must join/count partial failed work, and must not evaluate a truly causal stage before its ancestors. Reuse PR91 residual-dominance screening, per-station history reset and preexisting source/counter contracts.
Only JVP fallback if value-only candidate declines at its fixed budget; preserve warm starts, restarts, nondefault routes, counters, fatal priority and transactional output.
No tolerance changes or production promotion. Test same achieved error full-IVP against strong RHS-only sequential, including negative cases and entire fallback cost. Exact nonnormal and unstable Jacobian-free counterexamples must remain.

## Archive and receipts
ZIP VIGILODE_N01_DERIVATIVE_FREE_R5_20261029_EVIDENCE.zip (10652 bytes, 8 files)
SHA256 3dc8cfa72372f4885a14a6cfd00ca98a82eea2b7e16e6b1cb779e9fafeb9191b
Drive file 1kn8swYSyg85en80xSWDWSSau10tpwR4P: upload ACK, exact raw downloaded SHA256 match, CRC PASS.
Dropbox id:BSpOijBcT10AAAAAAEHVvA path /BASS_DERIVATION_DOSSIERS_20260912/VIGILODE_N01_DERIVATIVE_FREE_R5_20261029_EVIDENCE.zip: upload COMPLETE, metadata 10652; independent raw redownload NOT_RUN.
GitHub branch publishes R3 audit (historical focus), correction (superseding priority), actual R5 result, exact executable Fraction oracle and backup receipts as individual commits. A final archive does not equal separate cloud ACK for every stage.

## Return payload
Exact source SHA/tree, proof assumptions and original stage equations, code-path map and negative controls, native Rust command/exit and actual measured full-IVP error/work counters, parallel vs serial comparison under same physics/achieved accuracy, independent R6/C9 actual author IDs if available (else NOT_RUN), actual Google Drive/Dropbox provider ACK/readback and next copyable handoff.
