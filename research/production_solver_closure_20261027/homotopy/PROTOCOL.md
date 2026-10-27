# RHS-only production closure: fixed protocol

Baseline: PR91, `206298b4068d923bb47c7160b111b7439098660c`.
Research question: remove scalar container overhead and compare a direct algebraic
predictor against value-only iteration without weakening the original polynomial
residual or physical-error certificate.

The alternatives are (A) the existing generic Vec-valued homotopy implementation,
(B) a stack scalar specialization preserving its evaluation order, and (C) a
cancellation-free positive-cubic inverse predictor with original-residual interval
verification and a bounded scalar/bisection fallback. All are matrix/Jacobian/
Newton-free. Alternative C belongs to the sealed cubic model, not generic RHS.

Cases are k=1, g=1/2, Y=T=1 at physical caps 1e-3, 1e-4, 1e-5; plus a linear
case, a nonlinear-dominated case, zero state and a represented rounding-floor
refusal. Plans, dyadic grids and all old accuracy thresholds stay unchanged.
The main timing comparison uses identical warm Picard settings, five paired
ABBA/BAAB blocks with deterministic repetitions. Allocation/point output remains
inside each solve; JSON and oracle calculations stay outside timed regions.
Report full states, map/interval/predictor counts, exact-model error and bound.
Callgrind N and 2N invocation slopes may subtract process startup. These are
instruction counts, not hardware cycle or portable wall-time predictions.

All native builds/timings/profiles use the shared flock, pinned Rust1.94.1,
measurement profile, locked/offline dependencies, jobs2. User limit: maximum
three diagnosed correction/re-experiment passes; initial implementation is pass0.
Do not widen scope or relax caps after a failure. Preserve failed evidence.

The analytic formula is derived in THEORY.md. Candidate-root arithmetic need not
be correctly rounded: acceptance always uses the existing outward original
polynomial residual and represented state/time rounding displacement. A failure
retains accepted observations and every dispatched polynomial evaluation.
