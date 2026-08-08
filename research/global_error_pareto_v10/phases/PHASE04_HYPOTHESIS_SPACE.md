# Phase 4 — Hypothesis space

## H1 — Reference-scaled WRMS

Use fixed reference-based weights

\[
w_{k,i}=a_i+r_i|y_i^{\rm ref}(t_k)|
\]

and report endpoint, maximum-grid, and RMS-grid WRMS errors. Candidate values never enter the denominator.

## H2 — Reference uncertainty composition

If the reference has a certified WRMS error radius \(\epsilon_{\rm ref}\) under the same fixed weights, use

\[
E_{\rm conservative}=E_{\rm candidate-ref}+\epsilon_{\rm ref}
\]

by the triangle inequality.

## H3 — Exact-landing output grid

The first authority accepts only trajectories containing every requested output time. It rejects interpolation rather than silently adding a low-order interpolation bias.

## H4 — Per-metric Pareto fronts

Construct separate fronts for wall time, RHS evaluations, JVP vectors, nonlinear iterations, and factorization counts. Do not combine them into one weighted score.

## H5 — Target-matched selection

For each external global-error target, select the least-cost successful point for each candidate family. Internal step/tolerance controls may differ.

## H6 — Fixed-step anchor replay

Before adaptive methods are compared, use the unchanged fixed-step RODAS5P direct, BDF1/2, and Radau IIA1/3 anchors to validate error and accounting semantics.
