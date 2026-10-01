# R4 time/output audit: pre-execution contract

Source: `1c54194123ee6abc6daa512e8574922f510b4e2c`. Prior report commit:
`6598abd0db790162ab26f50bc13943f24a0c7e4a`. No production modifications.
This contract and all scientific inputs must be committed before execution.
GPT-6 Astra v4 research/coding harness is the requested workflow, not a claim
about the actual runtime model. Owner proposes; independent reviewer decides.

## Questions and fixed inputs

1. Original R3-TIME-01: constant flow on 8 ULP spans at origins +/-1e12,
   nominal h=1e-4. Fixed Rodas, fixed BDF and fixed Radau should integrate
   represented duration. Native endpoint tolerance 1e-12. Typed time-resolution
   refusal is a separate safe outcome, not numerical success.
2. Original R3-TIME-02: uniform spacing 1 on a 4 ULP span at 1e12 and lone [tf]
   must be rejected; explicit endpoints must retain labels and integrate flow.
3. BDF history/unit covariance: boundaries 2^p, p=-40,0,10,40; t0=boundary-8
   lower ULP, tf=boundary+8 upper ULP; h=0.675 upper ULP. Constant velocity
   v=1/(tf-t0). For exact-binary64 inputs the reference is v*(t-t0), not 1
   assumed exact. Constant flow has no BDF discretization error with the
   correct variable-step coefficients. Gate 1e-12 absolute per recorded state.
   Compare native fixed BDF with standalone driver applying the existing public
   bdf_step_variable to exactly the same recorded times. Exact rational BDF
   recurrences independently check the same_step selection and quantify its
   defect. A known disclosed absolute floor is not automatically a new bug;
   newly reproduced wrong values, if any, are the material evidence.
4. Cap boundary: origin 1e12, span 8 ULP, max_step=[0.49,0.75,1.0,1.5,2.0]*ULP,
   min_step=1e-20, initial_step=max_step, 100 attempts. Adaptive Rodas records
   accepted step sizes. Strict cap and declared one-resolution-slack policies
   are scored separately, because docs presently mix both descriptions.
5. Step doubling: Radau1 and BDF adaptive at 1e12 over 8 ULP, h=[1,2,3]*ULP.
   Correct constant-flow endpoint or explicit midpoint-resolution refusal;
   no silent zero-width or overlap acceptance. 100 attempts maximum.
6. Nonautonomy: y'=2(t-shift)/unit^2, exact y=((t-shift)/unit)^2,
   analytic partial_t=2/unit^2 and JVP=0. shift=[0,2^30], unit=[1,2^10],
   64 nominal steps. Also zero/cross-zero constant flows (-0.7,0.3),
   (-1e-300,1e-300) with 10 nominal steps. Quadratic-flow gate is 1e-10 +
   128 eps |shift|/unit (explicit input-time conditioning allowance).
7. The existing r3_represented_clock_contracts.rs target is executed once by
   the runtime owner or this lane; duplicate execution not needed.
8. Unequal-halves estimator: t0=1e12, H=3 ULP, y'=2(t-t0)/H^2,
   y(t0)=0, exact final y=1, Radau1, atol=0.5, rtol=1e-12,
   initial_step=max_step=H, min_step=1e-20, 100 attempts. The 2+1 ULP split
   gives exact implicit-Euler fine error 5/9 but coarse-minus-fine 4/9;
   standard equal-half divisor underestimates by factor 5/4. Direct native
   public radau_step triple + public step_doubling_wrms_error and adaptive
   driver are both observed. A split-ratio candidate multiplies the difference
   by q/(1-q), q=(h1/H)^(p+1)+(h2/H)^(p+1), p=1. This is exact for this
   quadratic primitive and leading-order for smooth equal-coefficient LTE;
   it is not a global ODE bound. Gate: estimator metadata must not claim
   equal-half Richardson semantics for materially unequal represented halves.

## Oracles and candidate

`exact_oracle.py`: Python Fraction.from_float, no imported production clock,
matrix, tableau, BDF coefficient or interpolation functions. Exact constant
flow and independently derived three-node differentiation weights. The
candidate reuses the native Newton solver only; its oracle is independent.
`run_probe.py`: writes raw JSONL + stderr + process exit + source SHA.

Commands after root announces PREREG_COMMITTED and runtime readiness:

```
source runtime_r4/env.sh
python3 r4/time/run_probe.py --repo vigilode --output r4/time
python3 r4/time/exact_oracle.py --input r4/time/native_raw.jsonl --output r4/time/EXACT_ORACLE.json
```

Cargo builds use the global `runtime_r4/cargo-build.lock`, one process, no
incremental build. Build timeout 600s; execution timeout 120s. Environment
errors preserve first failure and allow one evidence-based repair. No broad
performance test or post-hoc tolerance relaxation. Input expansion after
results requires a separately committed addendum, otherwise exploratory only.

## Completion and claim ceiling

Deliver raw observations, exact comparisons, source locations, original-finding
closure matrix, distinct new/regression/known limitations, Korean explanation,
and machine-readable next steps. No global solver correctness/speed claim.
