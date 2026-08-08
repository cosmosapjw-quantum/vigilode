# Phase 8 — Numerical Reproducibility

## Deterministic reports

Canonical one-thread and four-thread runs contain 120 successful rows each and have the identical scientific checksum

```text
3147e9e891c7da462eb07deacf7caee800875dbe067649820c77757cd108bc0e
```

The local Rayon pool changes only independent-case scheduling. Rows are sorted by stable record key before the checksum is computed.

## Throughput observation

```text
T1 scientific suite  0.072130858 s
T4 scientific suite  0.024106208 s
ratio                2.992210886x
```

This is case-level throughput, not solver-internal stage or Newton parallel speedup. It is descriptive because no repeated randomized timing campaign was run in this cycle.

## Durable inputs and outputs

- code and dependency lock are inherited from the Rust 1.94.1 pure-Rust baseline;
- all result JSON/CSV files are committed;
- no external BLAS/LAPACK backend is introduced;
- analytic references, output grid IDs, candidate IDs and tolerances are serialized in every row.
