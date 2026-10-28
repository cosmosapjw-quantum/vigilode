# A02-TXN-01: 검증된 κ=0 궤적의 동결 연구 계약

현재 판정: **READY_FOR_BOUNDED_IMPLEMENTATION** (A02-TXN-01만). v1은 AT08 arithmetic-overflow fixture와 구현 계약의 C1/C2에서 보류됐으며, 이전 원문·결정·review handoff는 `evidence/continuation_a02_research/supplement_01/`에 그대로 보존했다. Supplement 02의 frozen V2 입력(1679dd7542028d3c103b321ca4293d1fcf4aa4f7622ee663c3cef3ba5557abb3)과 overlay에 대한 독립 read-only delta review는 `READY_FOR_BOUNDED_IMPLEMENTATION`이고 남은 contract blocker가 없다. 근거: `evidence/continuation_a02_research/supplement_02/INDEPENDENT_DELTA_REVIEW.json`. 구현 승인은 해당 제한된 단위에만 적용한다. A02 수치 실행과 과학적 admission은 아직 HOLD다. 원래 A02 전체 trajectory 목표와 모든 acceptance 의무는 유지한다.

## 1. Source와 재사용 조건

- 연구의 동결 source base: `6b722a918deb0ab178b8e77090cac1280dbb3051`, 브랜치 `audit/production-readiness-20261018`.
- 작성 시 관측한 HEAD: `7e2873887ed6038b47b667c976435810851c5dba`. 이 후속 커밋은 두 `weighted_radius()` accessor의 주석만 추가했다. base/current SHA-256과 실제 diff는 `evidence/continuation_a02_research/SOURCE_BINDINGS.json`, `BASE_SHA256SUMS.txt`, `CURRENT_SHA256SUMS.txt`, `accessor_documentation.diff`에 따로 보존한다. 바이트 변화와 수식·실행 의미 변화는 구별한다.
- 원전은 `THEOREMS.json`의 T1, `docs/reviews/20261018_production_audit/THEORY_KO.md`의 T1a–c, 현재 `packet_curve.rs`, `packet_curve_output.rs`, `packet_trajectory.rs`다. 기존 T1의 독립 검토 범위는 재사용하되, 아래 새로운 운송 조합과 시간 lift는 별도로 검토한다.
- `ω_j = width * ξ_j`, `κ=0`, `χ∈R`, 저장된 `w_j>0`, 정확한 실수 합 `S=Σw_j`를 유지한다. carrier는 metadata다. 가중치 합을 1로 대체하지 않는다.
- 원래 `TrajectoryConfig.mean_tolerance`, `mode_tolerance`는 양의 유한 absolute 물리 tolerance다. 같은 요청·동일 grid에서 기존 값을 그대로 쓰며 새로운 rtol/atol이나 통과용 threshold를 만들지 않는다. 초기 uncertainty의 기존 outward `S·E_global≤1` admission도 유지한다.
- A01-LCERT의 독립 `PASS_SCOPED`가 zero-tube를 포함한 nonlinear κ=0 represented local flow, 동일 곡선의 continuous residual, 실제 반환 중심 displacement와 immutable owner/view를 포괄해야 한다. 전체 A01 legacy 경로가 HOLD라면 정확히 이 하위 scope를 DAG의 소비 의존성으로 명시해야 한다. `roundoff_enclosed=false` 결과를 증거로 사용하지 않는다.
- 이 계약의 독립 증명 검토 후에만 하위 구현 작업을 dispatch한다. 검토 전 code/test 변경은 허가하지 않는다.

## 2. 문제와 accepted-state 불변식

자율계는 `a'_j=iω_j a_j+g(m(a))`, `g(m)=iχ|m|²m`, `m(a)=Σw_j a_j`, `||a||_w²=Σw_j|a_j|²`다. 원래 초기 product tube에서 outward 계산한

`E_global ≥ Σw_j(|c_0,j|+r_0,j)²`, `e_0 ≥ sqrt(Σw_j r_0,j²)`

를 보존한다. T1a로 실제 모든 허용 궤적의 에너지는 전시간 `E_global` 이하이다. `E_global`은 상속 marginal disks로 매번 다시 키우지 않는다.

Private `VerifiedTrajectoryState`는 accepted `(t_k,c_k,r_k,e_k,E_global,next_observation)`와 고정된 모델 identity를 가진다. 각 accepted 상태에는 다음 두 부등식이 **동시에** 성립해야 한다.

