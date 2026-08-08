# Phase 7 — Scientific validation

## Common gates

Every executable candidate is tested against:

1. the same protected sequential/direct RODAS5P oracle;
2. the same C1 and C2 correction diagnostics;
3. a C3 refined-root reference when it converges;
4. the C0 oracle as an explicitly labelled offline fallback when C3 exhausts;
5. a global fifth-order screen before the roundoff floor;
6. a scalar stiff-decay screen;
7. nonnormal and noncommuting-mass one-step cases;
8. identical output budget and WRMS scale;
9. fallback-inclusive counters;
10. Tier-specific measured performance.

## Canonical result

The exact one-thread canonical result contains:

```text
cases                    6
nonlinear rows          936
completed               349
rejected                587
uncertified               0
numerical failures        0
C0 reference fallbacks    4
first-correction false accepts 0
```

All four C0 fallbacks occur on the severe `mv-s10000-m1000-eta0.9-h0.001` case. Each candidate is rejected by the protected oracle with output WRMS between roughly `5.8e4` and `1.2e6`; no failure is converted into a success.

## Scientific-only versus joint verdict

Scientific gates alone identify 23 implemented configurations that pass the current finite correctness screen:

- 7 iterative sequential variants;
- 4 SABR variants;
- 12 `theta=1,q=7` homotopy configurations.

This is not a promotion result. After adding the locked performance layer, the joint verdict is:

```text
reference   2
hold      154
deferred    7
promote     0
```

The two scoped references are:

- `sequential-direct-off` for nonlinear-stage correctness;
- `sequential-gmres-off` for Tier-L linear-solver performance.

## Tier-L observations

The best nonreference Tier-L result is `sequential-lgmres-off`, with median wall speedup about 1.06x but median operator work about 1.068x the GMRES/OFF reference. It misses the locked 1.15x promotion threshold.

GCRO-DR STAGE/PERSISTENT reduces median operator work relative to GMRES/OFF in this suite, but projected/recycle algebra makes wall time slower. This is consistent with the earlier Rust allocation audit and does not constitute a recycling no-go.

## Tier-N observations

All SABR variants reduce batch depth to one quarter of sequential, but they use about 3.125x the RHS evaluations, 5.7x–21.7x the JVP-vector work, and 2.3x–5.6x the measured candidate time. They are held.

The 12 scientifically eligible homotopy configurations use `theta=1,q=7`. Their median batch depth is 24–56, and their scalar work is far above sequential. They are correctness controls, not fast paths.

No low-depth homotopy configuration simultaneously passes global order, stiff decay, nonnormal certification, C3 availability, and measured performance.

## Scientific interpretation

The harness blocker is resolved: every implemented candidate receives the same criteria and cannot hide fallback or certification work. The scientific result is negative but useful: no current nonreference candidate merits promotion under the frozen gate.
