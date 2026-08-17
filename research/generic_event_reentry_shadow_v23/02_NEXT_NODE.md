# Next node — tolerance-aware early-defect telemetry preflight

## Branch authority

Create a successor branch from the committed G4-S5B3 attempt-geometry closeout:

```text
generic-tolerance-scaled-early-defect-v2.4
```

Do not implement an event-reentry state machine, early abort, controller/cache
transfer, or active method switching in this node.

## Why this node is next

The P0-05 campaign found an exact cross-tolerance collision: identical raw
`eta_c2` and proposed step produced both accepted and rejected outcomes because
the adaptive error norm changes with tolerance. Event conditioning alone cannot
restore information that the scalar discards. A tolerance-aware diagnostic must
therefore be tested before opening the re-entry shadow state machine.

## Contract candidate

For the physical component prefix `i=1..m`, seal before producing output:

\[
\rho_{c_2}^{\mathrm{WRMS}}
=\left[
\frac1m\sum_{i=1}^m
\left(
\frac{|h|D_{2,i}}
{\mathrm{atol}+\mathrm{rtol}\,\max(|y_{n,i}|,|U_{2,i}|)}
\right)^2
\right]^{1/2}.
\]

This is a proposed admission diagnostic, not a local-error theorem. The exact
scale must be checked against the existing controller convention before the
contract is committed. For time augmentation, exclude the clock component.

## Non-negotiable implementation boundary

Reuse already available `y_n`, `U2`, and `D2`. Required added expensive work:

```text
RHS calls             0
JVP calls             0
JVP vectors           0
phi actions           0
partial-t calls       0
Jacobian builds       0
Newton iterations     0
```

Record component scaling, WRMS accumulation, and scalar normalization in a
separate diagnostic ledger. Do not alter canonical `WorkCounters`.

## Predeclared views

Run both views without selecting between them post hoc:

1. **all-attempt view:** every adaptive E-K attempt;
2. **entry-window view:** attempt indices 0 and 1 of each trajectory.

The entry-window view is descriptive in this node. It is not yet a runtime
event detector and does not authorize switching.

## Frozen corpus and controls

- same 4 problems x 3 tolerances = 12 trajectories;
- full MGS, one thread, endpoint-only output;
- raw `eta_c2` retained as the control score;
- disabled/read-only numerical identity repeated;
- complete per-trial work ledger retained;
- paired overhead protocol unchanged;
- no physical-client data or threshold tuning.

## Kill tests

1. The quadratic tolerance triplet must no longer have an exact scaled-score
   collision; ordering must move in the direction of the adaptive error norm.
2. Any change to state, output, attempt sequence, controller history, or existing
   work counters stops the node.
3. Any added RHS/JVP/phi/Jacobian/Newton work stops the node.
4. Any unscorable attempt in the canonical corpus stops the node.
5. If all-attempt or predeclared entry-window sweep is no better than all-admit,
   reject that use of the score; do not tune a threshold around the failure.

## Required output

- complete raw rows for `eta_c2` and `rho_c2_WRMS`;
- exact collision audit;
- pooled/per-problem/per-tolerance CDFs;
- all-attempt and entry-window false-count Pareto frontiers;
- leave-one-problem-out concentration audit;
- paired wall and Gamma overhead;
- PHYS-MATH and PHYS-MATH-CODE audits;
- plot-based adversarial reading;
- no selected or recommended threshold.

## Opening condition for the following node

Only if a tolerance-aware score survives the frozen preflight may the project
open a **read-only event-triggered re-entry shadow state machine**. That later
node must separately define event timing, residence/cooldown, failed-shadow work,
and fifth-order recovery obligations.
