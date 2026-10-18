# A12 — 전체 DAG 독립 적대적 감사 및 범위별 판정

판정은 **검토한 인증 API의 제한된 적용 범위에서 PASS**, **범용 production solver 및 실측 속도 승격은 HOLD**이다. 이는 모든 구현을 미완료로 남긴다는 뜻이 아니다. 원래 A01–A12의 12개 ID를 그대로 대조했고, 기존 이론의 유효한 전제는 재사용하면서 새 코드와 그 연결부를 독립적으로 검토했다. 원래 A09의 represented-stage H5 의무와 새 window-shooting client를 별개의 결과로 유지한다.

검토자 `/root/dag_a12_independent`는 해당 구현을 작성하지 않았다. A05/A11은 별도 독립 검토자 `/root/dag_a12_independent/dense_independent`에 배정했다. A04/A07/A08은 구현 작성자가 아닌 root의 독립 검토를 결합했고, root가 직접 수정한 A07 source-asset binding과 handoff runner는 본 검토자가 다시 검토했다. 정확한 검토 파일 SHA-256은 `REVIEW.json`, 범위별 의존성과 제한은 `DECISION.json`에 있다. 이후 소스가 바뀌면 그 변경 부분은 다시 대조해야 한다.

## 원래 12개 노드와 현재 결과

| 노드 | 검토한 실제 결과 | 허용되는 결론과 남은 경계 |
|---|---|---|
| A01 | immutable weighted initial tube, returned-center radius, 두 최신 소스의 통합 | 기존 undamped packet 정리의 전제 아래 local enclosure. allocator 권위와 수치 포함 검사는 별개다. |
| A02 | transactional advance, 고정 physical budgets, exact-duration adaptive subdivision, 실패 이력 보존 | 전체 requested mesh의 원래 궤적 포함. 임의 tolerance에서 성공하거나 별도 dense-query budget를 보장하지 않는다. |
| A03 | exact damping frame, enclosed averaged forcing, 계수 변화 오차, weighted multiwindow carry | zero-carrier rotating frame의 감쇠 궤적 endpoint. 비영 carrier/lab-frame는 typed unsupported다. |
| A04 | 완전 오차 certificate에서만 private admission token 생성 | 원래 budget와 현재 출력이 결합된 admitted Leja action. 임의 operator 또는 estimate-only 경로의 승격은 아니다. |
| A05 | immutable provider와 공통 continuous-residual driver | PR101, collective, Gray–Scott 모델 capability. arbitrary callback와 pure-relative 검증은 지원하지 않는다. |
| A06 | Schur inverse bound 확장, 모든 stage의 선택적 residual witness | exact-sum weights를 가진 frozen stage linear system. global ODE error와 elapsed speed는 별도다. |
| A07 | 실패 arm 보존, source/build/config/embedded asset 결합, achieved-error 비교 | 재현 가능한 진단 비교. empirical reference 및 instrumented timing만으로 production speed를 승격하지 않는다. |
| A08 | 동일 target/operator/budget의 Leja·Laguerre·Chebyshev guarded selection | 실제 admitted operator class의 action. Krylov comparator는 estimate-only이며 historical ABSTAIN은 유지한다. |
| A09 | full represented K의 nonlinear H5와 별도의 nonlinear window endpoint verifier | H5는 stage target, window certificate는 true ODE requested endpoints. 두 권위를 혼합하지 않는다. |
| A10 | 동일 frozen residual의 Bernstein refinement와 A02 opt-in replay | 유효한 bound를 단조 감소시키고 원래 physical gates를 재검사한다. threshold success와 실제 window 절약은 다르다. |
| A11 | 전체 symmetric reaction Jacobian와 contractive diffusion, 실제 inherited tube 연결 | 소유된 periodic Gray–Scott 모델의 continuous certificate. 일반 PDE 또는 큰 격자의 속도 주장은 없다. |
| A12 | 독립 소스/증명/실행 증거 검토 및 범위별 판정 | 전체 노드를 검토했으나 일반 production·실측 speed를 일괄 PROMOTE하지 않는다. |

## 핵심 수학·코드 감사

### A01/A02: 반지름의 소유권과 rejected-state 불변성

`PacketFrozenOutput::from_weighted_radius`는 초기 weighted ball을 private scalar로 고정한다. query는 그 소유자의 값만 사용하고 반환 center의 평가 변위를 포함한 `centered_weighted_radius`를 다음 window에 전달한다. component disk로 매번 확장한 뒤 다시 weighted ball로 축소하지 않으므로 불필요한 반복 차원 계수가 생기지 않는다.

A02는 후보 생성, frozen owner, 원래 mean/mode budget 검사, 선택적 headroom 검사, 출력 복사 및 다음 source center admission을 모두 끝낸 뒤 한 번만 state/time/radius를 바꾼다. 실패한 후보는 이 scientific state를 바꾸지 않으며 원인과 work snapshot은 남는다. 이에 대한 induction은 정확히 원래 초기 ball에서 시작한 원래 nonlinear trajectory의 포함을 준다. midpoint는 단지 representable 후보이며 양쪽의 directed subtraction이 동일한 양수를 주는지 다시 검사하므로 물리 시간의 drift를 숨기지 않는다.

