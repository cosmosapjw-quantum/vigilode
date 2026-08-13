# G2 Plot-based CRAG Audit

## Plot 1 — nonlinear order convergence

`G2_ORDER_CONVERGENCE.png` shows three clean, nearly parallel log-log trends with the expected separation. No order crossover or roundoff flattening appears over the chosen grid.

- Correctness: slopes agree with declared 2/4/5 orders.
- Retrieval: agrees with the raw order rows and exact solution `1/(1-t)`.
- Augmented check: the conclusion is limited to one smooth scalar nonlinear problem.
- Generation: predicts that an adaptive pexprb pair can be viable, not that it is efficient.

Surviving claim: the implementations reproduce their declared classical orders on this bounded nonlinear test.

## Plot 2 — φ oracle error

`G2_PHI_ORACLE_ERROR.png` keeps every point around `1e-16`, far below the `2e-11` foundation gate.

- Correctness: matches high-precision receipt.
- Retrieval: both nonnormal and oscillatory matrices are represented.
- Augmented check: dimensions are small; large Krylov residual behavior is not represented.
- Generation: projected dense oracle is adequate for the next Krylov-engine development.

Narrowed claim: roundoff-level small-matrix accuracy, not production residual certification.

## Plot 3 — depth versus executed work

`G2_DEPTH_VS_EXECUTED_WORK.png` is the decisive negative plot. pexprb54s4 has dependency depth 3, but separate φ actions and JVP-vector counts are 15 and 18 per scalar step.

- Correctness: raw counters agree.
- Retrieval: prevents conflating stage depth with work.
- Augmented check: actual JVP cost will grow with state dimension and RHS expense.
- Generation: fused linear-combination φ actions are the load-bearing next optimization.

Surviving claim: parallel structure exists.
Rejected claim: depth 3 already implies speedup over RODAS5P.

## Plot 4 — stiff linear decay

`G2_STIFF_LINEAR_DECAY.png` shows roundoff-level agreement for `|z|` up to 500.

- Correctness: expected because the linearization is exact and the nonlinear remainder vanishes.
- Retrieval: confirms sign and scaling of `hJ`.
- Augmented check: does not probe nonlinear order reduction or inaccurate JVP.
- Generation: nonlinear stiff/oscillatory holdout remains mandatory.

Narrowed claim: exact-linear stiff decay only.

## Adversarial mutations

- Sign mutation `z→-z` would produce growth and immediately break the stiff-decay interpretation.
- Lower Krylov dimension would expose whether nested-dimension diagnostics miss nonnormal error; deferred to G3.
- Removing level-one concurrency would make actual depth 4; this was found and repaired before closeout.
- Treating each φ term as a free parallel operation would hide total JVP work; explicitly rejected.

## Plot verdict

The plots support G2 correctness and expose, rather than hide, the current economic blocker. No performance promotion is warranted.
