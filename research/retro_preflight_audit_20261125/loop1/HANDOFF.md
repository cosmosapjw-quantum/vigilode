# Loop 1 completed: input admission and arithmetic ownership
Two reproducible RED failures (exit 101) in original implementation: Scaled
zero bypass; batch buffer mutated before invalid argument. GREEN core 6,
integrators 7 plus affine event 13 passed, 0 failed; one timing test ignored.
Centralized safe existing binary64 environment probe in core, check before
dictionary OnceLock acquisition, strict scaled domain, prevalidate full batch
before touching output, cap prepared matrix before inverse gap traversal.
Arithmetic precondition after acquisition remains per-thread; runtime worker
failure requires discarding outputs (no claim of full transactional rollback).
Clean worktree removal complete, original branch refs and dirty jobs retained.
Next loop: reuse faer batch workspace, compare exact sparse residual verifier,
verify numerical equivalence and failure behavior. No M0 solver run.
