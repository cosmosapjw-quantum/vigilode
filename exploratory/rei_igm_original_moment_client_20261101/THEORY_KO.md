# VigilODE REI IGM | 원본 46,080-cell photon source의 정확한 3-moment reduction

**소유자:** `vigilode`는 수치 방법/solver client, `rei_bianchi`는 원래 26-event H/He/OTS 물리 원본 소유자. 이 계산은 원본 `PR332`가 실제 REI 46,080개 공간 노드, 4 photon-group, 3 중성종 총수에서 계산·검증한 보존적 광자 흡수 source의 **하나의 source-frozen 부분계**이다. 원래 26-event REI 화학/열/광자 결합 IVP를 다시 풀거나 수정하지 않는다.

## 원본 소스와 가정

- REI PR #332 exact HEAD `2e3305c8ba9bff5d00412c5d79d9c1aa1e6f6e9a`.
- PR #332 실제 Actions ZIP SHA256 `83d94b5383ac7c1f36d89486e83bf8b4a56dd1993a25af3ae1f8fed06a695ba9`, 내부 원본 H/He source 상태 SHA256 `cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01`; 원래 2026-08-29 26-event 이력 tar SHA256 `a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b`.
- 시작은 원본 첫 canonical interval의 accepted tick 160. `h=4.87184874916828e9 s`는 원래 proper-time interval의 한 tick 길이이다. 입력 중성종 원자 총수 `S0=(N_HI,N_HeI,N_HeII)`는 실제 원본 46,080개 셀의 총합이며 단위는 particles. Source current `J_g` 단위 photons/s.
- 네 개 photon current와 원래 횡단 광자 sink 비율·species 흡수 비율을 tick160에서 **고정**한다. 셀별 photoionization은 유지하지만 재결합, 충돌 이온화, local OTS heat, Hummer-Seaton/SDIRK 열근, 원래 네 단계 photon forcing 재평가, 시간적 배경, Bianchi shear, 계량 반응은 모두 이 특수 부분계에서 제외한다.

## 셀 수준 원본에서 정확한 reduction

원본 source의 단일 광자군 흡수는 공간 셀 i, 중성종 s에 대해 다음의 공통 분모를 공유한다.

\[
F_{isg}(t)=J_g\frac{C_{sg}N_{is}(t)}{E_g+\sum_{a=\mathrm{HI,HeI,HeII}}C_{ag}S_a(t)},
\quad S_s(t)=\sum_{i=1}^{46080}N_{is}(t).
\]

이때 `C`는 REI owner의 종별 서로 다른 normalization과 `YHE`를 이미 포함한 계수다. `E_g`는 원본 외부 광자 sink다. `J_g,C,E`를 그대로 고정하는 특수 부분계에서 합산하면 다음 세 개 전역 모멘트가 **닫힌 자율식**을 만족한다.

\[
R_s(S)=\sum_g J_gp_{sg}(S),\qquad
\dot S_{\rm HI}=-R_{\rm HI},\quad
\dot S_{\rm HeI}=-R_{\rm HeI},\quad
\dot S_{\rm HeII}=R_{\rm HeI}-R_{\rm HeII}.
\]

여기서 초기 그룹 흡수 비율 `p0_sg`와 외부 비율 `e0_g`를 PR332에서 원본 값 그대로 추출하면 계수의 공통 분모 전체를 재생성할 필요 없이

\[
p_{sg}(w)=\frac{p^0_{sg} w_s}{e^0_g+\sum_a p^0_{ag}w_a},
\quad w_s=S_s/S_s^0,\qquad p_{\rm ext,g}(w)=\frac{e^0_g}{e^0_g+\sum_a p^0_{ag}w_a}
\]

가 정확하게 성립한다. 이 등식은 단일 원본 고정 source에 대한 동일한 rational photon competition의 계수 불변 축약이다. 종/외부 흡수 합은 각 그룹에서 1이다.

무차원 시간 `u=(t-t160)/h`에서 VigilODE native client는 `y=(w_HI,w_HeI,w_HeII,a_HI,a_HeI,a_HeII)`의 여섯 성분을 적분한다. `a_s=N_{\gamma,\mathrm{abs},s}/S_s^0`는 누적 광흡수 수를 원래 입자 수로 정규화한 값으로 양수이며, `da_s/du=hR_s/S_s^0`. 따라서

\[
w_{\rm HI}+a_{\rm HI}=1,\quad w_{\rm HeI}+a_{\rm HeI}=1,\quad
w_{\rm HeII}-1-\frac{S^0_{\rm HeI}}{S^0_{\rm HeII}}a_{\rm HeI}+a_{\rm HeII}=0.
\]

원본 group-3 current `J3=5e-324 photons/s`는 f64 최하위 subnormal로 **물리적으로 의미 있는 분광 플럭스의 측정값이 아니라 수치 표현 하한**이다. 이 구간에서 HeII 직접 흡수 기여는 machine-float로 0이므로, 중요한 HeII 상태 변화는 HeI→HeII로부터 나온다. 이 하한을 관측적인 0이라고 해석하지 않는다.

## VigilODE의 실제 기여: analytic matrix-free JVP

임의의 `v_s=δw_s`에 대해 한 그룹에서 `P_{sg}=p^0_{sg}w_s/W_g`를 계산하면

\[
δP_{sg}=P_{sg}\left(\frac{v_s}{w_s}-\sum_aP_{ag}\frac{v_a}{w_a}\right).
\]

이것은 sparse local term과 세 전역 species moment의 shared denominator feedback을 결합하는 정확한 Jacobian-vector product다. `Jv`는 original frozen `J_g`를 그대로 사용하며 assembled 46,080×46,080 Jacobian이 없다. VigilODE의 기존 matrix-free **RODAS5P** `OdeProblem` JVP callback에 바로 연결한다. 두 개의 누적량을 편미분할 때도 같은 원본 photon current를 사용한다.

## 결과 범위

- 참조: SciPy Radau, 별도 SciPy DOP853 동일 original frozen input. native VigilODE RODAS5P 결과는 별도 실행 증거가 있을 때만 인정.
- 실제 공간 분포: HI, HeI 광자 흡수는 각 종 내에서 동일 전역 hazard를 가지므로 `A_{is}(u)=[R_{is}(0)/R_s(0)] S_s^0 a_s(u)`다. PR332가 출력한 46,080 원본 `R_is(0)`의 총합과 매칭해 각 셀에서 실제 photon/photoelectron source-frozen 차분을 생성한다. HeII original source는 subnormal이므로 해당 셀 직접 흡수는 수치 구별불가.
- 원본 전체 26-event 계와 달리 재결합·열/광자/중력 이력이 없음. 이 연구를 source-owned **subsystem client**라고 부르며 원래 `F_h-G_2\circ G_1`의 전체 집합 certified defect나 canonical REI interval 완료로 주장하지 않는다.

문헌상 관련 검증 방법: Neher–Jackson–Nedialkov (2007), DOI 10.1137/050638448, validated ODE uncertainty wrapping; Kopecz–Meister (2018), DOI 10.1007/s10543-018-0705-1, modified Patankar positivity/conservation; Pawlik–Schaye (2011), DOI 10.1111/j.1365-2966.2010.18032.x, H/He radiative-transfer chemistry context. 이 논문들은 여기서 실행한 source-frozen VigilODE client 자체를 검증한 출처가 아니다.
