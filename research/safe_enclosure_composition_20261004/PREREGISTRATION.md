# SAFE-ENCLOSURE — local exact audit of directed exp/time/midpoint compositions (prospective registration)

RVJ development DAG node `SAFE-ENCLOSURE`, proof obligations
`INTAKE-NATIVE-O2` and `INTAKE-NATIVE-O3`
(`research/rvj_integration_20261004/FINDINGS.json`). Base commit `01535400`.
No false certificate is claimed before the probes below have run.

## The three compositions

C1 (chart, `chart_transport.rs::enclose_flow`). The decay factor is
`exp_neg_enclosure(mul_up(kappa, tau))`, an enclosure of `e^{-x}` at the
rounded-up product `x_up >= kappa tau`. Because `e^{-x}` decreases, the true
`e^{-kappa tau}` can exceed the reported upper endpoint unless the series
slack covers the product rounding.

C2 (stepped certificate, `nonnormal_certificate.rs::certify_exp_action_stepped`,
`decay`). The exponent is `mul_up(mul_up(count, h), re_hi)`. For
`re_hi < 0` the upward time product makes the exponent smaller, so the
reported upper bound on `e^{count h re_hi}` can fall below the true value.

C3 (same function, `radius_norm` with the midpoint `x = 0.5 lo + 0.5 hi`).
The radius is the rounded-up half width, but the distance from the
actually rounded midpoint to the far endpoint can exceed the half width.

## Probes (run on the unmodified base functions, then on the repaired ones)

A Rust test exports, for fixed seeded inputs, the base (and later the
repaired) values as IEEE bits; a Python script checks them with mpmath at
60 digits on the exact binary inputs.

- P1 (C1): 20,000 SplitMix pairs `(kappa, tau)` with `kappa` in
  [1e-3, 1e3] and `tau` in [1e-6, 1], log-uniform, kept when
  `kappa tau <= 700`, plus 200 hand pairs where `kappa tau` is inexact
  near 1, 10, 100, 700. Violation: true `e^{-kappa tau}` outside the
  reported interval.
- P2 (C2): 20,000 triples `(count, h, re_hi)` with `count` in [0, 4096],
  `h = tau / N` (tau log-uniform in [1e-3, 10], N a power of two in
  [1, 4096]) and `re_hi` in [-1e3, -1e-6] and [1e-6, 10]. Violation:
  true `e^{count h re_hi}` above the reported decay upper.
- P3 (C3): 20,000 intervals: random `lo` and `hi = lo + k ulp` (k in
  1..64), random wide intervals, sign-straddling and subnormal-adjacent
  ones. Violation: `max(m - lo, hi - m)` above the reported radius.

Each base violation found is recorded verbatim (inputs, reported value,
exact value) as a local counterexample to the composition. A local
counterexample is not by itself a false end-to-end certificate; that is
checked separately below.

## Repair (made whatever the probes show; each is outward by construction)

- C1: the decay is the enclosure of `e^{-x}` over the product interval
  `[mul_down(kappa, tau), mul_up(kappa, tau)]` (new public
  `exp_neg_interval_enclosure`; `exp_neg_enclosure` itself unchanged).
- C2: the exponent is the upper endpoint of the interval product
  `[mul_down(count, h), mul_up(count, h)] * re_hi`.
- C3: the radius is `max(sub_up(m, lo), sub_up(hi, m))` about the rounded
  midpoint `m`, for the start vector and every step.

## Gate (all required for PASS)

G1. The repaired compositions have zero violations on P1-P3.
G2. Changed consumers: the existing chart, REV-02 and RVJ boundary tests
    pass; the REV-02 generator rerun with the repaired code (into this node,
    not over the REV-02 files) has every bound enclosing the 50-digit truth
    under `tools/rev02_nonnormal_check.py`; the R-NEXT-05 chart export rerun
    (into this node) has every physical bound and box enclosing the 50-digit
    reference under `tools/rnext05_chart_check.py`.
G3. End-to-end, the base REV-02 and R-NEXT-05 recorded runs are re-checked
    only through the published results: if a base local counterexample
    exists, it is reported whether any published end-to-end bound was
    actually below the truth (from the published RESULTS files; no rerun of
    those campaigns).

Reported: number of base violations per probe, the largest relative
under-coverage, and the change in the REV-02 bounds and chart bounds
(relative). The REV-02 FAIL verdict (L-0060) and the R-NEXT-05 verdict are
not changed by this node.

## Kill / abstain

A sampled slack argument is not accepted as an enclosure proof; a
campaign PASS is not accepted in place of the monotone composition.

## Results (append only after the recorded run)
