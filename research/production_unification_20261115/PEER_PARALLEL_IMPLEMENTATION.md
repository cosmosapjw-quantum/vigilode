# Peer-W execution and transaction checkpoint

Base: `bee5563c51264a7da19c519295bdda2fb317caef`. Runtime receipt:
[`evidence/peer_execution_receipt.json`](evidence/peer_execution_receipt.json).

The PPSW1-6b new-stage RHS refresh now runs through the existing explicitly
selected local Rayon pool. Its six inputs already depend only on completed stage
solves, so no equation, tableau, controller, tolerance, startup method or RODAS
stage dependency changes. Sequential execution preserves the original early-error
prefix semantics. The parallel callback contract requires values independent of
invocation order; user callbacks already have `Send + Sync` bounds.

Each parallel job owns a local `WorkCounters`. A job returns its callback result
inside a successful execution envelope, ensuring all six jobs finish before any
callback error propagates. The caller first accumulates every ledger and then
returns the first error in stage-index order. This preserves scalar callback
counter identities and avoids falsely claiming unexecuted work was performed.
On successful nonlinear, nonautonomous cases, all state bits and all work counters
match sequential execution. Actual callback overlap is observed with an atomic
active-job counter and remains bounded by the configured two threads.

Independent audit also reproduced foreign-session proposal acceptance, NaN-time
acceptance, and zero time advance. The same scoped checkpoint repairs them with a
private `Arc` identity in each session/proposal, finite and dimension-consistent
history checks, positive representable step ratios, finite stage input checks,
and finite strictly advancing step endpoints. Accept, reject and the nonfinite
`try_step` rejection path close the current generation; sibling proposals then
fail without mutating state or work. A fresh retry remains valid. Startup requires
each `h0` interval and the full `2 h0` interval to advance representably because
its forward collocation/history grid uses both intervals. This is a deliberate
input-domain guard, not a change to the Radau starter formula.

Executed evidence:

- Parallel contracts: baseline **2 passed / 3 failed**; patch **5 passed**.
- Boundary contracts: **0 passed / 5 failed** before repair, then all five passed.
  A separate sixth regression exposed nonfinite-rejection sibling reuse and
  failed before the additional one-line generation repair.
- Final same-source run: **171 passed / 0 failed / 1 ignored**: 123 library unit
  tests, 37 existing PPSW contracts, five parallel contracts, six boundary
  contracts. The ignored test is an existing optimized wall-time protocol.
- Exact final source/test SHA-256 identities match the tested remote files.
  `rustfmt --check` and `git diff --check` pass for changed files.
- Strict `clippy -D warnings` **fails** on eight existing lints in unchanged
  `source_owned_graph_schur.rs`. A diagnostic clippy run on the final source
  completes with those same eight warnings and no warning in changed files.

The remote run uses the repository's Rust/Cargo 1.94.1 toolchain, locked offline
dependencies, a detached worktree and a separate target directory. Raw RED and
GREEN logs are retained. No matched-achieved-error wall-time speedup is asserted;
the overlap tests include an artificial callback delay solely to detect execution
concurrency. Full production qualification remains HOLD. Dense Jacobian/common-W
factorization and the Newton-based startup remain explicit method properties.

Remaining scoped limitations: existing stage-linear-solve precharging on failure
is unchanged; panic-unwind work accounting is outside these Result-returning
callback contracts. Workspace-wide all-target release qualification remains a
separate obligation.
