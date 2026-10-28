# 정보기하 기반 구면 알고리즘의 독립 검토

2026-10-23. 검토 대상은 이번 연구의 다섯 소규모 prototype과 그 증명이다. 첨부 논문 전체의 신규성 심사나 VigilODE의 범용 production 인증은 이 검토의 범위가 아니다. 본 검토자는 구현 담당자와 별도로 소스, 실제 결과, 논증을 대조하고, 다른 표현을 사용하는 작은 독립 계산을 수행했다. 파일 identity는 `results/independent_review.json`에 있다.

## 판정

**명시한 유한 격자·고정 부분문제 범위에서 채택 가능하다.** 발견한 비용·적용범위 지적은 아래처럼 수정되었다. 새 Rust solver에 이미 이식되었다거나, cell 수 감소와 조건수 개선을 전체 PDE 실행 시간의 개선으로 해석할 근거는 없다. 다음 실제 개발에는 현재 원자료와 식을 재사용하고, 결합 적분·적응 transfer·target error criterion만 추가로 검증하면 된다.

## 1. 보존형 정보 손실과 구면 격자

셀 평균으로 에너지를 유지하면 Bregman 선형 항이 사라지므로 Jensen gap이 정확한 remap 정보 손실이다. 중첩 partition의 telescope도 직접 성립한다. $F''(\rho)\ge m_C$와 셀 평균 보존을 적용한 관측량 경계

\[
|\Delta O_K|\le\sum_C\sqrt{2\eta_C V_{K,C}/m_C}
\]

를 독립적으로 확인했다. $\rho=0$인 fine point는 하한을 이용한 유한 경계와 양립하지만, $F''$의 상한을 쓰는 명제는 양의 lower bound가 필요하다. 코드의 모든 인증 대상은 제공된 fine values이며, 미해상도 연속 구조나 시간 적분 오차를 포함하지 않는다.

65자리 `decimal`로 $r=\pm10^{-8},\pm10^{-12}$의 scalar Bregman remainder를 독립 평가했다. 저장된 double 구현과의 상대 차이는 $6.57\times10^{-17}$ 이하였고, 별도의 무작위 장에서 중첩 entropy identity의 잔차는 표시 정밀도에서 0이었다. 독립 무작위 kernel 오차도 계산한 bound 이내였다. 다만 그 사례에서 실제 오차 2.76에 비해 상한은 약 1448이었다. 정보 bound의 유효성과 sharpness는 별도 성질이다.

코드의 greedy 우선순위는 제거되는 entropy/추가 cell 수이다. 관측량 bound는 사후 계산한다. 현재 구현을 특정 관측량 허용오차까지 자동으로 맞추는 완성된 goal-oriented controller로 부르지 않는다. 균일 baseline은 같은 허용 entropy budget을 만족하는 첫 중첩 level이므로, 두 방법의 실제 오차가 정확히 같지 않다는 문구를 유지한다.

에너지-only thermal 평균은 입자수를 증가시킬 수 있다. 코드가 이 drift를 보존하고 joint $(n,\rho)$ MB 예제로 수정을 확인한 점은 적절하다. 실제 photon-number 보존 client는 energy-bin별 occupation 또는 joint moments를 옮겨야 한다.

**수정된 비용 지적:** prefix sum만 보면 합 조회는 $O(1)$이지만, 구현은 각 rectangle의 min/max와 안정화 fallback에서 값을 훑는다. 현재 진단 비용은 균형 계층에서 $O(N_\mathrm{fine}\log N_\mathrm{fine})$, heap 비용은 별도로 $O(N_\mathrm{active}\log N_\mathrm{active})$다. 구현 설명에 반영되었다. 이를 매 RHS에서 실행하면 압축으로 절감한 비용을 상쇄할 수 있다.

## 2. Fisher 전처리

양의 동일 구적과 동일 score를 사용하면 각 rank-one 항에 susceptibility ratio를 적용하여 $aH_0\preceq H\preceq bH_0$를 얻는다. 등방 기준의 angular orthogonality와 중심화된 radial variance는 2×2 block whitening을 정당화한다. MB/FD의 log-susceptibility 미분 절댓값은 1 이하이고, regular BE margin 아래에서는 $\coth(\gamma/2)$가 상한이다.