`|a_j(t_k)-c_k,j|≤r_k,j`, `||a(t_k)-c_k||_w≤e_k`.

`e_k`는 실제 committed binary64 중심 `c_k`를 기준으로 한다. 이는 independent marginal disks와 서로 다른, 함께 유효한 정보다. public mutable 인증 필드나 임의 사용자 scalar를 통해 이 불변식을 만들 수 없어야 한다.

### 반복 product-disk 확장의 실패 기전

T1의 weighted 반경 `B`를 `r_j=B/sqrt(w_j)`로 바꾼 뒤 이를 independent disks로만 다음 window에 넘기면

`sqrt(Σw_j r_j²)=sqrt(N)·B`.

이는 χ=ω=0, residual=0, evaluation error=0에서도 발생하는 정보 손실이다. 예를 들어 `w_j=1/N`이고 초기 marginal 반경이 모두 `r`이면 진짜 불확실성은 그대로지만, 위 변환을 반복한 계산 반경은 window마다 `sqrt(N)`만큼 증가한다. h를 줄이는 것만으로 없앨 수 없다. 별도 `e_k`를 보존하고 marginal radii에서 다시 계산한 값으로 대체하지 않는다. 이 항등식은 수학적 반례이며 실행 결과로 표시하지 않는다.

## 3. 같은 represented curve와 candidate mean의 대응

각 window에서 accepted 중심 `c_k`를 초기값으로 packet/candidate를 생성하고 `RepresentedPacketCurve::freeze`로 유한 binary64 계수의 mode 곡선 `Y_k(u)`를 동결한다. `Y_k(0)=c_k`는 저장된 상수항으로 성립한다.

현재 `packet_curve.rs`는 그 mode 계수에서 outward `M_Y=Σw_jY_j`를 구성하고, 그 `M_Y`의 cubic force와 물리시간 미분으로 `R_Y=Y'_k-f(Y_k)`를 계산한다. 따라서 candidate polynomial `P`와 `M_Y`의 동치를 가정하지 않는다. P의 생성·moments·forcing·재구성 오차는 실제 Y의 residual에 이미 포함된다. 별도의 exponential-series tail을 더해서 중복 계산하지 않는다.

해당 곡선을 **zero initial radii**의 `PacketFrozenOutput`으로 소유한다. 현재 중심에서 시작한 정확한 해를 `b_k`라 하면 A01 local view는 모든 지원 `u∈[0,H]`에서 `b_k(u)`를 실제 반환 mode/mean 중심 주변으로 감싼다. 그 weighted accessor `B_local(u)`는 `||b_k(u)-Y_k(u)||_w`의 상계이며, 반환 binary64 중심과 Y의 displacement를 포함하지 않는다.

## 4. 상속 weighted 오차의 T1 수송

실제 해 a와 정확한 center flow b_k의 초기 에너지 상한은 각각 `E_global`, `E_center,k≥Σw_j|c_k,j|²`다. 모두 outward 계산하고

`E_* = max(E_global,E_center,k)`, `λ_flow=S²|χ|E_*`, `G(u)=exp(λ_flow·u)`

를 둔다. T1a와 weighted Cauchy–Schwarz로 두 exact flow의 mean은 `sqrt(S E_*)` 이하이고, 두 상태를 잇는 segment의 mean도 같은 상계 이하이다. T1b의 logarithmic norm으로

`||a(t_k+u)-b_k(u)||_w ≤ G(u)·e_k`.

이는 두 exact solution 사이의 흐름 gain이다. `Y_k`의 energy가 `E_*` 이하라는 가정을 추가하지 않는다. b_k와 Y_k의 차이는 별도의 A01 local certificate가 `max(S E_center,k,M_Y²)`를 포함해 감싼다.

### Component Duhamel 상계

`δ=a-b_k`라 하면

`δ_j(u)=exp(iω_j u)δ_j(0)+∫_0^u exp(iω_j(u-s))[g(m(a(s)))-g(m(b_k(s)))] ds`.

실수 미분으로 `Dg(m)v=iχ(|m|²v+2m Re(conj(m)v))`이므로 `||Dg(m)||₂≤3|χ||m|²`. 두 mean을 잇는 segment에서 `|m|²≤S E_*`이고 `|m(δ)|≤sqrt(S)||δ||_w`이다. 따라서

