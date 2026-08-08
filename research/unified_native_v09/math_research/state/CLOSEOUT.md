# Phase 10 — Closeout

## Confirmed

- The first-correction certificate blocker cannot be repaired by threshold tuning alone; reliable nonlinear tail control requires derivative-variation information or conservative fallback.
- A two-level operational/rigorous certificate is the honest next architecture.
- BDF1/BDF2 and Radau IIA1/3 are the smallest bounded native complete-integrator anchors.
- Cross-family promotion must use external global-error versus total-cost Pareto results.
- Deferred families require capability tiers rather than forced universal comparisons.

## Rejected or weakened

- Universal cheap rigorous certificate from residual + one correction.
- Equal input tolerance or accepted-step count as authoritative ranking.
- Treating external RADAU5/CVODE timings as same-backend Rust comparisons.
- Implementing all deferred families in a single coding change.

## Still uncertain

- Whether a stage-recursive output radius is tight enough for online use.
- Whether native dense Radau remains useful beyond correctness-scale dimensions.
- Which large-system traces justify ROK/BOROK/Leja.

## Next primary question

Can a shared Rust nonlinear-solve and complete-integrator contract implement mathematically exact fixed-step BDF1/BDF2 and Radau IIA1/3 anchors with transparent work accounting, correct order/stiff behavior, and no regression of the protected RODAS5P baseline?