코드의 실제 anisotropic displacement가 retained natural-score family에 들어가는지 least-squares residual도 검사한다. CG와 PCG는 같은 원래 좌표의 $\|Hx-b\|_2/\|b\|_2\le10^{-9}$를 사용한다. 이 검사는 transformed residual을 작게 만든 것과 다르며, 선형해 오차 자체는 별도 저장값으로 확인한다.

독립 계산에서는 block inverse 전체 행렬을 조립하고 그 대칭 양의 제곱근으로 $P^{1/2}HP^{1/2}$를 계산했다. 중심 score로 직접 만든 whitened Gram spectrum과의 최대 차이는 $1.10\times10^{-13}$ 이하였다. 독립 고유값 구간 $[0.84245,1.26682]$은 실제 susceptibility 구간 $[0.75651,1.44806]$ 내부였다.

**수정된 비용 지적:** rank별 radial 통계뿐 아니라 mode별 block 배열도 생성하므로 setup은 $O(N_E(L+1)+(L+1)^2)$다. 최종 JSON에 반영되었다. 현재 측정은 dense Hessian을 사용한 작은 진단이며, 문서의 matrix-free $O(Nd)$ 이론 비용과 구분된다. setup을 포함한 PCG timing이 작은 문제에서 더 느린 사례도 최종 자료에 남아 있다.

## 3. 충돌 부분흐름

기존 VigilODE의 scalar Thomson 우변은 이미 rank 6이다. dense matvec 대비 새 Python rank-6 matvec의 timing을 기존 VigilODE의 개선율로 쓰면 안 된다. 이번 유효한 추가 구성은 기존 유한체적 factor $U,V$에 대한 정확한 시간 action이다.

멱급수 또는 block exponential을 이용하면 singular $B=V^TU$에도

\[
e^{\tau(UV^T-I)}y=e^{-\tau}y+Ue^{-\tau}\tau\varphi_1(\tau B)V^Ty
\]

가 성립한다. 이식을 위해 continuum quadrupole 고유값 0.1을 임의의 cell-average grid에 대입해서는 안 된다는 지적이 실제 기존 oracle 대조로 확인되었다. cached small-matrix 경로의 step 비용과, 매 호출 $B$ 및 작은 exponential을 계산하는 현 Python reference의 비용은 구별해야 한다.

**수정된 Euler 비교:** 유한 격자에서는 gain 행렬의 양의 대각 성분 때문에 $\kappa h>1$에서도 Euler가 양성을 보존할 수 있다. 따라서 $J\ge\lceil\tau\rceil$는 표준 충분조건 $\kappa h\le1$을 사용하는 Euler와의 비교로 한정했다. exact collision action의 유효성과 보존 정리는 이 교정의 영향을 받지 않는다.

편광 확장에서 $\langle PTP\rangle=(7T+\operatorname{tr}T\,I)/15$를 독립적으로 전개했다. 따라서 gain 고유값은 $1,0.7,0$, 감쇠율은 $0,0.3,1$이며 scalar 감쇠율 0.9와 구분해야 한다. 양의 표현은 PSD를 보존한다. $a_0-a_2\ge0$는 $x^{0.3}\le0.7+0.3x$에서 따른다. 전개된 small-$\tau$ 계수의 2차항은 $0.15\tau^2$로 일치한다.

이 exact arithmetic 보존 정리와 부동소수점 eigenvalue 검사 허용치는 구별되어 있다. real symmetric transverse coherency는 $I,Q,U$를 포함하고 $V=0$, cold electron rest frame의 한 고정 shell을 가정한다. Bianchi streaming, tilt, energy redistribution, moving screen과의 결합은 별도 구현 의무다.

## 4. 반복 remap의 누적 정보 예산

질량좌표에서 column-stochastic $T_j$는 Hellinger 거리 $H(p,q)=\|\sqrt p-\sqrt q\|_2$를 축소한다. pointwise $D_{\rm Poisson}\ge H^2$와 삼각부등식으로 실제 계산 경로의 local remap loss $\eta_j$에 대해

