# v3.5 Fresh Enforced-Budget Safety Holdout Contract

## Purpose
Evaluate the already-frozen v3.5 transactional prefix-budget semantics and the already-frozen v3.3 zeta34 threshold on one new intermediate profile. This profile is not N=2048 and does not authorize active switching.

## Frozen upstream policy
- persistence: k=3
- absolute speculative prefix cap: B_abs=80 JVP vectors
- cumulative speculative fraction: delta=0.25
- zeta34 recommendation: recommend iff a completed, non-exhausted prefix has zeta34 <= 13.39706618860016
- runtime E-K continuation: forbidden
- active switching: forbidden

## Fresh profile (predeclared before output)
- profile name: enforced-budget-holdout-320
- N=320
- atol=1.0e-7
- rtol=1.0e-5

Rationale: N=320 is the arithmetic midpoint of the already-consumed N=256 and N=384 profiles. The tolerance pair is the natural central 1e-7/1e-5 scale and was fixed before any N=320 output.

## Structural hard gates
1. six family shards complete successfully;
2. no row has actual_prefix_jvp_vectors > budget_cap_jvp;
3. budget_breaches = 0;
4. every budget-exhausted row has prefix_succeeded=false, zeta34=null, audit_full_e_completed=false, and all completed speculative work retained in prefix_work;
5. R-JF remains the only committed method; runtime_full_e_continuations=0 and switching_active=false;
6. explicit Jacobian builds, direct factorizations, and Newton iterations remain zero on the primary JF path.

## Frozen safety/coverage gates
Among completed non-exhausted prefixes:
- unsafe recommendations = 0;
- recommendations >= 6;
- distinct recommended families >= 3;
- max single-family recommendation fraction <= 0.50.

## Discriminating-evidence gate
For promotion evidence rather than mere structural correctness, the fresh profile must contain at least one audit-full-E inadmissible eligible event. If it contains none, the result is INCONCLUSIVE_FOR_SAFETY_PROMOTION rather than a policy failure.

## Forbidden
- retuning B_abs, delta, tau, or k;
- family-specific thresholds;
- using N=320 output to alter the runner before verdict;
- N=2048 execution;
- active switching or runtime E-K activation.
