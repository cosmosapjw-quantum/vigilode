# SAFE-ENCLOSURE-b — the underflow floor of `directed::exp_interval` (prospective registration)

Follow-up of `safe_enclosure_composition_20261004` (L-0064, FAIL). Base
commit `c1e9618`.

## Defect

`exp_interval(x)` returns `[0, 2^-1020]` for every `x < -707`. Since
`ln(2^-1020) = -707.02345...`, the upper endpoint is below `e^x` for
`x` in `(-707.0234, -707)`. L-0064 recorded one probe row there
(`x = -707.01`, 0.30 % under). REV-02 checked 251 grid points and missed the
window.

## Repair

For `x < -707` return `[0, 2^-1019]`: `e^x < e^-707 = 9.86e-308 <
2^-1019 = 1.78e-307`. Nothing else in `exp_interval` changes. Consumers
(REV-02 growth and decay factors, SAFE-ENCLOSURE `decay_upper`) only become
looser for exponents below -707.

## Gate (all required for PASS)

G1. 60-digit mpmath check: `e^x` lies in `exp_interval(x)` for (a) the
    L-0064 counterexample, (b) 20,001 equispaced points on [-709, -700]
    plus the 64 binary64 neighbours on each side of -707 and of
    `ln 2^-1020`, (c) the 251-point REV-02 grid, (d) 20,000 SplitMix points
    log-uniform in |x| on [1e-300, 709] with random sign.
G2. The L-0064 C2 probe rows re-exported with the repaired code (same seed)
    have zero repaired violations; C1 and C3 stay at zero.
G3. Existing directed, nonnormal (INT-05, REV-02) and chart tests pass.

## Results (append only after the recorded run)

### Executed result — 2026-10-04 (source `171a4d2`)

**Verdict: PASS.** G1: zero violations in 40,511 rows (the L-0064
counterexample, 20,001 grid points on [-709, -700], 258 neighbours of -707
and of ln 2^-1020, the 251 REV-02 grid points verbatim, 20,000 random
points). G2: the SAFE-ENCLOSURE probes re-exported with the same seeds have
zero repaired violations in C1, C2 and C3; the base C2 replica now has 676
violations (the floor one is gone, the 676 time-direction ones remain).
G3: core lib, INT-05, REV-02, SAFE-ENCLOSURE tests and the chart tests
(INT-04 contracts, native re-audit, RVJ boundaries, R-NEXT-05) pass; clippy
on rodas5p-core clean.

Correction (after the independent review, same day): `ln 2^-1020 =
-707.010124`, so the false window was `(-707.0101, -707)`, not
`(-707.0234, -707)` as written in the defect description above. The "64
neighbours of ln 2^-1020" in the export used the wrong centre
(-707.0234...); the window is still covered by the 20,001-point grid on
[-709, -700] (spacing 4.5e-4, about 22 points inside the window), all
without violation. Verdict unchanged.