`|g(m(a))-g(m(b_k))|≤Q·G(s)·e_k`, `Q=3|χ|S^(3/2)E_*`.

κ=0이므로 `λ_flow≥0`, `G(s)≤G(u)`이다. 따라서 동시에 유효한 두 상계의 최소값

`p_j(u)=min(G(u)e_k/sqrt(w_j), r_k,j+u Q G(u)e_k)`

를 쓴다. χ=0에서는 두 번째 항이 기존 `r_k,j`를 보존한다. 작은 가중치를 가진 mode를 첫 항만으로 불필요하게 확대하지 않는다. 모든 수치 항은 outward이며, overflow/vanished denominator는 typed refusal이다. 하나의 계산이 실패한 경우 유효한 다른 bound를 선택하려면 그 선택의 독립적인 산술 증거와 work를 남겨야 한다.

정확한 local time에서 반환 반경은

`R_j(u)=R_local,j(u)+p_j(u)`,

`R_mean(u)=R_local,mean(u)+sqrt(S)G(u)e_k`.

선택적으로 inherited mean perturbation은 `min(sqrt(S)G e_k, Σw_j p_j)`로 줄일 수 있으나 같은 returned local mean center를 유지하고 선택 비용을 기록한다.

## 5. 실제 global 관측시각의 temporal lift

입력 binary64 `t_request`, `t_k`는 각각 정확한 실수로 해석한다. 정확한 차 `d=t_request-t_k`는 일반적으로 binary64 하나로 표현되지 않는다. 단순히 `u=fl(d)`를 계산한 뒤 u의 enclosure에 `t_request` label을 붙이면 오류다.

Outward interval subtraction으로 `d∈D=[d_lo,d_hi]`를 만들고, finite binary64 `u∈[0,H]`와 `Δ≥max(|d_lo-u|,|d_hi-u|)`를 outward 구한다. D 전체와 u는 represented horizon `[0,H]` 안에 있어야 한다. 정확한 physical span과 polynomial horizon H를 별도 필드로 보존한다. interval이 범위를 벗어나거나 시간을 전진시킬 수 없으면 typed refusal이다.

`Ω_max≥max_j |width·ξ_j|`를 outward 계산한다. 실제 trajectory 에너지 상한에 대해

`||f(a)||_w ≤ Ω_max sqrt(E_global)+sqrt(S)|χ|(S E_global)^(3/2)`

`=sqrt(E_global)(Ω_max+S²|χ|E_global)=V_w`.

첫 항은 weighted diagonal skew action의 operator norm, 두 번째 항은 모든 mode에 동일한 g가 더해짐에서 따른다. 적분하면 `||a(t_k+d)-a(t_k+u)||_w≤V_w Δ`다. 따라서 `K_time=V_w Δ`를 추가하여 원래 global label에 대해

`R_j(t_request)=R_local,j(u)+p_j(u)+K_time/sqrt(w_j)`,

`R_mean(t_request)=R_local,mean(u)+sqrt(S)(G(u)e_k+K_time)`

가 유효하다. exact-offset의 경우 Δ=0이다. 기존 exact-offset gate는 staging 검증에 재사용할 수 있으나 최종 A02의 observation domain을 dyadic grid로 축소하지 않는다. overflow/subnormal/time-domain 실패는 enclosure 또는 typed refusal로 처리한다.

## 6. Window 연결과 returned-center weighted seed

Endpoint view가 반환한 중심 `C_j`와 exact Y_k(u)의 차를 interval evaluation으로 감싸 `δ_j≥|Y_k,j(u)-C_j|`, `δ_w≥sqrt(Σw_jδ_j²)`를 outward 구한다. Local view의 `weighted_radius()`만으로는 이 정보가 없다.

`e_next = outward[B_local(u)+G(u)e_k+K_time+δ_w]`.

새 중심은 이 view의 **같은 C_j**이고 새 component radii는 같은 global endpoint의 `R_j`다. `E_global`은 원래 초기값을 그대로 보존한다. Constructor admission/identity를 통과한 다음 모델은 같은 ξ,w,width,χ,carrier,κ와 C_j를 사용한다.