\[
H(p_J,q_J)\le\sum_j\sqrt{\eta_j},\qquad
|K^T(p_J-q_J)|\le\operatorname{range}(K)\sqrt M\sum_j\sqrt{\eta_j}
\]

를 얻는다. 유한 양의 동일 질량 상태라는 전제를 포함하면 증명이 닫힌다. 직접 scalar 검사에서도 zero component와 서로 다른 질량을 포함한 pointwise inequality가 유지되었다. 질량이 같다는 가정은 마지막 관측량의 kernel centering에서 사용한다.

실험 8개 중 6개는 10회 또는 40회의 비영 remap loss를 실제로 누적한다. 나머지 2개는 첫 remap 이후 partition/rotation이 맞아 손실이 0인 대조군이다. 320개 step 검사는 단일 remap의 반복 복제만은 아니다. 누적 bound가 자명한 최대 거리보다 커지는 경우에도 부등식은 유효하며, 실제 설계에는 tighter contraction factor 또는 budget 재분배가 유용하다.

이 결과를 $-\rho^{3/4}$ Bregman이나 일반 BE/FD entropy의 전역 삼각부등식으로 바꾸어 쓰면 안 된다. 검증한 적분 지도는 positive Markov map이며, 일반 고차 DOPRI/RODAS가 같은 성질을 갖는다고 가정하지 않는다.

## 5. Laguerre와 shell별 cutoff

추가된 spectral_tools_demo.py의 generalized Laguerre 정규화는 $x^2e^{-x}dx$에 대해 $\int (L_n^{(2)})^2x^2e^{-x}dx=(n+2)!/n!$를 사용하므로 맞다. 첫 두 기저 $1/\sqrt2$, $(x-3)/\sqrt6$도 실제 코드와 일치한다. 16점 generalized Gauss–Laguerre rule은 8개 기저의 모든 곱의 차수 14를 적분하기에 충분하다.

$\mathcal P_v$의 harmonic multiplier $v^\ell$와 Parseval을 적용한 tail 경계, mean-normalized addition theorem에 의한 $L^\infty$ 경계, 알려진 유한 계수로부터 얻는 positivity margin을 검토했다. $v=0$의 monopole와 $v=1$의 전체 알려진 rank 보유를 별도 처리한다. 이 구현이 사용하는 계수 tail은 생성된 $L_{\max}=24$ field 전체에서 알려져 있으며, 미지의 continuum tail을 관측 모멘트로 추정했다고 주장하지 않는다. 실제 최종 JSON의 평가 항 수는 2013/3750, Gram 최대 오차는 $2.45\times10^{-15}$ 미만이며 원본 코드 해시와 일치한다. 이는 basis 평가 항 수와 orthogonality 검사이고 PDE 실행 시간 검사는 아니다.

주 보고서 RESEARCH_REPORT_KO.md의 수치 요약도 최종 JSON과 대조했다. Fisher의 어려운 MB 사례 조건수 356678.31→1.59005, 반복 124→9, 좁은 band 42464852.82→1.58573, 등방 11→1이 모두 일치한다. mesh 4096→106과 충돌 finite-cell oracle 비교도 해당 scope로 서술되어 있다.

## 재사용 및 다음 검증

최종 배포용 loader는 고정 oracle을 동봉하고 import 전에 SHA256을 검사한다. 독립적으로 동봉 파일 해시, 보존된 이전 결과의 byte identity, loader 변경 전후 12개 과학 계산 함수의 AST 동일성을 확인했다. 이 portability 변경은 과학 결과를 재실행한 것으로 간주하지 않으며, 별도 `--fvm-only` 실행 및 잘못된 oracle 거부 결과는 각각 보존되어 있다.

현재 증명과 작은 검증은 재사용 가능하다. 다음에는 (1) 실제 invariant를 보존하는 angular transfer, (2) cached collision action, (3) positivity와 결합 오차가 명시된 transport integration, (4) 같은 관측량 오차 상한의 end-to-end timing을 확인한다. 모든 numerical pass는 명시된 double-precision tolerance에서의 검사이며 directed-rounding interval certificate가 아니다.

독립 작은 계산: `python3 analysis/review_spotchecks.py`.
