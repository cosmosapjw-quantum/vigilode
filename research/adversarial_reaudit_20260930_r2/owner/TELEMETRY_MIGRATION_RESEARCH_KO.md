# 정규화 수정 이후 telemetry와 frozen policy의 이전

상태: 직접 유도·Wolfram exact 검산·독립 Python toy 실행. 최신 소스의 실제 ζ34 policy를 재보정하거나 새로운 holdout을 읽지 않았다. 이 절의 구현은 연구용이며 production 변경이 아니다.

## 현재 소스가 이미 밝힌 문제

`research/generic_timing_replication_continuation_transaction_v37/reports/ADDENDUM_20260930_PHI_NORMALIZATION.md`는 φ augmentation 정규화로 Arnoldi residual history와 ζ34가 달라졌고, frozen τ=13.397의 구분 능력이 유지되는지 모른다고 명시한다. `V37_TRAJECTORY_SNAPSHOT_V4_20260930.json`은 branch와 변경 원인을 기록하며 consumed N=192 replay의 변경을 별도 snapshot으로 보존한다. 원 sealed 자료를 덮어쓰지 않은 것은 적절하다. 이 문단은 현재 문서에 대한 source-confirmed 판단이며 N=192의 새 독립 실행 결과는 아니다.

**권고:** 정확도 수정을 되돌릴 이유는 없다. 다만 새로운 operator representation의 telemetry에 옛 calibration의 권한을 자동으로 옮기지 않아야 한다. 현재 shadow/research 제한은 유지하고, active-switching 승격 전에 feature provenance와 calibration epoch의 일치를 필수 조건으로 삼는다.

## 물리적 오차와 내부 norm의 차이

무차원 계산 시간 u∈[0,1], 상수 augmented operator M에 대해 x′=Mx, x(0)=q, 물리 projection P를 둔다. x_m(0)=q인 근사와 residual r=x_m′−Mx_m를 정의하면

$$e'=Me-r,\qquad e(0)=0,\qquad
Pe(1)=-\int_0^1P e^{(1-u)M}r(u)\,du.$$

가역 S에 대해 M̂=S⁻¹MS, q̂=S⁻¹q, x̂_m=S⁻¹x_m, P̂=PS를 함께 변환하면 r̂=S⁻¹r이고

$$\widehat P e^{(1-u)\widehat M}\widehat r(u)
=P S S^{-1}e^{(1-u)M}S S^{-1}r(u)
=P e^{(1-u)M}r(u).$$

따라서 정확 산술의 projected defect integral은 불변이다. 반면 ‖S⁻¹r‖₂는 일반적으로 ‖r‖₂와 다르다. 비직교 similarity 아래 Arnoldi의 Euclidean orthogonal projection 자체도 바뀌므로 실제 두 Krylov iterate가 x̂_m=S⁻¹x_m 관계를 갖는다는 주장은 하지 않는다. 이 계산은 비교 대상을 정의하는 원리다.

## 직접 실행한 판별 toy

$$M=\begin{pmatrix}-1&1&0&0\\0&0&1&0\\0&0&0&1\\0&0&0&0\end{pmatrix},
\quad q=e_4,\quad P=(1,0,0,0),\quad x_1(u)=q+uMq.$$

Sα=diag(1,α²,α,1), α>0이면 r(u)=−uM²q이며

$$\|\widehat r(1)\|_2=\alpha^{-2},\qquad
Pe(1)=\varphi_3(-1)=\tfrac12-e^{-1}\simeq0.13212056.$$

Wolfram exact 출력은 error-integral difference=0, similarity integrand difference=0을 반환했다. α=4에서 residual norm은 1→1/16이지만 물리적 오차는 같다. Python은 α=1/4,1,4,16,64의 5개 값에서 expm 결과와 projected integral을 각각 analytic reference와 10⁻¹³ 이내로 확인했다. 예시 norm gate 0.5의 통과 여부는 달라지지만 physical error gate 0.1은 전부 실패한다. **이 예시 gate는 실제 ζ34가 아니므로 그 policy의 false accept 반례로 세지 않는다.**

최초 p=2 Wolfram fixture는 residual이 physical component에만 남아 norm drift를 보여주지 못했다. 해당 출력도 보존했고, 비영 auxiliary residual이 있는 p=3 fixture로 목적을 분리했다. Wolfram context 검색은 관련 항등식에 결과를 주지 않아 근거로 채택하지 않았다.

## 계산 가능한 구조적 경로

M̂=[[hA,B],[0,J]], Jᵖ=0이면 block exponential의 upper-right는

$$[e^{v\widehat M}]_{12}
=\int_0^v e^{(v-s)hA}B e^{sJ}\,ds
=\sum_{j=0}^{p-1}v^{j+1}\varphi_{j+1}(vhA)B J^j.$$

따라서 projected residual propagation은 physical action과 유한 nilpotent chain으로 분리할 수 있다. 이 항등식은 직접 적분 전개에서 도출되며, 전체 augmented matrix의 Euclidean logarithmic norm 하나를 쓰는 것보다 구조적 bound를 설계할 여지가 있다. 이 식을 이용한 유효 enclosure·roundoff·quadrature 총오차 상계 구현은 미완료다. 새 인증의 완료로 읽으면 안 된다.

## 구체적인 다음 개발 계약

1. `FeatureProvenance`에 `operator_representation`, `physical_projection`, `state_scale`, `auxiliary_scale_policy`, `residual_definition`, `sampling_grid`, `arithmetic`, source SHA를 기록한다. 내용의 hash는 identity일 뿐 calibration validity의 증명은 아니다.
2. policy artifact에 동일 feature version과 calibration corpus identity를 넣고 불일치는 `NotEvaluated`로 반환한다. sealed 과거 결과는 유지한다.
3. 기존 consumed corpus에서는 정확히 어느 feature와 recommendation이 바뀌는지만 paired 재생한다. 여기에 threshold를 맞춘 결과를 새 holdout 결과로 표시하지 않는다.
4. 물리 projection을 포함한 defect diagnostic 후보를 원 residual feature와 같은 input·오차 metric으로 비교한다. 총 bound 없이 diagnostic을 certificate로 바꾸지 않는다.
5. 새 policy를 고정한 뒤 독립 family/session holdout에서 false acceptance와 full attempted cost를 검증한다. 공개 benchmark gate와 동일한 통계 단위·소비자 경로를 사용한다.

필수 acceptance: representation mismatch 거절, 기존 sealed snapshot 불변, 물리 projection identity 대조, invalid scale typed failure, 학습/검증 데이터 분리, 모든 prefix·continuation·fallback 비용 포함. 새 threshold의 값이나 예상 speedup은 이번 보고서가 정하지 않는다.