δ_w 구현 선택은 독립 검토에서 고정한다. (a) A01에 좁은 returned-center weighted accessor를 추가하거나, (b) curve를 owner로 옮기기 전에 같은 u의 ordinary/interval evaluation으로 δ_w를 계산하고 이후 view 중심과 bit identity를 검사한다. (b)의 중복 evaluation도 비용에 포함한다. 이미 δ를 포함한 marginal radii를 norm으로 축약해 새 e를 만드는 방식을 주 운송으로 쓰지 않는다.

각 local Y는 C¹이다. 다음 curve의 상수항 C는 직전 exact Y(endpoint)와 다를 수 있다. 이 작은 연결 jump는 δ_w와 새 initial weighted seed로 감싼다. 전체 stored reconstruction이 단일 C¹ polynomial이라는 주장 대신, window별 continuous residual과 명시적으로 감싼 연결 오차를 합성한다. 모든 실제 시간에서 true solution이 포함된다는 명제와 sampled center 사이 임의 interpolation의 정확도는 구별한다.

## 7. Transaction, 출력 identity, 비용

첫 bounded 구현 `A02-TXN-01`은 전체 nonlinear κ=0 window composition의 핵심을 대상으로 한다. 권장 API는 `prepare_verified_window(accepted_state,...)`와 `advance_verified(state,window,requested_grid,workspace,work)`다. 외부에서 임의 weighted seed를 삽입하는 public 경로를 허용하지 않는다.

1. Immutable accepted identity로 trial packet/candidate/curve/owner를 준비한다. Size overflow, malformed request, NaN/Inf와 unsupported damping은 typed refusal이다.
2. 요청 grid는 입력 그대로 보존한다. 점마다 원래 global time, owner/window identity, local u, temporal lift와 같은 view의 centers/radii를 함께 묶는다. `MeanOnly`도 endpoint 운송에 필요한 모든 mode를 내부적으로 계산한다.
3. 모든 requested observations 및 endpoint를 private trial storage에서 평가한다. `mean_tolerance`와 모든 `mode_tolerance`는 원래 값 그대로 각각 검사한다. Endpoint 통과를 전체 관측점 통과로 대신하지 않는다.
4. Constructor/identity, 모든 tolerance, center/radius finite 검사와 endpoint seed 구성이 성공한 다음 한 번에 commit한다.
5. 실패하면 accepted time, modes, component/weighted radii, E_global, 기록된 outputs, cursor를 bitwise 보존한다. 시도 work와 rejection count는 남긴다. 재시도 감소 h는 accepted state와 분리한다. 재현되는 입력/인증 실패를 무의미하게 반복하지 않고 원인과 typed refusal를 보존한다.
6. 성공한 window owner를 보존하여 관측점 사이 arbitrary-time query도 동일한 continuous residual로 감싼다. Record와 owner의 immutable identity binding을 깨는 mutable public descriptor를 만들지 않는다.

Outward energy/speed/gain/seed, exact time arithmetic, 각 source comparison, copied scalar, setup allocation 요청, coefficient/residual, 모든 관측점과 endpoint 평가, 중복 displacement 평가, 거절 비용을 누적 기록한다. Validation-only comparisons와 산술 counter의 의미를 섞지 않는다. Counter별 성공과 첫/중간 실패의 정확한 delta를 source에서 유도하고 검사한다. `total_packet_terms`를 FLOP 또는 전체 runtime으로 해석하지 않는다.

Preallocated warmed query/advance의 zero allocation은 같은 allocator의 `CollectiveModel::clone` 세 Vec 양성 대조군 검출 후에만 주장한다. 현재 packet compiler/candidate와 retained owner setup은 할당하므로 그 비용을 포함해 보고한다. 전체 trajectory allocation-free 승격에는 별도 preallocation 구현·관측이 필요하며 local warmed 결과로 대체하지 않는다. Wall timing은 `DIAGNOSTIC_ONLY`다.

### Window 시작 seed와 commit 세대의 불변성 (C2 보완)

각 `PreparedVerifiedWindow`와 성공 후의 `RetainedVerifiedWindow`는 private immutable `WindowStartSeed`를 **소유**한다. 이 seed는 `lineage_id`, `accepted_generation`, 시작 global binary64 시각 `t_k`, 시작 binary64 중심 `c_k`, component radii `r_k`, 반환 중심 기준 weighted seed `e_k`, 고정 원래 `E_global`, 해당 시작 중심에서 outward 산출한 `E_center,k`, static model identity의 ξ/w/width/χ/carrier/κ 비트, 이 seed에서 만든 owner/curve identity, physical end의 global binary64 시각, 그 끝과 `t_k`의 정확한 차를 감싼 interval, represented horizon `H`, 동결 config/grid identity를 포함한다. physical end와 H는 같은 scalar로 대체하지 않는다. Vec/slice가 필요한 데이터는 private 소유 저장소 또는 변경 불가능한 공유 owner에 보존한다. 이후 live accepted state의 배열을 가리키는 가변 참조나, 현재 상태에서 읽어오는 getter를 historical seed의 대용으로 쓰지 않는다. `E_center,k` 역시 과거 중심에서 도출한 값을 유지한다.

