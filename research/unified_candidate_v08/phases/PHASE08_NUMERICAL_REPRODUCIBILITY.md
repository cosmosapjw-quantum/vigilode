# Phase 8 — Numerical reproducibility

## Deterministic execution

Independent cases run inside local Rayon pools. Stage and timestep order inside each case remains sequential and deterministic. Rows are canonically sorted before scientific hashing.

## One-thread/four-thread identity

Canonical executions:

```text
threads  elapsed   max RSS
1        4.17 s    45,236 KiB
4        3.11 s    49,008 KiB
```

The scientific checksum is byte-for-byte identical:

```text
06054fc562d578a240c41de29445aaf8c66a2aa427cc879a10f3905f94d9e42c
```

Observed end-to-end speedup is approximately 1.341x, with parallel efficiency approximately 0.335 at four threads. Timing fields are intentionally excluded from the scientific checksum.

## Canonical artifacts

- `results/CANONICAL_EXACT_T1.json`
- `results/CANONICAL_EXACT_T4.json`
- `tables/THREAD_SCALING.csv`
- `receipts/INDEPENDENT_RESULT_AUDIT.log`

The full JSON files differ because elapsed times and thread counts are real measurements; their scientific projection is identical.