headroom schedule은 ordinary floating-point step-selection heuristic이다. 모든 성공 후보가 원래 outward physical gates를 통과해야 하므로 planner rounding이 허용오차를 약화시킬 수 없다. 기존 budget-starvation 실패와 stricter schedule 수정은 기록을 보존했다. 이 수정은 성공을 강제하기 위한 tolerance 완화가 아니다.

### A03: 감쇠 frame과 coefficient averaging

원래 source는 `a'=(-κ+iω)a+iχ|m|²m`이고 `a=e^(−κt)b`로 변환하면 cubic coefficient는 `χe^(−2κt)`이다. 구간 평균은 정확한 damped moment의 enclosure에서 얻는다. coefficient 평균을 freeze한 오차를 0으로 가정하지 않고 별도 model error로 더한다.

평균 μ를 가진 `[l,u]` 확률변수에 대한 `E|X−μ|≤2(u−μ)(μ−l)/(u−l)≤(u−l)/2`는 절댓값의 chord bound로 증명된다. 이를 시간 평균에 적용한 half-range bound는 유효하다. 전체 transformed flow와 frozen flow는 같은 weighted ball을 보존하므로 cubic forcing norm과 logarithmic gain에 같은 radius를 사용할 수 있다. 동일 주파수인 경우만 common rotation과 cubic flow의 가환성을 이용하여 평균화가 endpoint에서 정확하다고 판정한다. 그 경우에도 represented coefficient displacement는 남긴다.

packet returned-center radius에 model error를 더한 후 damping upper factor로 scale하고, 실제 scaled binary64 center의 displacement를 별도로 합산한다. mean summation displacement와 mode-radius 변환도 포함된다. 매 subwindow에서 물리 source로 되돌아와 반지름을 직접 이어 가므로 damping frame을 잘못 누적하지 않는다. 일반 unequal-frequency bound는 보수적 first-order이며 많은 subdivision이 필요할 수 있다는 한계를 유지한다.

### A06: Neumann 충분조건 밖의 실제 선형 복잡도 확장

정확한 weight sum 1에서 `A=D−sUCW`, `P=D⁻¹`, `S₂=I−sWPUC`이며 Woodbury 항등식으로

`||A⁻¹||w ≤ p+s p² L B_S`,

`p≤1/(1+sκ)`, `L≥3|χ||m|²`, `B_S≥||S₂⁻¹||₂`를 얻는다. 2×2 adjugate의 Frobenius norm은 원행렬과 같으므로 enclosed entries의 magnitude 제곱합과 determinant의 절댓값 하한으로 `B_S`를 구성한 코드는 올바르다. 기존 Neumann 영역은 기존 bound를 유지한다.

후보 factorization의 ordinary rounding은 독립 interval residual `b−Ax`에 모두 드러난다. 따라서 inverse bound와 곱한 witness는 exact frozen mathematical Jacobian의 선형 해를 포함한다. 모든 stage witness는 같은 source/state identity를 검사하고 다음 plain/failed attempt가 stale witness를 무효화한다. exact singular Schur와 non-Neumann dyadic family를 별도로 시험했다. native all-stage 경로의 O(N) 비용과 zero warmed allocations는 해당 범위의 증거이며 ODE global accuracy를 대신하지 않는다.

### A09: represented H5와 실제 ODE endpoint를 구분한 완성

실제 stored alpha/beta의 diagonal/upper entry까지 포함하는 stage target을 검토했다. `W=I−hγJ`의 approximate inverse는 후보로만 쓰며 interval replay의 `ρ≥||I−VW||∞<1`로 진짜 inverse norm을 bound한다. common tube에서 `Df−J`의 전체 derivative를 포함한 nonnegative majorant A를 구성하고, strictly lower B와 나머지 E를 분리한다. `S=(I−B)⁻¹`, `q≥||SE||∞<1`은 floating leakage를 없다고 가정하지 않는다.

후보 residual은 λ에 affine이므로 λ=0,1에서 absolute maximum을 취한 bound가 전체 `[0,1]`에 유효하다. positive series와 tail로 얻은 proposed radii에 대해 `d+A r≤r`와 declared tube inclusion을 마지막에 outward로 재검사한다. 이 self-map과 q 조건은 각 λ의 존재·유일성, smooth implicit branch 및 최종 represented output functional의 포함을 준다. 이것은 연속 ODE truncation certificate가 아니다.

window client는 ordinary sin/cos, split arithmetic 및 vector prefix를 **후보**로 사용한다. 최종 고정 center들에서 독립 A01/A03 local verifier를 병렬로 다시 실행한 뒤 candidate successor와 verified reference center의 거리를 포함한 true-flow defect를 계산한다. outward scalar prefix가 이 defect를 운반한다. native code는 source energy의 고정 ball에 실제 center가 들어감을 확인하고 보수적 `exp(c h)` gain을 쓰므로 exact floating nilpotence나 구현되지 않은 transcendental interval backend에 의존하지 않는다.