모든 historical global query는 이 window의 시작 seed로만 D, u, Δ, G, p_j, K_time과 출력 identity를 구성한다. 두 번째 이후 window가 commit되어 live `(t,c,r,e)`가 바뀌어도 첫 window query에는 첫 seed를 쓴다. 연속 residual owner만 보존하고 시작 uncertainty/gain/time-origin을 버린 record는 full trajectory certificate가 아니다. Retained window와 그 seed는 외부에서 생성·수정·재결속할 수 없어야 한다. `MeanOnly`가 공개 출력에서 mode vectors를 생략해도 이 내부 seed는 같은 정보를 보존한다.

Prepare는 시작 accepted generation과 seed 전체를 private token으로 묶는다. `advance_verified`의 commit 직전에는 같은 lineage/generation이고 시작 `t,c,r,e,E_global`, model/config/grid binding이 현재 accepted 상태와 bitwise 일치하는지 확인한다. 토큰의 generation만 재사용하거나 같은 scalar time으로 상태 동치를 추정하지 않는다. 다른 window가 이미 commit되었으면 이전 준비 결과의 commit/replay는 typed `stale prepared window` refusal이다. 검사를 거절한 work도 누적하지만 accepted snapshot/cursor/history는 보존한다. generation 증가의 checked overflow도 typed refusal다. 아직 commit되지 않은 준비 결과를 accepted trajectory의 인증 output으로 공개하는 경로는 없다. 성공하여 retained가 된 과거 window의 query는 live generation과 달라도 그 자신의 immutable seed로 계속 유효하다.

AT07/AT09의 실행 순서는 manifest에 고정했다. epoch0에서 P0와 Q0를 준비하고 P0만 commit한다. 첫 window 내부 global `1/32`의 전체 출력 bits를 저장한다. Q0의 stale commit을 거절하고 epoch1에서 P1을 새로 준비·commit한다. 두 commit 뒤 retained window0에서 같은 `1/32`, 시작·끝·requested point를 다시 query하여 저장한 identity/center/radius bits와 원래 초기값 기준 reference를 검사한다. P0의 commit 재사용도 거절한다. 각 owner의 join 출력은 같은 true flow를 독립 포함해야 하며 서로 다른 owner의 중심 bits가 같다는 조건은 추가하지 않는다.

## 8. 동결 acceptance matrix

모든 항목은 구현 전 기준이며 이 계약 작성에서 실행하지 않았다. 독립 검토자는 새 수식 및 source 경계를 채택/수정/HOLD로 판정한다. 채택 후 source/preregistration checkpoint를 게시하고, 그 EXEC_SHA의 입력 해시를 고정한 뒤 영향 범위만 실행한다. 첫 실패 raw log를 덮어쓰지 않는다.

### C1: 입력·config·tolerance 출처 동결

정확한 machine 입력은 `evidence/continuation_a02_research/supplement_02/FROZEN_ACCEPTANCE_INPUTS_V2.json`이다. 모든 model/config/grid/radius binary64 scalar는 hex `bits`가 권위이며 decimal `display`는 설명이다. 모든 resolved config 필드(order/degree/sweeps/table 차수, initial/min/max h, max_steps, output policy)를 기록했으므로 미래 default 변경에 의존하지 않는다. `AT01`–`AT12` 각각은 fixture/config/oracle/변이·실행 순서와 limits를 이 manifest에 연결한다. 입력 hash와 code source binding은 같은 폴더의 `SOURCE_BINDINGS.json`, 순수 실행 전 계약은 `PREREGISTRATION.md`다. 구현 source checkpoint 뒤에는 이 입력들을 `EXEC_SHA`에 결속하여 먼저 고정하고 실행한다.

