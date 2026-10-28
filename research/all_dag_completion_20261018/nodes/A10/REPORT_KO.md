# A10: 인증된 연속 잔차에 Bernstein 적응 경계 연결

A10을 실제 `RepresentedPacketCurve`의 연속 ODE 잔차와 immutable `PacketFrozenOutput`에 연결했다. 먼저 기존 계수합 경계를 검사하고, 요청한 잔차 예산을 넘는 경우에만 outward Bernstein 변환과 제한된 dyadic subdivision을 수행한다. A02의 `VerifiedPacketAdaptiveConfig::bernstein_max_depth`가 `Some(D)`이면 기존 물리적 출력/구간별 headroom gate가 거절한 창에서 이 경로를 시도하고 같은 gate를 다시 적용한다. 기존 거절과 계산량은 진단 기록에 남는다.

## 기존 결과 재사용과 이번 변경

기존 T2 (`research/production_audit_20261018/THEOREMS.json`)의 convex-hull 정리와 outward 변환을 재사용했다. 현재 소스에는 estimate-only `compute_certificate`의 후보 domain gate와 8개 Bernstein 검사가 이미 있었다. 이 경로를 새로 만든 것으로 기록하지 않았다. 과거 보고서의 helper-only 판정은 그 시점의 기록으로 보존한다.

이번 추가 사항은 다음과 같다.

- `RepresentedPacketCurve::with_bernstein_residual_bound`: 각 mode의 **정확한 represented curve 잔차 계수 interval**을 사용한다. 반환되는 curve의 계수, source identity, 초기상태, 평균 경계는 원래와 같다.
- `PacketFrozenOutput::with_bernstein_residual_bound`: 초기 component disks 또는 weighted ball의 소유권을 그대로 이동한다. 외부 모델/초기 tube를 다시 주입할 수 없다.
- 기존 잔차 경계와 새 경계의 최솟값을 취한다. 깊이를 모두 소진해도 얻은 유효 경계는 사용할 수 있고 `threshold_met=false`로 남는다. 물리적 gate는 별도로 반드시 적용한다.
- 단일 polynomial degree 1024, depth 8 제한, fallible buffer 예약, 실제 transform/subdivision/copy/예약/실패 계수 기록을 추가했다.
- A02의 optional fallback은 cheap output 거절 뒤 `rho/2`를 refinement의 **휴리스틱 목표**로 사용한다. 이 목표의 달성 여부로 물리적 출력 tolerance를 대신하지 않는다.

## 정리 A10-R: 잔차 경계의 안전한 감소

`tau in [0,1]`에서 frozen residual을 `R_j(tau)=sum_q r_jq tau^q`라 하자. 각 복소 계수는 제공된 rectangle interval에 포함되고 `w_j>0`는 frozen model의 실제 가중치이다. 기존 경계를 `rho_0 >= sup_tau sqrt(sum_j w_j |R_j(tau)|²)`라 하자.

각 mode의 outward Bernstein hull과 subdivision으로 `U_j >= sup_tau |R_j(tau)|`를 얻고,

`rho_1 = min(rho_0, sqrt_up(sum_up_j mul_up(w_j,mul_up(U_j,U_j)))))`

로 정하면 같은 연속 residual의 전 구간 weighted norm은 `rho_1` 이하이다. 또한 `rho_1 <= rho_0`이다.

증명: degree d의 power-to-Bernstein 변환은 `b_i=sum_(q<=i) [C(i,q)/C(d,q)] r_q`이다. 코드의 ratio recurrence는 이 유리수를 outward interval로 포함한다. Bernstein 기저의 각 원소가 비음이고 그 합이 1이므로 polynomial 값은 control point들의 convex hull 안에 있다. 복소 rectangle의 `|Re|+|Im|` upper는 modulus를 포함한다. Dyadic de Casteljau는 같은 polynomial의 각 절반 제한을 정확히 표현하며 모든 평균을 interval로 계산하므로 하위 control polygon에도 포함관계가 유지된다. 부동소수점 interval 폭 때문에 하위 bound 자체가 증가할 수 있어, 각 깊이에서 이미 유효한 bound와 `min`을 취한다. 각 `|R_j|<=U_j`에 양의 가중치를 곱해 합산하고 제곱근을 취하면 새 weighted bound를 얻는다. 두 유효 upper bound의 최솟값 역시 유효하다.

이 경계는 T1의 `B_w(t) <= exp(lambda t)(e_0+t rho)`에 직접 들어간다. frozen curve와 mean bound, 초기 tube가 같으므로 `lambda`, `e_0`와 출력 center의 rounding displacement도 같다. 따라서 반환되는 T1 radius는 증가하지 않는다. 전체 ODE truncation 오차는 정확한 continuous residual 안에 계속 포함된다. 기존 packet tail 항을 제거하거나 다른 모델의 residual로 치환하지 않았다.