별도 theory의 refined commutator도 다시 확인했다. `L=−κI+K`와 cubic homogeneity에서 `C=2κB+C_K`, `C_A=4κ²B+4κC_K+[K,C_K]`, `C_B=[B,C_K]`가 성립하므로 pure damping은 C_B에서 정확히 사라진다. 제시된 `K_A=min(16(κ²+Ω²),4(κ+2Ω)²)βR`, `K_B=24Ωβ²R` 및 finite-step residual integration은 타당하다. factorial convergence는 기존 parareal 문헌과 연결된 조건부 결과이고 새 우선권 또는 실측 속도 주장이 아니다. 보존된 theory snapshot의 'future implementation' 문구는 당시의 기록이며 현재 native implementation 권위는 별도 IMPLEMENTATION/H5 문서와 소스에 있다.

### A10: refinement의 진짜 비용과 권위

각 mode의 exact represented residual coefficient intervals에서 Bernstein bound U_j를 구하고 `sqrt_up(sum_up w_j U_j²)`를 취하면 같은 continuous weighted residual을 포함한다. 기존 bound와 min을 취해도 둘 다 유효하므로 유효성과 단조성이 유지된다. refinement budget 소진 시 best finite bound를 반환하는 경로 역시 유효하다. immutable curve/source/initial tube는 바뀌지 않는다.

현재 알고리즘은 각 dyadic cell 경로를 다시 구성한다. 따라서 per-mode work는 **O(d²(1+D·2^D))**, live extra storage는 **O(d)**이며 degree≤1024, depth≤8이라는 제한을 함께 명시했다. `bernstein_residual_false_rejections_saved`는 residual threshold 통과 수이며 실제 physical endpoint acceptance가 아니다. 기존 A02 fixture의 accepted/rejected counts는 개선되지 않았음을 그대로 보고했다. bound 사이에 budget를 놓은 constructive fixture는 개선 가능한 경우의 존재 증거이고 preregistered speed 실험이 아니다.

## 결합한 독립 검토

A05/A11 상세 검토는 `../../reviews/DENSE_INDEPENDENT.md`에 있다. exact represented quartic의 endpoint는 저장된 `step.y_new`와 같으므로 inherited endpoint radius의 recentering 항이 누락되지 않았다. provider는 residual을 전체 curve에서, logarithmic norm을 실제 enlarged tube에서 평가한다. Gray–Scott의 full symmetric offdiagonal과 periodic diffusion의 음의 준정부호 성질을 사용한다. 이 공통 driver는 세 immutable model family에 generic하며 arbitrary callback 인증을 주장하지 않는다.

A04/A07/A08 상세 검토는 `../../reviews/ROOT_A04_A07_A08.md`에 있다. 본 검토자는 root가 추가한 A07 compile-time asset binding을 별도로 검사했다. 실제 coefficient snapshot과 현재 literal include_str/include_bytes assets가 포함된다. 이는 현재 source inventory에 충분하지만 일반 Rust dependency parser라는 주장은 하지 않는다.

## 발견·수정·남은 개발 경계

- A10의 일반 O(d²) 표기는 depth 재탐색 비용을 누락할 수 있어 실제 O(d²(1+D2^D))로 명확화했다. threshold counter를 physical rejection savings와 분리했다.
- A09의 동일 구현끼리 overlap 검사만으로 target fidelity를 평가하지 않도록 독립 full represented-beta Fraction Gaussian oracle를 요청했다. 작성자는 16×16 linear target의 18개 exact containments와 nonlinear 90-digit 검사·exact radius closure를 추가했다. 고정밀 numerical root는 proof를 대체하지 않음을 명시했다.
- A07 source identity에서 빠졌던 embedded coefficient snapshot을 root가 수정했고, 본 검토자가 실제 경로와 새 hostile contract를 확인했다.
- handoff runner가 before/after manifest를 각각 검사하는 것만으로는 중간 source+manifest 교체를 탐지하지 못하는 provenance 문제를 발견했다. 수정 상태와 정확한 재검토 결과는 REVIEW.json의 R-A12-01에 기록한다.

범위 밖 입력에서 typed refusal은 정상 동작이다. production 전역 승격에 남은 일은 목표 deployment model/grid/horizon/output/tolerance의 명시, 해당 크기에서 원래 budget 성공과 자원 상한 확인, 실제 hardware에서 완전 비용과 독립 timing uncertainty의 확보다. 모델이나 tolerance를 바꿔서 gate를 통과시키는 작업은 허용하지 않는다. 이 보고서는 핵심 기능 구현을 local에 떠넘기지 않으며 local에는 source identity 확인, compile, 지정된 checks, 실제 workload 실행과 작은 환경 수정만 남긴다.
