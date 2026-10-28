# N05 후속 이식: 수학적 잔차 상한과 구조 활용

상태: 소스 이식, 독립 수학/API 검토, native 표적 검증 **7/7 PASS**. 일반 nonlinear production 승격은 열지 않는다.

## 구현된 결과

`stage_target_policy.rs`의 `BoundLinearStageModel`은 행렬과 양의 고정 Hilbert 가중치를 소유한다. 외부 callback·사용자 주장·mutable matrix handle을 받아 인증 권한으로 바꾸지 않는다. 생성 시 `S^-1 J S`의 대칭 부분에 outward Gershgorin을 적용하여 dissipativity를 증명한다. 증명되지 않은 입력은 typed error로 거절한다. 이는 dissipative 행렬의 필요조건 검사가 아니라 충분조건 검사이므로 일부 유효한 행렬도 안전하게 거절할 수 있다.

- 일반적으로는 Cayley contraction으로 normalized raw stage residual → output 및 → embedded estimator 전달 상한을 구한다. 비정상 dissipative 행렬과 회전 행렬도 포함한다.
- 고정 가중치가 동일하고 원래 행렬이 정확히 대칭이면 negative-real spectral calculus를 선택한다. `R=(I-hγJ)^-1`의 스펙트럼은 `[0,1]`에 있으므로 각 전달 다항식의 Bernstein 계수와 깊이 4의 dyadic subdivision으로 supremum을 감싼다.
- 두 경우 모두 실제 runtime이 파싱한 tableau의 binary64 수치를 정확한 실수로 해석하며, 전체 계수 recurrence와 norm 변환을 outward interval로 계산한다. 외부에서 coefficient majorant를 임의로 주입할 수 없다.
- `LinearStageTargetPlan`은 모델·초기 상태·step·tolerance·원래 controller budget에 바인딩된다. normalized residual target 및 일반 GMRES의 **absolute raw Euclidean residual target**을 제공한다. 상대 tolerance나 left-preconditioned residual target과 혼용하지 않는다.
- `assess(U, returned)`는 실제 8개 stage와 실제 반환 center에서 residual을 직접 재계산한다. output perturbation, embedded estimator contamination, 반환 center 반올림 차이를 모두 계상한다. 임의의 잔차 스칼라를 증거로 받지 않는다.

일반 lagged callback의 비선형 Jacobian drift는 이 모델의 전제가 아니다. 따라서 기존 heuristic target을 새 theorem target으로 자동 교체하지 않았다. 생산 경로의 기본 routing은 그대로이며, frozen-linear 전용 client가 안전하게 사용할 수 있는 실제 planning/assessment API를 이식했다.

## 정리와 코드 대응

고정 norm을 `||v||_s=(Σ_i(v_i/s_i)^2/n)^(1/2)`로 둔다. `a=hγ>0`, `R=(I-aJ)^-1`, `J`가 이 norm에서 dissipative이면 `C=2R-I`는 contraction이다. `v=Rx`에 대해 `x=v-aJv`, `Cx=v+aJv`이고 두 제곱 norm의 차이가 `4a<v,Jv>≤0`이므로 성립한다. 이 증명은 normality를 요구하지 않는다.

RODAS5P U 좌표의 정확 stage residual을 `r_i=aJ(y0+Σ_{j<i}A_ij U_j)+γΣ_{j<i}C_ij U_j-U_i+aJU_i`로 정의하면, stage error의 lower-triangular 전달 block은 `T_ij=γ C_ij R+A_ij(R-I)`이다. 선택한 출력 계수 벡터 `b`에 대해 뒤에서부터 `p_i=b_i+Σ_{j>i}p_j T_ji`, `m_i=R p_i`를 계산한다. 마지막 stage의 embedded estimator에는 `b=e_8`을 별도로 적용한다. 각 polynomial의 Cayley coefficient 절댓값 합이 그 operator norm의 상한이다. 대칭 negative-real 경우에는 각 polynomial의 Bernstein 상한을 쓴다. stage별 supremum을 먼저 구하고 합치므로 `sup Σ`와 `Σ sup`를 잘못 바꾸지 않는다.

