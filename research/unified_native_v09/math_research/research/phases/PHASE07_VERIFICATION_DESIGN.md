# Phase 7 — Minimal Decisive Verification Design

## V-001 — Dense Newton contract

1. Claim: one Newton engine can expose identical residual/Jacobian/factorization counters for BDF and Radau.
2. Inputs: scalar nonlinear, noncommuting mass-matrix affine, and nonnormal vector problems.
3. Correct result: direct-oracle residual tolerance, no hidden evaluation, transactional failure.
4. Competitor: solver-specific nonlinear loops with different stopping criteria.
5. Pass: identical external certificate semantics and complete ledgers.
6. Kill: hidden RHS/Jacobian calls or false convergence.

## V-002 — BDF1/BDF2 anchors

- Pass: fixed-step observed orders 1 and 2; BDF2 startup explicitly charged; scalar stiff decay; noncommuting mass-matrix exact affine parity.
- Kill: root/history state corruption after failure or step restart.

## V-003 — Radau IIA anchors

- Pass: one-stage method equals backward Euler; three-stage method shows order 5 before roundoff, stiff accuracy, scalar \(R(z)\) parity, and nonlinear convergence/failure semantics.
- Kill: coefficient/tableau mismatch, hidden block work, or false convergence.

## V-004 — Two-level certificate

- Freeze previous false-accept corpus plus new BDF/Radau difficult Newton states.
- Operational lane pass: zero false accepts on holdout; every uncertain case falls back.
- Rigorous lane pass: interval/Kantorovich radius contains direct-oracle root on cases with supplied derivative bounds.
- Kill: a `rigorous=true` result without a validated derivative bound.

## V-005 — External global-error Pareto

- Run multiple fixed \(h\) values and calibrated tolerances.
- Record global error, failed/rejected work, RHS/JVP/Jacobian/factorization, memory and wall time.
- Authoritative ranking: nondominated Pareto front at matched problem/output requirements.
- Ambiguity: reference error is not at least 10x smaller; rerun with a tighter independent method.

## V-006 — DAE capability gate

- Not executed in this cycle.
- Requires residual-form \(F(t,y,\dot y)=0\), consistent initialization, algebraic residual, and index-1 test suite.
- Unsupported candidates report `NOT_APPLICABLE`, not failure.

## Escalation order

BDF1/2 -> Radau1/3 -> adaptive common gate -> variable-order BDF -> SDC -> ROK/BOROK -> Peer/W -> Leja capability lane.
