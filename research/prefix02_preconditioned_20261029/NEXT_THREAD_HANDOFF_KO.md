# VigilODE PREFIX02 → native left/right original-residual prefix handoff

$physmath-research-loop $scispace $exa $wolfram

PROJECT=cosmosapjw-quantum/vigilode
BASE=PR105 head 2a815e98a277c3b9ee807f6098f3d1950bf81985
PR104 native structural delivery=10854bc0bff81b6d3cf5cb0a44520ed9165c205f
PR103 CHEM6 sibling=1845e1d8bdfda222bf77637e8d38d5076743f238
UNIT=VIGILODE-PREFIX02
STATUS=PASS_SCOPED_STANDALONE_PENDING_NATIVE_AND_INDEPENDENT
WORKFLOW=18/20 (R6,C9 native independent NOT_RUN)
CORE_DAG=7/23 inherited scoped, 34 edges; REI_CENTRAL=28/88; PREFIX01=0/1; PREFIX02=0/1; PRODUCTION=HOLD

## 1. Restore and verify

Read PR106 (or current successor) `research/prefix02_preconditioned_20261029/{PREREGISTRATION.md,THEORY_KO.md,REPORT_KO.md,RESULTS.json,STAGE_LEDGER.json,NEXT_THREAD_HANDOFF_KO.md,DELIVERY_RECEIPTS.json}` and `standalone/`. Compare SHA256 in SOURCE_MANIFEST.json and ZIP; do not infer production status from standalone PASS.

From standalone directory with Rust 1.94.1:

```
cargo fmt --all -- --check
cargo test --locked --offline --all-targets
cargo clippy --locked --offline --all-targets -- -D warnings
cargo run --locked --offline --release --bin precond_bits > evidence/bit_probe.csv
python3 evidence/verify_bits.py
cargo run --locked --offline --release --bin precond_bench > evidence/bench.csv
python3 evidence/analyze_bench.py
```

Expected scoped: 25 Rust tests, 360 exact Fraction inequalities, 7 paired blocks. CPU timings are environment-specific. Preserve failures and actual achieved errors. Do not claim 2520 standalone solves are full-IVP.

## 2. Math to native map

Right: `B_R=A M^-1`, `x=x0+M^-1 V y`. Left: `B_L=M^-1 A`, `x=x0+V y`. Original residual `b-Ax`, NOT `M^-1(b-Ax)` and NOT Givens projected residual, owns admission.

For exact represented J and `σ≥0`, `μ_w(J)≤m`, `σm<1`, `K=1/(1−σm)` bounds inverse norm. Outward original `ρ` must satisfy `ρ≤τ_orig` AND `Kρ≤ε_stage`. Any matrix-free callback/source uncertainty not enclosed is a separate owner, so mark nonrigorous when absent.

Native PR104 `crates/rodas5p-krylov/src/gmres_into.rs` currently has `solve_gmres_into_right_monitored`, but reconstructs and least-squares solves every column. For actual cost reduction, add opt-in Givens projected-trigger BEFORE prefix backsolve/candidate construction and only then evaluate original residual. Do not silently alter public API, `GmresIntoOptions`, `ResidualAccounting`, left/right `PreconditionerSide`, `residual_scale`, warm starts, restart, `WorkCounters`, capacity, partial failure, callback counts, or default routing. Keep existing tests and Q-API failures.

## 3. Source/epoch and negative controls

Bind model/source/clock/norm/epoch before every operator and before output commit; reject stale callback. For native matrix-free JVP, acquire an independently validated `η_eval` and actual physical norm transport or claim only floating original-residual PASS. Require zero-shift, nonnormal, left distorted-norm false positive, underflowed preconditioner, wrong projected residual, failed/partial callback, nonfinite output, restart, preconditioner singular, source mutation, stale epoch, and no output mutation on refusal/error.

## 4. Full-IVP and release

Native source pin and executed binary SHA before timing. Seven ABBA/BAAB blocks with fixed controller, tolerances, same source, same requested grid and achieved error; compare positive and negative controls including fallback cost. Report JVP, MGS, QR, preconditioner and certificate work separately. Preserve PR104 2460 full-IVP historical evidence as historical, not newly executed.

PR105 `research-process` CI run40806732910 failed because PREFIX01 has no true preregistration and no append-only ledger row; do not backdate a preregistration. New PREFIX02 has genuine prereg before first execution and new ledger row. Inherited Q-LEDGER3/3, PR-N01/Q-LINT3/3 and full authority failures are retained. Main merge, force push, production promotion prohibited.

Return actual native source/tree SHA, commands/exit, source/clock/rate identity, error enclosure, matched-achieved-error benchmark raw data, new DAG completed/total, GitHub PR, Drive/Dropbox ACK/restore verification, and a copyable next handoff.
