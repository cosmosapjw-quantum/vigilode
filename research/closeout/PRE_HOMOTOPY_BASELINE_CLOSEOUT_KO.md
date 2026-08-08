# RODAS5P Rust pre-homotopy baseline — final closeout

Date: 2026-08-07 Asia/Seoul
Baseline ancestor: `rust-parity-v0.1.0-alpha1`
Final tag: `rust-pre-homotopy-v0.2.0-alpha1`

## 1. 종료 범위

이 closeout은 다음 pre-homotopy 작업을 닫는다.

- Python/SciPy와 분리된 Rust-only numerical baseline;
- same-kernel GMRES/LGMRES/fixed-left GCRO-DR;
- protected sequential/adaptive RODAS5P;
- structured block and SABR5P fallback;
- strict Fair A/B trace, budget and true-residual contracts;
- allocation-free shifted operator;
- reusable Krylov workspaces;
- R10-A paired and allocation audit.

Homotopy/continuation source는 포함하지 않는다.

## 2. R10-A numerical evidence

Benchmark source HEAD: `2d3fcaffa688e2cb99d7e79d9f59536fae4dae06`.
Final tag는 이후의 lint-equivalent API cleanup, documentation, receipts and packaging을 포함한다.

- paired rows: `252`;
- baseline failures: `0`;
- current failures: `0`;
- total operator-work delta: exactly `0` in every paired row;
- maximum absolute solution-error delta: `5.294e-23`;
- GMRES/OFF wall ratio median: `0.9918`;
- LGMRES/OFF wall ratio median: `0.9907`;
- GCRO-DR/STAGE wall ratio median: `1.0151`;
- GCRO-DR/PERSISTENT wall ratio median: `1.0056`.

Allocation-audit medians show GCRO-DR/OFF at about `4.59x` the allocation-event count and `1.97x`
the allocated bytes of GMRES/OFF. This identifies projected/recycle algebra as deferred tuning work; it does
not establish a general algorithmic no-go.

## 3. Final disposition

### PROMOTE

- Rust workspace as the immutable pre-homotopy research baseline;
- allocation-free shifted operator;
- reusable solver workspaces;
- solver-independent certification/budget contracts;
- R10-A raw results and receipts.

### HOLD / DEFER

- default solver change;
- unconditional stage/persistent recycling;
- GCRO-DR production performance claim;
- `(m,k)` optimization;
- adaptive recycle admission/reset;
- sparse/MPI/GPU/DAE generalization;
- BDF/Radau final ranking.

The deferred items will be evaluated jointly with the homotopy branch rather than optimized in isolation now.

## 4. Branch transition

The next active branch must fork from `rust-pre-homotopy-v0.2.0-alpha1`. The current numerical baseline,
R10-A results and receipts are read-only comparison evidence. Homotopy code must not be backported into this tag.
