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

### Executed result — 2026-10-04 (source `4e5dc91`)

Commands: `SAFE_ENCLOSURE_PROBES=.../probes.json cargo test --release -p
rodas5p-core --locked --test safe_enclosure_probes -- --ignored`, then
`python3 tools/safe_enclosure_check.py`, the R-NEXT-05 export and check and
the REV-02 generator and check into this directory, and `summarize.py`
(RESULTS.json). The probe test was lint-fixed after the recorded run
(`% k == 0` to `is_multiple_of`, one needless `mut`, rustfmt); a re-export
with the committed source is byte-identical to `probes.json`.

**Verdict: FAIL (G1).** The repaired C2 has one violation in 20,000 rows:
count 1071, h = 0x3f5d28b01f11190b, a = 0xc0772ec0562e1c0f, so
`count h a = -707.01`; the reported upper is `2^-1020 = 8.9002e-308` and the
truth is `8.9274e-308` (0.30 % under). The cause is not the repair but
`directed::exp_interval` (REV-02): for every `x < -707` it returns
`[0, 2^-1020]`, which is false on `(-707.0234, -707)` because
`ln 2^-1020 = -707.0234`. The REV-02 grid of 251 points did not sample that
window. This is an error in a published directed primitive; it is repaired
in the follow-up node `safe_enclosure_exp_floor_20261004`, not here.

Base local counterexamples (all recorded; first 50 verbatim per probe in
PROBE_RESULTS.json):
- C1 chart decay at the rounded-up product: 2 of 20,200, at most
  4.7e-16 relative under the truth.
- C2 stepped decay with the upward time: 677 of 20,000 (676 from the time
  direction with `re_hi < 0`, at most about 4e-15 relative; 1 from the
  `exp_interval` floor above).
- C3 half width about the rounded midpoint: 2,660 of 20,000, at most
  0.5 ulp of the midpoint under.

Repaired C1 and C3: zero violations.

G2: existing chart (INT-04, native re-audit, RVJ boundaries) and nonnormal
(INT-05, REV-02, shared-shift) tests pass; the R-NEXT-05 rerun check is
PASS with every box and physical bound enclosing the 50-digit reference
(19,652 points, every `y` bit-identical to the published run, bounds larger
by 1e-16 to 1.2e-8 relative); the REV-02 rerun has every bound enclosing
the truth (enclosure gate true; its own stiff-usefulness gate stays false,
L-0060 unchanged). 35 of 72 REV-02 bounds grew by 11 % to 100 %, all
candidates bit-identical: where the rounding term dominates and a Horner
interval is one or two ulps wide, the rounded midpoint sits on an endpoint
and the distance is the full width, twice the half width the base used.

G3: the published end-to-end results show no bound below the truth (REV-02
enclosure gate true, R-NEXT-05 PASS). The base local under-coverage was
covered there by other slack; no false end-to-end certificate is claimed.
