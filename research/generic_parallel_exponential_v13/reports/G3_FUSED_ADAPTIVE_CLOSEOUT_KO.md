# G3 Fused Adaptive Parallel Exponential Closeout Report

Date: 2026-08-13
Branch: `generic-parallel-exp-adaptive-v1.3`
Base tag: `generic-parallel-exponential-foundation-v1.2.0-alpha1`
Scope: generic regular ODE, identity mass matrix, binary64, matrix-free JVP, no physical clients

## 1. 연구 질문

G2의 parallel fifth-order exponential Rosenbrock method는 logical dependency depth가 3이었지만 pexprb54s4 한 step에 15개의 separate phi-action이 필요했다. G3의 질문은 다음이다.

1. 여러 `phi_k(hJ)v` linear combination을 하나의 augmented Krylov process로 융합해 operator/orthogonalization work를 실제로 줄일 수 있는가?
2. fused pexprb54s4가 explicit Jacobian과 Newton 없이 RODAS5P 수준의 step economy를 유지하는가?
3. 동일 external error에서 protected sequential JF RODAS5P보다 빠른가?
4. within-step 4-thread path가 실제 wall time을 줄이는가?

## 2. 구현

### 2.1 Fused phi action

다음 조합을 하나의 augmented exponential action으로 계산한다.

\[
  w(\tau)=e^{\tau A}b_0+\sum_{k=1}^{p}\tau^k\varphi_k(\tau A)b_k.
\]

`A`는 materialized Jacobian이 아니라 matrix-free JVP operator다. Augmented operator의 Krylov basis를 만들고 projected dense exponential만 계산한다.

### 2.2 Error diagnostics

Current convergence diagnostic은 첫 Arnoldi residual term

\[
  \epsilon_m\approx
  |\tau h_{m+1,m}e_m^T\varphi_1(\tau H_m)\beta e_1|
\]

을 사용한다. Nested-dimension difference는 독립 diagnostic으로 보존한다.

이 값은 arbitrary nonnormal operator에 대한 rigorous upper bound가 아니다. G3에서는 correctness reference와 controller diagnostic으로만 사용한다.

### 2.3 Integrators

- fused EXPRB2
- fused EXPRB43
- fused parallel order-5 pexprb54s4 with embedded order 4
- adaptive PI controller
- explicit time augmentation when an exact `F_t` contract is available
- full-MGS primary and IOP2 research backend

pexprb54s4의 code dependency levels는 실제로 3단계다.

1. U2
2. U3/U4 in parallel
3. main/embedded endpoint in parallel

### 2.4 Failure accounting fix

Adaptive run이 endpoint 전에 budget을 소진하면 기존 `OutputCollector::finish()`가 먼저 오류를 내어 attempted work를 반환하지 못할 수 있었다. Failure path에서 `finish_partial()`을 사용하도록 수정했다.

- success path: requested output coverage를 계속 엄격히 검사
- failure path: 마지막 committed output과 all attempted counters를 반환
- false success: 금지

## 3. Canonical protocol

- 7 CPU-pinned campaigns
- same Rust 1.94.1 / faer 0.24.4 measurement binary
- problems: quadratic, complex Dahlquist, oscillatory Prothero–Robinson, semilinear advection–diffusion
- tolerances: `1e-4`, `1e-6`, `1e-8`
- candidate families:
  - protected JF RODAS5P
  - held G1 q1→q2
  - fused pexprb54s4 full-MGS T1/T4
  - fused pexprb54s4 IOP2 T1/T4
  - frozen BDF2
  - frozen Radau IIA3
- each RODAS error level defines a target; every family uses its cheapest successful run at or below that external error
- raw wall fields excluded from scientific checksum

All seven campaigns have scientific checksum

```text
a921e2ce3cf6f2fe752b916bc965c44ba1131aece81054ae3d4eb4ee470a804a
```

## 4. Phi fusion result

| Backend | Cells | Median wall speedup | Median JVP reduction | Median orthogonalization reduction | Max relative error vs dense |
|---|---:|---:|---:|---:|---:|
| Full-MGS | 9/9 | 4.096 | 5.000 | 5.073 | 6.98e-11 |
| IOP2 | 9/9 | 4.124 | 5.000 | 29.368 | 8.01e-11 |

