# Phase 0 — Task Contract

Goal: implement native fixed-step BDF1/BDF2 and Radau IIA1/3 anchors after the math research gate.

Protected: existing RODAS5P/SABR/homotopy behavior, Rust 1.94.1, faer backend, no external BLAS.

Non-goals: production adaptivity, variable-order BDF, SDC/Peer/ROK/BOROK/Leja, singular-mass DAE.

Completion: targeted TDD, full regression, order/stiff/mass gates, registry execution, independent review and durable artifacts.
