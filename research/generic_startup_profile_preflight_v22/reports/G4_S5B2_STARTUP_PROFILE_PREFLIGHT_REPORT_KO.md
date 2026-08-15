# G4-S5B2 Startup-Profile Preflight Report

## 1. Current DAG node

G4-S5B1은 R-JF-default와 E-K-first asymmetric policy 모두가 active-switch gate를 넘지 못한 뒤,
첫 accepted state에서 actual E Arnoldi/R GMRES prefix를 한 번 비교하고 trajectory 전체의 profile을
고정하는 G4-S5B2 계약을 열었다.

이번 단계에서는 실제 prefix code를 새로 연결하기 전에 다음 필요조건을 검사했다.

> 완전한 미래 정보를 가진 최적 startup classifier조차 one-time fixed-profile state machine에서
> calibration/holdout regret와 startup overhead gate를 통과할 수 있는가?

답은 **아니다**. 따라서 actual bake-off 구현은 정보 이득 없이 구조만 늘리는 작업이 되므로 중단했다.

## 2. Evidence basis

- G4-S1 hard corpus accepted-step telemetry
- G4-S5B0 transition-atlas accepted-step telemetry
- G4-S5B1의 corrected unit-physical-time cost authority
- actual reusable prefix wall measurements inherited from G4-S4/G4-S5B1
- seven paired canonical timing campaigns

Candidate equations, controllers, BDF/Radau comparators, physical clients는 변경하지 않았다.

## 3. Contracts and dimensions

Method cost authority는 current-step raw wall이 아니라 동일 physical interval을 덮는 비용이다.

\[
\Gamma_m=\frac{T_m}{\widehat h_m},\qquad
C_{m\to h_R}=\Gamma_m h_R.
\]

Trajectory regret는

\[
\mathcal R_{\rm traj}
=\frac{C_{\rm selected}+C_{\rm prefix}+C_{\rm fallback}}
       {C_{\rm step-local\ feasible\ oracle}}
\]

로 정의했다. 모두 무차원이다.

## 4. Hypotheses tested

### H1 — first-step perfect oracle

첫 accepted state의 **실제 future-resolved cost ratio**를 안다고 가정해 E/R profile을 고른다.
이것조차 calibration median/P95 regret `1.407` / `2.541`,
holdout `1.386` / `2.895`로 실패했다.

### H2 — perfect complete-future fixed-profile oracle

각 trajectory의 모든 future step을 본 뒤 E profile과 R profile 중 더 싼 것을 고른다.
이는 어떤 realizable startup classifier보다 강한 upper bound다.

| Split | Median regret | P95 regret | P95 startup overhead | Aggregate speedup vs static R |
|---|---:|---:|---:|---:|
| Calibration | 1.154 | 2.234 | 15.03% | 1.880x |
| Holdout | 1.322 | 2.895 | 135.08% | 2.033x |

Median/P95 regret gate를 모두 실패한다. 따라서 one-time fixed profile은 classifier quality와 무관하게 구조적으로 탈락한다.

### H3 — perfect one-sided startup profile

E를 고르면 R prefix를 만들지 않고, R을 고를 때만 E prefix 비용을 부담하는 더 강한 upper bound다.

| Split | Median regret | P95 regret | P95 startup overhead |
|---|---:|---:|---:|
| Calibration | 1.131 | 2.230 | 15.03% |
| Holdout | 1.009 | 2.510 | 5.18% |

Holdout P95 regret `2.510`는 여전히 1.5 gate를 넘는다. 원인은 prefix가 아니라 regime drift다.

### H4 — optimistic safety-triggered re-entry

Initial profile은 future-perfect하게 고르고, E failure 뒤 R로 fallback한 다음 cooldown 후 E-admissibility가 회복되면 E로 재진입한다.

Selected cooldown: `1`.

| Split | Median regret | P95 regret | P95 startup overhead | Aggregate speedup |
|---|---:|---:|---:|---:|
| Calibration | 1.051 | 1.170 | 15.03% | 3.016x |
| Holdout | 1.005 | 1.108 | 5.18% | 2.967x |

Regret gate는 통과하지만 overhead gate가 calibration과 holdout P95에서 실패한다.

## 5. Failure geometry

두 failure mechanism이 분리됐다.

1. **Regime drift**: van der Pol, nonautonomous forcing, HIRES, rotating nonnormal에서 한 profile을 고정하면 future winner change를 추적하지 못한다.
2. **Short-horizon amortization failure**: 3–6 accepted-step trajectory에서 startup prefix가 전체 solver 비용의 큰 비율을 차지한다.

Short trajectory statistics:

```text
rows with steps <= 6         6
median startup overhead      15.44%
P95 startup overhead         169.63%
long-trajectory median       1.04%
```

## 6. P0 architectural conclusion

Perfect future profile choice is an upper bound on every actual startup classifier under the same state machine.
Because that upper bound fails, implementing the actual dual-prefix bake-off cannot repair the contract.

\[
\boxed{\text{startup-only fixed profile}\Rightarrow\text{P0 architecture failure}}
\]

The reusable prefix kernels themselves remain valid. Their correct placement is event-triggered re-entry, not one-time permanent profile choice.

## 7. Final disposition

```text
G4_S5B2_STARTUP_ONLY_CONTRACT    REJECT
G4_S5B2_ACTUAL_PREFIX_BAKEOFF    BLOCKED / NOT IMPLEMENTED
G4_S5B2_PREFIX_KERNEL_ASSET      PRESERVE
G4_S5B2_ACTIVE_SWITCHING         HOLD
```