The fused action therefore resolves the G2 load-bearing implementation blocker. This is a kernel result, not yet a universal integrator result.

## 5. Same-error result

Primary candidate: `pexprb54s4-fused-full-mgs-t1`.

```text
successful control runs         12/12
same-error target coverage      11/12
median speedup on covered       1.519x
median accepted-step ratio      1.10
maximum accepted-step ratio     8.333
median JVP ratio                0.380
explicit Jacobian builds        0
Newton iterations               0
```

### By problem

| Problem | Coverage | Median speedup | Median step ratio | Interpretation |
|---|---:|---:|---:|---|
| Complex Dahlquist | 3/3 | 3.366 | 0.125 | ideal linearized stiff/oscillatory regime |
| Advection–diffusion | 3/3 | 1.677 | 1.333 | wall benefit despite slightly worse step economy |
| Quadratic | 3/3 | 0.567 | 1.10 | scientifically valid but slower |
| Oscillatory PR | 2/3 | 0.0943 | 6.827 | load-bearing failure regime |

The aggregate median crosses 1.15 only because favorable regimes are strong. Full coverage is absent and the nonlinear oscillatory regime violates the step-economy objective.

## 6. Within-step thread result

Full-MGS T1 and T4 scientific/work fields are exactly identical. Wall ratios are defined as `T1/T4`.

```text
median over 12 pairs  0.525
minimum               0.357
maximum               0.724
```

Thus the four-thread within-step path is slower on the current small-state corpus. Local Rayon remains implemented and tested, but must be admission-controlled by dimension/work evidence before default use.

## 7. Residual-estimator trade-off

The legacy nested-dimension heuristic had two IOP2 failures at tight quadratic tolerance. Their attempted work was preserved:

```text
RHS evaluations    55,488
JVP vectors       460,976
accepted steps     13,872
rejected steps      6,128
```

The residual-based estimator closes the current canonical IOP2 runs, but for the full-MGS primary it changes 3/12 step records and has median wall ratio

\[
T_{\rm residual}/T_{\rm nested}=1.764.
\]

Therefore the residual estimator is a safer research diagnostic, not a free optimization. IOP2 remains HOLD.

## 8. Comparator interpretation

- Frozen BDF2 covers only 2/12 current RODAS error targets; no general ranking follows.
- Frozen Radau IIA3 covers 10/12 with median speedup 4.188, but uses explicit Jacobian/full implicit reference machinery and does not satisfy the project structural objective.
- Held G1 q1→q2 covers 12/12 but has median speedup 0.211 and remains a structured plugin, not the generic core.

BDF/Radau sources remained byte-frozen.

## 9. Claim boundary

### Supported

- fused phi combinations reduce repeated Krylov work substantially;
- primary path builds no explicit Jacobian and performs no Newton iteration;
- pexprb54s4 maintains the G2 observed fifth-order gate;
- actual three-level code dependency and T1/T4 scientific identity;
- strong same-error gains in linearized stiff/oscillatory and semilinear regimes;
- all failures and rejected work remain visible.

### Not supported

- universal acceleration;
- complete same-error coverage;
- superiority on nonlinear oscillatory problems;
- IOP2 production robustness;
- small-state within-step threading benefit;
- general nonautonomous, nonidentity-mass, DAE, sparse/MPI/GPU support;
- physical-client validation.

## 10. Verdict

```text
FUSED_PHI_INFRASTRUCTURE       PROMOTE
FULL_MGS_PEXPRB54S4            PROMOTE_TO_EXPANDED_SEALED_HOLDOUT
IOP2                           HOLD_RESEARCH_ONLY
GENERIC_ACCELERATED_SOLVER     HOLD
PERFORMANCE_PROMOTION          false
```

The next node must freeze the current pexprb54s4 policy, expand the sealed generic holdout, and implement EPIRK-K/W as distinct sibling methods rather than retuning current thresholds against oscillatory PR.