## 구체적 정확도 및 비용 결과

`all_dag_a10_bernstein`의 한 mode cubic 모델은 `a(0)=0.5`, `chi=2`, `omega=kappa=0`, `h=0.5`, order/degree 4, Picard sweep 2이다. `initial_weighted_radius=1e-10`을 그대로 유지했다.

| 양 | 계수합 경로 | Bernstein 경로 |
|---|---:|---:|
| 연속 weighted residual upper | 2.60646648294942523e-3 | 2.60530687932307653e-3 |
| endpoint centered weighted radius | 1.96828195665799382e-3 | 1.96740627813520768e-3 |
| 동일 budget 1.96784411739660096e-3 | 거절 | 수용 |

이 budget은 두 계산된 bound 사이에 택한 **구성적 fixture**이다. 별도 benchmark의 사전 고정 tolerance 성공으로 보고하지 않는다. A02의 고정 원래 tolerance 시험은 A02 evidence를 참조한다. 이 예의 감소량은 작으며, 개선 이득은 residual 계수의 상쇄 구조에 달려 있다. 알려진 T2의 `(1-tau)^12`에서는 coefficient bound 4096 대 정확한 supremum 1이라는 큰 차이가 존재한다. 새 검사는 정확한 정수 binomial 계수로 degree 0–16 family와 복소/vector `tau(1-tau)`를 검사한다.

단일 degree `d`, 최대 depth `D`에서 현재 streaming 구현의 작업량은 `O(d²(1+D 2^D))`, 추가 live storage는 `O(d)`이다. 각 cell의 경로를 다시 계산하므로 `D 2^D`를 생략하면 안 된다. 전체 mode residual의 비용은 mode별 이 비용을 합한 값과 weighted norm 집계다. 원래 curve 저장공간은 별도이다. 모든 mode마다 source interval을 복사하고 transform buffer를 예약하는 비용을 계수에 포함했다. Cheap pass이면 transform/예약 비용은 0이다. `bernstein_residual_false_rejections_saved`는 **잔차 threshold를 달성한 횟수**이고, 실제 physical window 수용 횟수는 A02의 acceptance 기록 또는 위 명시적 gate fixture로 확인한다. elapsed speedup은 주장하지 않는다.

## 검증 및 재개

- 새 native 검사 4개 PASS: 실제 nonlinear output gate recovery, unchanged source/mean/initial tube, cheap-pass 무작업, exhaustion의 유효 bound 유지, exact binomial/complex/vector, degree/NaN refusal.
- 21개 물리시간의 mode/mean에 대해 별도 Python `Fraction` Taylor oracle의 disk containment 42개 PASS. native `sin/cos` 한 점 비교는 보조 수치 진단이고, 인증의 근거는 interval proof와 Fraction 검사이다.
- A02 opt-in 통합 검사도 PASS: 같은 1e-4 tolerance와 비영 초기 disks에서 새 Fraction containment 30개 및 기존 120개를 통과했다. 해당 fixture의 accepted/rejected는 16/14로 유지되어 이 입력에 대한 창 거절 감소는 주장하지 않는다 (`../A02/native_with_bernstein_final.log`).
- 변경된 kernel의 기존 8개 Bernstein 회귀 PASS. 이전 대규모 campaign은 다시 실행하지 않았다.
- 첫 compile은 동시 편집 중 `actual_k_homotopy.rs`의 owned/reference mismatch로 실패했다. 해당 owner의 수정 후 통과했다. 원래 failure 로그를 유지했다.
- A12 독립 검토는 weighted residual 집계, budget exhaustion, monotone min의 수학적 타당성을 확인했고, depth 비용과 counter 의미를 명시하도록 지적했다. 위 비용/범위에 반영했다.

Local 재개 명령:

```sh
cargo test --locked --offline -p rodas5p-integrators --test all_dag_a10_bernstein -- --nocapture
cargo test --locked --offline -p rodas5p-integrators --test production_audit_packet bernstein -- --nocapture
cargo test --locked --offline -p rodas5p-integrators --test production_audit_packet_adaptive -- --nocapture
```

다음 성능 실험에서는 동일한 physical budget/output grid에서 A02 `None` 대 `Some(0/1/2)`를 비교하고 transform 비용, 거절 후 재시도, 최종 수용 창, 전체 wall time을 함께 기록한다. 손실이 많은 polynomial에서만 이득이 생길 수 있으므로 default는 현재 opt-in으로 유지한다. A10의 사용 판정은 인증 잔차 refinement 범위의 `PASS_SCOPED`; 전체 production 승격은 A12의 aggregate 판정을 따른다.