물리 tolerance는 기존 fixed-grid `mean=mode=1e-5` (`packet_trajectory_contracts.rs:216–224`, 설정 222행), inherited uncertainty `mean=mode=1e-4` (302–321행, 설정 314행), actual rejection `mean=mode=2e-4` (379–389행, 설정 382행)를 그대로 보존한다. 기본 order=8, degree=10, sweeps=4, min_h=1e-6, max_h=2·initial_h, max_steps=1000의 출처는 `packet_trajectory.rs:99–112`다. Rejection의 기존 order=2, degree=3, min_h=1/64를 그대로 쓴다. 해석적 inclusion 검사에는 물리 tolerance 이외의 slack을 더하지 않는다.

AT02는 기존 A01 unequal-frequency **local** fixture의 ξ,w,z,width,carrier,κ, 네 χ/uncertainty arms, 전체 horizon=1/32, 다섯 요청 시각과 Fraction reference를 재사용한다 (`production_audit_packet_local_certificate.rs:50–75,96–105`). 그 local fixture에는 `mean_tolerance`/`mode_tolerance`가 **없다**. 새 multi-window trajectory gate는 별도 fixed-grid fixture의 변경 없는 `1e-5/1e-5`를 이 계약에서 명시적으로 선택하고 독립 검토에 회부한다. 따라서 “local oracle의 기존 물리 tolerance를 재사용했다”는 주장은 하지 않는다. 새 schedule은 `initial_h=max_h=1/64`, order=4, degree=4, sweeps=2, min_h=1e-6, max_steps=1000으로 동결한다. Degree40/48은 독립 reference의 차수이며 solver degree=4와 다르다. 원래 초기값에서 한 번 만든 reference를 쓰고 committed 중심에서 reference를 재시작하지 않는다.

AT05의 모든 유한 binary64 시각 자체는 dyadic rational이다. 이 검사는 “비dyadic binary64”를 사용하지 않는다. `t_k=bits(0.1)`, `t_request=bits(0.9)`의 **정확한 실수 차**가 단일 binary64에 표현되지 않는 경우를 쓴다. 정확한 차는 `28823037615171175 / 36028797018963968`이며, rounded `u=bits(0.8)`와의 차는 `-1/36028797018963968`다. 따라서 subtraction enclosure가 non-singleton이고 Δ>0인 경로를 확인한다. 첫 physical window 끝은 `bits(1.1)`이고 H는 그 exact span 전체를 감싸도록 따로 저장한다. 잘못된 offset binding은 exact identity 음성 대조군으로 검출한다. 기존 반경이 one-ulp 시간 오차까지 우연히 감쌀 수 있으므로 음성 대조군에서 근거 없이 반드시 numerical disk inclusion이 실패한다고 가정하지 않는다.