새 exact Fraction oracle은 represented coefficient를 직접 읽고 다음 값을 구했다. 아래 표시는 근사이며 JSON에는 정확한 유리수를 보존한다. Runtime의 outward 값은 이에 대한 상한이다.

| 전달 norm 상한 합 | Cayley dissipative | 대칭 negative-real Bernstein |
|---|---:|---:|
| normalized stage residual → output | 31.334688503951696 | 23.334835830430087 |
| normalized stage residual → embedded estimator | 17.29375435880804 | 8.254857835165163 |
| 두 budget의 합 | 48.628442862759736 | 31.58969366559525 |

이것은 상한의 개선이며 측정된 wall-time speedup을 의미하지 않는다. 기존 `0.58` 첫-stage target이 회전 `z=i`에서 `|m1|≈0.85190467449`를 감싸지 못한다는 exact 반례는 재사용한다. 새 모델은 회전을 negative-real 경로에 넣지 않는다.

원래 controller weight를 `d_i=atol+rtol max(|y0_i|,|returned_i|)`라 하고, `c=min_i((atol+rtol|y0_i|)/s_i)>0`의 하한을 구한다. 그러면 `||v||_d≤||v||_s/c`이다. stage residual bound를 `r_i^+`, output/estimator 전달 상한을 `M_i,E_i`, 실제 center의 조립 오차를 `δ_center`라 하면

`output_error ≤ Σ_i M_i r_i^+ + δ_center`,

`estimator_error ≤ Σ_i E_i r_i^+`.

target은 `θ c/[8 max(M_i+E_i,1)]`, `0<θ≤0.1`로 정한다. 잔차 target 만족만으로 center rounding까지 통과했다고 선언하지 않고 실제 assessment를 다시 요구한다. 최종 combined metric은

`||U8_candidate||_d + (output_error+estimator_error)/c`.

이 값이 1 이하일 때 **candidate-end weighting에서 ideal-stage embedded estimate와 stage perturbation**이 원래 unit controller budget 안에 들어간다. 이것을 exact ODE의 truncation-error enclosure 또는 전역 정확도 증명으로 부르지 않는다. 원래 tolerance를 늘리지 않았고, unbounded nonlinear drift를 반영하지 않은 채 linear budget만 절반 적용하는 shortcut도 사용하지 않았다.

raw residual `c-WU`와 normalized residual의 관계는 `r_normalized=hγ r_raw`이다. `||v||_s≤||v||_2/(min(s)√n)`이므로 absolute Euclidean target은 `target_normalized min(s)√n/(hγ)`의 directed lower bound다. preconditioned residual에는 이 공식을 그대로 적용할 수 없다.

## 실제 저비용 resolvent 경로

`StalenessRule::Diagonal`을 두 lagged stage solver에 opt-in으로 이식했다. 저장된 `J_P`가 정확히 diagonal이고 각 diagonal이 nonpositive인지 직접 확인한다. `a=h_Pγ`, `R_ii=(1-a J_P,ii)^-1`를 outward 계산하고

`q ≥ max_i Σ_j |R_ii[(1-h_P/h)δ_ij+a(J-J_P)_ij]| s_j/s_i`

를 계산한다. 이는 정확한 ideal-matrix `I-P^-1W`의 norm 상한이며 row별 cancellation을 보존한다. 비대각 old Jacobian·양의 diagonal·zero weight·overflow·증명 불가인 경우 `+∞`를 내어 기존 direct refresh로 돌아간다. `1-q`는 directed lower rounding을 쓴다.

**보장되는 비용 감소:** 기존 `Resolvent` witness가 refresh마다 요구한 n개의 추가 triangular solve를 이 경로에서는 정확히 0개로 줄인다. 별도 dense inverse matrix도 할당하지 않는다. 저장 `J_P`와 현재 dense matrix를 검사하는 O(n²) 비용은 남으며, `diagonal_matrix_entries`에 실제 검사한 entry pair 수를 기록한다. 실패/거절된 검사도 `diagonal_checks`, `diagonal_refusals`에 남는다. 전체 LU application accounting은 기존 `direct_solve_calls`를 유지한다. 이 구조 검사는 행렬 구성이 dense인 현 API 때문에 O(n)이라고 주장할 수 없다.

