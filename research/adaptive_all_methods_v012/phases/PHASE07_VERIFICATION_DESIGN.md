# Phase 7 — Minimal Decisive Verification

## V1 — Variable-step BDF2 polynomial exactness

- Input: \(y(t)=t^2\), \(y'=2t\), unequal adjacent steps.
- Pass: one BDF2 step reproduces the exact endpoint to binary64-scaled tolerance.
- Kill: coefficient or history mismatch.

## V2 — Constant-step regression

- Input: existing BDF2 anchor suite.
- Pass: fixed-step trajectory and counters unchanged when variable mode is not requested.

## V3 — Transactional rejection

- Force a rejected BDF, Radau, RODAS, SABR, and homotopy trial.
- Pass: state/history/recycle/output index remain unchanged; work remains counted.

## V4 — Tolerance response

- For each family run at least three tolerance levels on analytic problems.
- Pass: tighter tolerance does not increase external global error; failures are explicit.

## V5 — Global order

- BDF1/Radau1: first order.
- BDF2: second order after startup.
- Radau3/RODAS5P: fifth order before roundoff.
- SABR/homotopy: same endpoint order only when their fast path is accepted or fallback is recorded.

## V6 — Stiff and nonnormal screen

- Prothero–Robinson, noncommuting mass matrix, nonnormal manufactured system.
- Pass: no false success, NaN/Inf, or unbounded rejection loop.

## V7 — Common-grid fairness

- All methods use the same `OutputSchedule` and requested-output-only storage.
- Compare external global error against full work and wall time.

## V8 — Controller ablation

- I versus PI, bounded factor changes, clipping behavior.
- Holdout criterion: no order regression and fewer/equal rejections or lower total cost.
