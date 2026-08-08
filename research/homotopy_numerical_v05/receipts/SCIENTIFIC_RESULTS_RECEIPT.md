# Scientific results receipt

Canonical source:

```text
research/homotopy_numerical_v05/results/HOMOTOPY_EXPERIMENT_SCREEN.json
SHA256 a665562e3088950e3a7d64e985f220708cac6cad7356f096baf378e4ee27026e
```

Key counts:

```text
cases=57
controls=114
candidates=8208
fast_accepts=3510
fallbacks=4698
failures=0
false_accepts=0
low_depth_fast_rows=401
low_depth_cases=16
```

Main negative result:

```text
The fixed one-step output budget E_out <= 0.1 is locally safe on the canonical grid but does not
preserve global fifth order for low-depth c=0 paths.
```

Equal-work result:

```text
No fast candidate is no-worse than sequential or SABR simultaneously even in RHS evaluations
and JVP vectors. `direct_solve_calls` is retained only as a batched-call-depth counter; low-depth
paths use a median of 40 common-W solve vectors versus 8 sequential stage solves.
```
