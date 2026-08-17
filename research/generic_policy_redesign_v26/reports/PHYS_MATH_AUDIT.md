# P1-01R v2.6 Attempt-Causal Trace — PHYS–MATH AUDIT

## Scope
Read-only R-JF attempt tracing only. No E-K evaluation, no prefix evaluation, no active switching, and no N=2048 execution occurred in this node.

## Governing causal contract
A policy event observed after accepted committed R-JF step `j` can only act on the controller's **first subsequently proposed trial**. If that trial is rejected, the eventually accepted-grid step `j+1` is future information and is not an admissible authority record for the speculative action.

### Proposition — accepted-grid look-ahead failure
Let `A_j` be the event decision formed after committed accepted step `j`, let `q_1` be the first controller proposal after `A_j`, and let `q_*` be the first subsequently accepted proposal. If `q_1` is rejected and `q_* != q_1`, then any replay that prices or evaluates the speculative action at `q_*` instead of `q_1` is not runtime-causal.

**Proof.** At the decision time immediately after accepted step `j`, `q_1` is available only after the controller forms its next trial proposal; `q_*` is known only after observing rejection of `q_1` and any intervening trials. Therefore `q_*` is measurable with respect to a strictly later information sigma-algebra than the decision. QED.

## Observed counterexamples
The exact frozen v2.5 k=1 policy contains one causal miss in each dimension:

- N=128, `nonautonomous-stiff-forcing`, decision accepted step 10:
  first next `h = 0.010883...`, rejected with error norm 1.336366...;
  eventually accepted `h = 0.008707... = 0.8 h_first`.
- N=512, same family and decision step 10:
  first next `h = 0.010871...`, rejected with error norm 1.366993...;
  eventually accepted `h = 0.008697... = 0.8 h_first`.

The development k=2 persistence diagnostic eliminates these two but reveals two stronger N=512 semilinear misses: the first proposed trial fails/rejects and the eventual accepted `h` is only `0.2 h_first`.

The k=3 persistence diagnostic has zero rejected-first-target events in both dimensions on the current development corpus:
- N=128: 0 / 26 events;
- N=512: 0 / 30 events.

This is evidence for k=3 as the current development survivor, not a theorem of future safety.

## Numerical/physical invariants
- R-JF accepted rows from the new attempt tracer are exactly equal to the v2.5 raw R-only shard rows in `t_start`, `h`, transition level, embedded error, RHS evaluations, JVP vectors, and linear matvecs.
- N=128: 431 attempts = 413 accepted + 18 rejected.
- N=512: 447 attempts = 426 accepted + 21 rejected.
- Rejected work is retained rather than inferred.
- Explicit Jacobian build, direct factorization, and Newton counts remain zero on the protected primary lane.
- No E-K/prefix quantity is used in the event trigger.

## Claim ceiling
Allowed: accepted-grid replay is a useful retrospective development approximation, but is not a runtime-causal cost/safety authority when rejected next proposals exist.

Not allowed: k=3 is now a safe/promoted re-entry policy. It has only survived this causal-target test on already-seen N=128/N=512 development corpora.
