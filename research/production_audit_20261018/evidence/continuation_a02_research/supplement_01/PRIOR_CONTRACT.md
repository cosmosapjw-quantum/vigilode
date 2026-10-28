# A02-TXN-01: 검증된 κ=0 궤적의 동결 연구 계약

판정: **READY_FOR_REVIEW**. 이 문서는 구현 전 독립 증명 검토를 위한 계약이다. 신규 수식의 독립 채택, A02 구현, 표적 검증, 과학적 승격은 아직 수행하지 않았다. 원래 A02 전체 궤적 목표를 보존하며, 아래 첫 구현 단위는 그 목표의 핵심 수송·관측·거절 의무를 구현한다. 전체 DAG와 기존 HOLD를 이 문서만으로 변경하지 않는다.

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

## 8. 동결 acceptance matrix

모든 항목은 구현 전 기준이며 이 계약 작성에서 실행하지 않았다. 독립 검토자는 새 수식 및 source 경계를 채택/수정/HOLD로 판정한다. 채택 후 source/preregistration checkpoint를 게시하고, 그 EXEC_SHA의 입력 해시를 고정한 뒤 영향 범위만 실행한다. 첫 실패 raw log를 덮어쓰지 않는다.

| ID | 입력/방법 | 완료 기준 |
|---|---|---|
| AT01 | 독립 exact one-mode linear 및 cubic 해; 최소 두 accepted windows; 각 window 중간/endpoint | 동일 실제 반환 mode/mean 중심의 outward disk inclusion; 원래 tolerance 전 관측점 통과 |
| AT02 | 기존 unequal-frequency Fraction fixture의 ξ,w,z,width,carrier,κ와 4 χ/uncertainty arms; 전체 horizon `1/32`, 최소 두 windows, 기존 다섯 global times | 원래 초기값부터의 독립 degree40/48 reference와 80 disk checks를 유지; ε40≤2^-120, ε48≤2^-160, ε48≤ε40; slack-free inclusion; center/radius 음성 대조군 거절 |
| AT03 | 비영 initial product disks, 최소 두 windows, weighted/marginal bounds 둘 다 관측 | 초기 불확실성의 전시간 수송, global E 보존, returned-center e_next의 inclusion |
| AT04 | χ=ω=0 및 N>1, 작은 양의 weight와 heterogeneous initial radii | 수송 수식에 window당 sqrt(N) 인공 증폭 없음; χ=0 component inherited bound는 원래 r_j 보존; tolerance를 키우지 않음 |
| AT05 | 비dyadic global origin/요청시각, 실제로 non-singleton offset interval인 fixture | Δ>0 경로 실행; 원래 binary64 global label의 exact-reference 포함; local u를 원래 global time으로 잘못 취급하는 음성 대조군 검출 |
| AT06 | 큰 window의 실제 tolerance rejection 후 작은 accepted trial; 첫/중간 evaluation refusal; setup/constructor refusal | accepted 전체 snapshot와 cursor bit identity; trial output 불노출; 누적 work 정확; 이후 결과는 동일 accepted state에서 시작한 직접 경로와 일치 |
| AT07 | 초기점, requested grid point, window join, 임의 내부 query; MeanOnly/AllModes | owner/time/center/radius identity 일치; retained continuous residual로 각 query 포함; 모든 요청점 tolerance gate |
| AT08 | NaN/Inf, negative/bad radii, malformed shape, huge checked sizes, zero/short horizon, subnormal time/state/radius, overflow | 지원 산술에서 포함 또는 typed refusal; accepted state 보존; underflow exact-zero 가정 없음; 실패 비용 남김 |
| AT09 | public fields/view/state 위조·수정 compile-fail와 후보 사후변이 | 임의 인증 상태 생성/재결속 차단; frozen owner outputs는 외부 후보변이와 독립 |
| AT10 | 같은 allocator의 세 Vec clone 양성 대조군; 그 후 warmed successful query/advance | 양성 대조군 ≥3 allocations; 측정 구간의 warmed allocations=0; setup/retention allocation 별도 실측·기록 |
| AT11 | 산술·identity·copy·setup·query/rejection counter source tally | 성공/첫 실패/중간 실패의 exact delta 일치; prior work 누락·reset 없음 |
| AT12 | 원시 command/environment/inputs/outputs/exit/source identities와 독립 review | 원본 실패 보존; 기존 ledger hash mismatch를 신규 수치 실패와 구분; A02 scoped scientific decision은 실제 검토 이후 |

전체 A02의 최종 완료는 full trajectory inclusion, 고정 요청시각 및 전체 시간 query, 비영 uncertainty, unchanged tolerance, rejection atomicity/work, 실제 산술 범위, 독립 review가 모두 닫힌 뒤에만 판정한다. κ>0/A03, Leja/A04, generic dense/A05, speed/A07 및 전체 production은 이 계약의 판정 범위 밖이며 기존 의무를 유지한다.