ideal diagonal witness의 outward q가 **floating LU backward error**를 자동 포함하지는 않는다. 기존 generic residual monitor 역시 ordinary arithmetic이다. 따라서 이 opt-in rule 전체를 certified production route로 승격하지 않는다. 구조 확인이 유효한 문제에서 inverse-witness 비용을 실제로 제거하는 좁은 구현이다.

## 사용 계약과 로컬 검증

```rust
let model = BoundLinearStageModel::new(matrix, fixed_hilbert_scales)?;
let plan = model.plan(&y0, h, atol, rtol, 0.1)?;
let absolute_raw_l2_targets = plan.raw_euclidean_residual_targets()?;
// A frozen-linear GMRES client uses these absolute raw targets.
// U must be the actual eight U-coordinate stages; returned the actual center.
let receipt = plan.assess(&u_stages, &returned)?;
let allowed = receipt.within_stage_budget() && receipt.within_controller_budget();
// If false: refine/recompute the stages or reduce the step; never inflate tolerance.
```

현재 이 API는 stage 후보와 exact frozen-linear 모델을 결합하는 실질적인 구현이며, 임의 `OdeProblem` callback에 자동 authority를 부여하는 장치는 없다. 일반 nonlinear 이식을 열려면 stage tube의 segment-average Jacobian drift를 검증하는 provider가 추가되어야 한다. 현 소스와 테스트를 로컬에서 다시 설계할 필요는 없으며, 후속 client는 위 계약으로 연결하고 NCP 환경에서 동일 정확도·전체 비용을 측정하면 된다.

실행 완료한 표적 명령:

```text
cargo test --locked --offline -p rodas5p-integrators --lib n05_tests
cargo test --locked --offline -p rodas5p-integrators --test n05_diagonal_staleness_contracts
```

Rust 1.94.1, locked/offline, `final-target`, debug information 0, incremental 0, `-j1`에서 planner unit 4개와 diagonal integration 3개가 첫 실행에 모두 통과했다. 로그와 exit code는 `evidence/integrator/n05_planner_contracts.{log,exit}`, `evidence/integrator/n05_diagonal_contracts.{log,exit}`에 있다. 무관한 기존 test-support/packet fault-enum의 dead-code warning은 로그에 그대로 남겼다.

검증 목록: immutable model/scope guard, 회전 및 nonnormal 반례, 독립 exact Bernstein/Cayley 상한, forward-stage solve와 실제 perturbation 비교, raw residual의 hγ 단위 변환, estimator contamination과 반환 center charge, zero scale/NaN/overflow refusal, diagonal rule 실제 적분과 모든 inverse-solve/scan accounting, unsupported structure direct fallback.

## 남은 개발 단계

1. NCP에서 `Off/Bounded/Resolvent/Diagonal`을 같은 problem·tolerance·output schedule에서 비교한다. 필요 성능 지표는 full runtime, 전체 LU solve 수, entry scan 수, accepted/rejected step 및 reference 오차다. factorization 수만으로 승격하지 않는다.
2. frozen-linear GMRES client의 실제 candidate를 `LinearStageTargetPlan`에 연결한다. public planner/assessment와 원래 controller budget은 이미 구현되어 있다. 모델·norm이 변하면 새 immutable model을 만들고, 기존 witness를 재사용하지 않는다.
3. 일반 비선형 extension은 `ΔJ_i=∫_0^1 J(Y_i+sΔY_i)ds−J0`의 stage-tube bound를 소유하는 provider를 추가한다. vector mean-value theorem의 단일 `J(ξ)` 가정을 쓰지 않는다. strict-lower stage coupling의 유한 nilpotent correction과 returned-center enclosure를 동일 budget에 더한다.
4. diagonal 외 band/low-rank resolvent는 `I-PB`의 outward defect와 Neumann correction 또는 구조적 inverse certificate를 갖춰야 한다. 단순 computed inverse norm으로 certificate를 대체하지 않는다.

근거: `evidence/integrator/n05_bernstein_oracle.{py,json,log}`, `evidence/theory/N05_INDEPENDENT_REVIEW.json`. 과거 published stage-sensitivity 증거와 기존 audit 결과는 재사용하며 새 메커니즘에 필요한 범위만 검증한다.
