# Phase 2 — Evidence Acquisition

## Literature genealogy

- Classical embedded one-step methods estimate local error with a lower-order companion and update \(h\) from the estimator's asymptotic order.
- RODAS5P already supplies an embedded 5(4) difference and therefore needs a controller, not step doubling, as its primary lane.
- RADAU5 uses a dedicated estimator plus bounded and predictive step control; a uniform step-doubling lane is nevertheless valuable as a method-independent correctness reference.
- BDF codes require dynamic coefficients, history-aware local-error estimation, rejection recovery, and step/order logic. Constant-step BDF2 cannot simply be called with a new \(h\).
- SDC exposes both step-size and sweep-count adaptivity. Peer methods require coefficient/order conditions valid under changing step ratios. ROK/BOROK and exponential methods couple time-step choice to iterative algebra cost.

## Directly supported conclusions

1. **SUPPORTED:** BDF adaptivity is inseparable from recent step-size history and order selection in production codes.
2. **SUPPORTED:** RADAU5 combines a local estimator, bounded factors, predictive control, and reuse decisions.
3. **SUPPORTED:** SDC can adapt both \(\Delta t\) and iteration count using method-specific estimators.
4. **SUPPORTED:** Exponential and iterative implicit methods may require cost-aware controllers because per-step cost changes with \(h\).
5. **SUPPORTED:** Peer/W variable-step accuracy and stability require method coefficients/order conditions designed for changing step ratios.
6. **SUPPORTED:** RODAS5P has an embedded estimator appropriate for its native adaptive lane.

## Evidence gaps

- No rigorous cheap production certificate currently exists for the homotopy fast path.
- No current Rust implementation exists for deferred SDC/Peer/ROK/BOROK/Leja families.
- A universal controller cannot guarantee equal efficiency because estimator reliability and per-step cost differ.
