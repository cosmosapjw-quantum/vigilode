# v3.4 Plot CRAG Audit

## Correctness

`N384_ZETA34_FROZEN_POLICY.png` shows all holdout audit-E errors below 1. The fixed threshold recommends 16 events and abstains on 11, but the profile contains no unsafe positives; safety discrimination is therefore untested rather than proven.

`N384_PREFIX_RESERVE_BREACH.png` isolates one semilinear event at 109 JVP vectors above the frozen 80-vector reserve.

`PREFIX_JVP_SCALING_ACROSS_PROFILES.png` shows stable median prefix work but a growing/nonmonotone upper tail across the deliberately different dimension/tolerance profiles.

## Retrieval

Automatic method-selection literature motivates fail-safe fallback and switching based on causal information, but it does not justify ignoring speculative work. Exponential Rosenbrock/Krylov work is adaptive, so a predicted fixed Krylov cost is not a theorem-level bound.

## Augmented adversarial interpretation

- Threshold mutation: forbidden; no retuning performed.
- Family mutation: breach is isolated to semilinear N384 but cannot be discarded because the family was predeclared.
- Cost mutation: cumulative 25% budget has large headroom; only the absolute 80 reserve fails.
- Safety mutation: there are no unsafe N384 labels, so the threshold is not adversarially challenged here.

## Claim status

SURVIVES: zeta34 remains a candidate safety witness conditional on obtaining it.
FAILS: full v3.4 policy promotion because budget_breaches != 0.
NEXT: replace predicted reserve semantics by an enforced pathwise prefix work cap before any new holdout.
