# Phase 5 — Independent adversarial review

1. **Reference bias:** one solver cannot certify itself. Initial corpus therefore uses analytic manufactured solutions only.
2. **Interpolation bias:** linear interpolation would reduce high-order methods to low-order output accuracy. Initial authority requires exact landing.
3. **Endpoint blindness:** endpoint error alone may cancel. Report max-grid and RMS-grid error too.
4. **Scale leakage:** using `max(|candidate|,|reference|)` in the denominator changes the target by solver. Use reference-only weights.
5. **Startup hiding:** BDF2 startup and history cost must remain in counters and wall time.
6. **Failed-run censoring:** failed/nonfinite/budget-exhausted points remain rows and never enter a Pareto front.
7. **Timer noise:** scientific output is deterministic; wall-time ranking requires repeated randomized fresh-process execution later.
8. **Composite-cost arbitrariness:** no universal conversion between RHS, factorization, memory, and seconds. Keep separate fronts.
9. **Reference uncertainty:** a numerical reference must carry a nonzero radius or explicit `uncertified` status.
10. **Overclaim risk:** this fixed-step replay validates the authority, not production solver rankings.
