# Phase 3 — Bounded Solution Design

## Option A — One universal step-doubling controller

Simple and auditable, but wastes native RODAS embedded information and triples nominal work for every
family.  Rejected as the sole production architecture; retained as a reference lane.

## Option B — Shared controller with native/reference estimator adapters (selected)

One validated state machine controls acceptance and bounded factors.  Sequential/SABR/Homotopy use
the existing RODAS endpoint estimator; BDF/Radau use coarse-versus-two-half-step estimates.  This is
the smallest design that preserves method identity and comparison fairness.

## Option C — Production-quality family-specific controllers immediately

Native RADAU5 estimator, variable-order BDF, cost-aware Homotopy/ROK/Leja logic, and adaptive SDC
would be more efficient but exceed the current correctness-first scope.  Deferred until the common
reference lane is measured.
