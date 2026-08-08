# Phase 2 — Localization

Primary code: `block.rs`, `homotopy.rs`, `sabr.rs`, `sequential.rs`, `problem.rs`, CLI `main.rs`. The target residual is `StructuredBlockSystem::apply(K) - nonlinear_rhs(K)`; the future certificate requires explicit stage Jacobians.
