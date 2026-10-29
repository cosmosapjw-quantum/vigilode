# VIGILODE-PREFIX02: preconditioned original-residual prefix, 2026-10-29

## Frozen authority before computation
- Owner `cosmosapjw-quantum/vigilode`, base PR105 `2a815e98a277c3b9ee807f6098f3d1950bf81985`.
- Related native PR104 `10854bc0bff81b6d3cf5cb0a44520ed9165c205f`, sibling chemical6 PR103 `1845e1d8bdfda222bf77637e8d38d5076743f238`.
- Preserve core 23 nodes / 34 edges / 73 paths / 8 suites / 10 interactions; inherited scoped 7/23; REI central 28/88; production HOLD.
- PR105's missing prereg/ledger and failed CI are historical facts. This is a NEW preregistered research node, not a retroactive preregistration of PR105. No root native solver, API, budget or default dispatcher modification.

## Question, domain, acceptance
Derive distinct right and left preconditioned Arnoldi relations and an original-equation residual acceptance contract. Implement a self-contained Rust1.94.1 testable prototype with invertible represented diagonal preconditioner, exact represented dense J, frozen source/clock/norm/epoch, positive weights, sigma>=0, and `1-sigma*mu_w(J)>0`.

- Right: `B=A M^-1`, `x=x0+M^-1 V y`; left: `B=M^-1 A`, `x=x0+V y`. Original residual `b-Ax`, not projected or left-preconditioned residual, owns acceptance.
- Outward interval original residual and weighted inverse bound must satisfy unchanged declared `tau_orig` and `stage_budget` simultaneously. Source/epoch mismatch, invalid/nonfinite preconditioner, nonpositive inverse margin, invalid scale, false projected convergence and partial/nonfinite operator must refuse without output mutation.
- Two independent verification methods: Rust unit/integration tests and Python exact `Fraction` recomputation from native binary64 bits; Wolfram algebra check. No claims for real matrix-free callback evaluation error, physical rate boxes, full30/33 or whole-slab event.
- Test identity, left, right, warm start, zero sigma, nonnormal matrix, wrong source/epoch, refusal, exact solution, overflow, and preconditioned norm false positive. Preserve first failure logs.
- Benchmark seven paired blocks at same input/tolerance and report achieved error, JVP/preconditioner work, rejected checks and wall time; distinguish prototype diagnostic from production speedup.
- Independent-author R6 and C9 require actual native delegation. If unavailable, mark NOT_RUN rather than simulated.

## Execution and backup
- Source/protocol must be committed to isolated research branch before first test. Bounded CPU tests and paired microbench only; avoid long production sweeps.
- Git checkpoint after each selected R1..R10, C1..C10 stage, including explicit NOT_RUN stage records. For cloud backup, prefer provider ACK for a final hash-manifest ZIP containing all 20 snapshots; never claim per-stage Drive/Dropbox ACK unless individually returned.
- Ledger: append one new valid row only after numeric results exist; never edit earlier lines or authority allowlist. If connector/file transport prevents cloud upload, retain Git/local hashes and report blocker.
- Success: mathematical conditional theorem, runnable code/tests, source-locked results, 20 stage receipts, reproducible handoff. Production remains HOLD.