| ID | 동결 fixture/config 및 방법 | 완료 기준 |
|---|---|---|
| AT01 | `F_ONE_LINEAR`, `F_ONE_CUBIC` / `C_FIXED`; local one-mode model 입력과 기존 fixed-grid global config를 명시적으로 연결; 원래 초기값의 exact Fraction analytic flow | 최소 두 accepted windows, 각 window start/interior/end의 실제 반환 mode/mean disk inclusion과 `1e-5/1e-5` gate |
| AT02 | `F_UNEQUAL_0/P/M/U` / `C_UNEQUAL`; 4 arms×5 원래 global times; solver order4/degree4/sweeps2, h≤1/64 | 원래 초기값부터 degree40/48 reference와 80 disk checks, ε40≤2^-120, ε48≤2^-160, ε48≤ε40; slack-free inclusion과 두 음성 대조군; 새 trajectory gate는 별도 출처 `1e-5/1e-5` |
| AT03 | `F_INHERITED` / `C_INHERITED`, `F_UNEQUAL_U` / `C_UNEQUAL`; 각각 기존 `[1e-6,2e-6]`, `[2^-30;3]` | 최소 두 windows, 초기 product uncertainty 전시간 수송, E_global 고정, returned-center e_next 포함; 각 fixture의 동결 tolerance |
| AT04 | `F_STATIONARY_SMALL_W` / `C_INHERITED`; χ=ω=0, w=`[2^-40,1/2,1/2]`, heterogeneous radii manifest 고정 | sqrt(N) 인공 재확장 없음, source-tallied outward Duhamel inherited branch와 일치, `1e-4/1e-4` 유지 |
| AT05 | `F_OFFSET` / `C_OFFSET`; stored binary64 origin0.1/request0.9/end1.1; 정확한 차의 non-singleton enclosure | Δ>0, 원래 global label의 exact analytic reference 포함, 잘못된 rounded-offset identity 대조군 검출; `1e-5/1e-5` 유지 |
| AT06 | `F_REJECTION` / `C_REJECTION` actual rejection; `F_FIXED` / `C_FIXED`의 private first/second-query fault seams와 setup refusal | h=1/4 actual tolerance rejection 후 ≥2 commits; snapshot/cursor/history bit identity; trial output 불노출; work 정확; direct accepted-size replay 일치. Fault seam은 transaction 검증이며 arithmetic 포함 주장에 쓰지 않음 |
| AT07 | `F_FIXED` / `C_FIXED`, MeanOnly/AllModes; 첫 window를 두 commit 뒤 다시 query | 첫 seed 불변, `1/32` 재query의 identity/center/radius bits 재현과 global reference 포함; start/end/requested/interior 및 join 검증 |
| AT08 | Manifest의 고정 NaN/Inf/bad radii/shape/checked-size/zero/subnormal/overflow/damping 변이 | 지원 입력의 exact inclusion 또는 typed refusal를 구분 보고; accepted state 보존, underflow exact-zero 가정 금지, work 누적 |
| AT09 | `F_FIXED` / `C_FIXED`; private 인증 필드 compile-fail, candidate 사후변이, P0/Q0→P0 commit→Q0 stale refusal→P1 commit→window0 query | 위조·재결속 차단, stale prepared snapshot과 이미 사용한 P0 commit typed refusal; retained 첫 window query는 두 commits 뒤에도 유효 |
| AT10 | `F_FIXED` / `C_FIXED`; 같은 allocator의 세 Vec clone calibration 뒤 각 1회 warmed query/advance 계측 | clone ≥3 allocation 검출 후에만 warmed 구간 allocation=0 인정; 실패 calibration은 `ALLOCATION_UNAVAILABLE`; setup/retention allocation 별도 기록 |
| AT11 | `F_FIXED`, `F_REJECTION`의 manifest 고정 operation sequence; 실행 전 source tally | 성공/첫·중간 fault/refusal/stale/rejection/commit/historical query exact integer work delta; prior work 누락·reset 없음 |
| AT12 | 동결 manifest/preregistration/source/prior review, command/environment/output/exit identities | 원본 첫 실패 보존, 기존 L-0124 ledger mismatch와 신규 수치 실패 구분, scoped decision은 실제 실행·독립 검토 이후 |

미래 bounded 실행은 targeted test binary 최대 1800초, unequal Fraction oracle 600초, analytic Fraction oracle 120초, 각 fixture max_steps=1000으로 제한한다. 이 보완에서는 solver/test/build를 실행하지 않았다. Source·manifest·문서 readback은 연구 입력 준비 근거이며 수치 완료 근거로 사용하지 않는다.

전체 A02의 최종 완료는 full trajectory inclusion, 고정 요청시각 및 전체 시간 query, 비영 uncertainty, unchanged tolerance, rejection atomicity/work, 실제 산술 범위, 독립 review가 모두 닫힌 뒤에만 판정한다. κ>0/A03, Leja/A04, generic dense/A05, speed/A07 및 전체 production은 이 계약의 판정 범위 밖이며 기존 의무를 유지한다.


## 현재 구현 실행 freeze

- 현재 권위 입력은 supplement_02 V2 manifest와 overlay이며, v1 입력 및 입력별 이전 refusal arm은 수정하지 않는다.
- A02-TXN-01만 독립적으로 승인됐다. 구현은 `packet_trajectory.rs`와 계약이 명시한 보조 파일에 한정한다. 다른 A03/A05 dirty source는 이 단위의 변경 또는 evidence로 포함하지 않는다.
- 구현 코드와 실행 전 preregistration을 fast-forward 게시해 `EXEC_SHA`를 정한 뒤에만 AT01–AT12를 실행한다. 원래 fixture, absolute tolerances, oracle order, acceptance limits를 바꾸지 않는다.
- 구현/실행 뒤 반환된 반경과 모든 observation inclusion, 초기 uncertainty transport, stale/rejection transaction, cumulative work, allocator positive control을 별도 검토한다. A02 full trajectory promotion은 그 모든 gate와 독립 검토 후에만 판단한다.
