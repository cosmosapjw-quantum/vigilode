# Independent Robertson reference computation

Date: 2026-10-25 UTC/KST host session. WolframLanguageEvaluator was called via lazy Wolfram MCP. This is reference generation only; no Wolfram timing was compared with solver timing.

Common exact equations:

```wl
eq = {
  x'[t] == -(1/25) x[t] + 10000 y[t] z[t],
  y'[t] == (1/25) x[t] - 10000 y[t] z[t] - 30000000 y[t]^2,
  z'[t] == 30000000 y[t]^2,
  x[0] == 1, y[0] == 0, z[0] == 0
};
```

First attempt, discarded: solve `{t,0,40}` using `Method -> "StiffnessSwitching"` with `WorkingPrecision -> 35, AccuracyGoal -> 20, PrecisionGoal -> 20` and separately `(50,28,28)`, both with default `MaxSteps=10000`. The tool emitted `NDSolveValue::mxst` at `8.991926802415100...` (WP35) and `9.288596670164189...` (WP50), then `InterpolatingFunction::dmval` for `t=40`. Therefore all `t=40` output from this attempt was **discarded**. The `t=0.01` output was not used for the final BDF reference.

Successful independent BDF precision pair:

```wl
s1 = NDSolveValue[eq, {x,y,z}, {t,0,40}, Method -> {"BDF"},
  WorkingPrecision -> 35, AccuracyGoal -> 20, PrecisionGoal -> 20,
  MaxSteps -> Infinity];
s2 = NDSolveValue[eq, {x,y,z}, {t,0,40}, Method -> {"BDF"},
  WorkingPrecision -> 50, AccuracyGoal -> 28, PrecisionGoal -> 28,
  MaxSteps -> Infinity];
Table[Through[s2[tt]], {tt,{1/100,40}}]
```

Both interpolation domains were `[0,40]`; no warning was emitted. WP50 values:

```text
t=0.01:
0.9996006826882932010120243311989982661709795327003002116671
0.0000364504788784424249079365653485540580633344379703542707
0.0003628668328283565630677322356531797709571328617294340622
t=40:
0.7158270687194050904744763315912011266141031523202485221676
9.1855347645577639038993128626928004356783975654990437e-6
0.2841637457458303517616197690959361805854611692821859787883
```

WP35-vs-WP50 max component differences: `3.6277791265156578e-20` at `t=0.01`; `1.1964937852086744e-17` at `t=40`. WP50 mass defects were zero at the printed precision. Separate `StiffnessSwitching`, WP50/goal28, `MaxSteps -> Infinity` over `[0,1/100]` gave:

```text
0.9996006826882932010120243321559211229176817022471598013827
0.0000364504788784424249079255090255879816030200549374240473
0.0003628668328283565630677423350532891007152776979027745697
```

The published endpoint at `t=40` is `(0.7158270687194135, 9.185534764558135e-6, 0.28416374574582)` in https://link.springer.com/article/10.1007/s10910-022-01427-7 (§6.3), for the exact same equations. Canonical RHS is also given at https://www.unige.ch/~hairer/testset/stiff/rober/equation.f.

Reference uncertainty `1e-12` in `REASSESSMENT_V2.json` is a conservative **numerical estimate**, not a certified mathematical bound. It is much larger than observed precision and cross-method/literature spreads while much smaller than the benchmark's `1e-5` reference allowance.
