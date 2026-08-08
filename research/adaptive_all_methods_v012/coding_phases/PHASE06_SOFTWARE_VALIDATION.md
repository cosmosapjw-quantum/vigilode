# Phase 6 — Software Validation

## TDD evidence

Every production change was preceded by a focused failing test and followed by focused and full-suite green runs. The durable receipts are under `coding_receipts/` as `TASK*_RED.log`, `TASK*_GREEN.log`, and `TASK*_FULL_GREEN.log`.

Covered contracts include:

- controller formula, validation, and PI-memory transaction;
- unchanged fixed-step and legacy RODAS adaptive behavior;
- nonuniform BDF2 quadratic-polynomial exactness;
- BDF startup-to-order-two transition;
- rejected BDF history rollback;
- Radau rejection and requested-output completion;
- Homotopy fast acceptance and forced fallback;
- all candidate IDs present in the unified report;
- exact T1/T4 scientific checksum identity.

## Pre-closeout static gate

The strict command

```bash
cargo clippy --workspace --all-targets --profile measurement --offline --locked -- -D warnings
```

passes after two test-only lint corrections (`contains` and struct-update initialization). No numerical production path was changed by those corrections.

## Failure semantics

- Nonfinite configuration and step-size data return typed errors.
- Rejected trials do not commit solution, output-observer, BDF history or recycle state.
- Trial work is retained in the counters.
- Maximum attempts and minimum step size terminate rejection loops.
- Fixed-step APIs remain additive and backward compatible.
