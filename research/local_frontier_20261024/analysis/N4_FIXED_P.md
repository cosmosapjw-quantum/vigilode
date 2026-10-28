# 고정 역연산자를 사용하는 비선형 continuation

## 구현 결과와 범위

`rodas5p_integrators::fixed_point_continuation`에 일반 유한차원 방정식
\(F(x,\lambda)=0\), \(0\le\lambda\le1\)을 위한 선택적 API를 추가하였다.
`continue_fixed_point`는 F의 값과 고정된 선형 P의 작용만 호출한다. 이 인터페이스에는
Jacobian, JVP, AD 또는 Newton callback이 없다. P의 준비 과정과 전 영역 경계의
생성은 provider의 별도 의무다. 기존 `nonlinear_homotopy`의 특정 ODE 인증 경로와
일반 Newton corrector를 변경하지 않았다.

가정과 구현을 연결하는 핵심은 다음 네 가지다.

1. 불변 target/coordinate scaling, epoch, branch, P identity와 patch를 결합한다.
2. 닫힌 infinity ball 전체의 수축성과 자기 사상 조건을 선언한다.
3. patch 교체 때 이전 근의 오차볼이 새 domain에 포함되는지 확인한다.
4. 최종 \(\lambda=1\)에서 현재 원래 F를 실제로 평가하여 허용 잔차를 만족한 뒤
   caller 출력에 한 번만 복사한다.

provider가 선언한 경계의 진실성은 소수의 callback 표본으로 증명하지 않는다.
선언형 타입에는 `Deserialize`가 없으며, scalar constructor 검사는 수학적
경계 생성의 대용물이 아니다. 이 절차가 인증하는 대상은 주어진 root 문제의 잔차와
조건부 근 오차이다. ODE의 전역 시간 적분 오차는 별도의 적분기 계약이다.

## 1. 정리: Newton 없는 patch의 존재·수렴과 작업량

유한차원 \(\mathbb R^n\)에 infinity norm을 고정한다. 성분에 단위가 다르면
caller가 먼저 좌표와 잔차를 스케일링하고 그 관례를 target identity에 포함한다.
하나의 patch는 닫힌 볼 \(B(c,r)\)와 매개변수 구간 \([a,b]\)이다. 다음을 가정한다.

- F는 \(B(c,r)\times[a,b]\)에서 연속이다.
- P는 patch 안에서 변하지 않는 가역 선형 연산자이다.
- \(T_\lambda(x)=x-PF(x,\lambda)\)는 모든 \(\lambda\in[a,b]\)에서
  공통 상수 \(0\le q<1\)로 수축한다.
- \(\sup_\lambda\|PF(c,\lambda)\|_\infty\le\beta\le(1-q)r\).

그러면

\[
\|T_\lambda(x)-c\|_\infty
\le\|T_\lambda(x)-T_\lambda(c)\|_\infty+
\|T_\lambda(c)-c\|_\infty
\le qr+\beta\le r.
\]

따라서 Banach 정리에 의해 각 매개변수에서 유일한 고정점 \(x_*(\lambda)\)가
존재한다. P가 가역이므로 이 고정점은 F의 근이다. 고정점 반복은

\[
x_{k+1}=x_k-PF(x_k,\lambda),\qquad
\|x_k-x_*\|_\infty\le q^k\|x_0-x_*\|_\infty
\]

를 만족한다. 이 유도에는 온라인 미분 계산이 없다. 수축성 자체는 식을 아는 모델의
다항식 경계, interval enclosure, 기존 analytic bound 등으로 공급해야 한다.

추가로 \(\|P^{-1}\|_\infty\le v\)라 하자. 정의식을 빼면

\[
P(F(x,\lambda)-F(x_*,\lambda))
=(x-x_*)-(T_\lambda(x)-T_\lambda(x_*)).
\]

그러므로 \(\|F(x,\lambda)\|_\infty\le v(1+q)\|x-x_*\|_\infty\)이고,
볼 안에서 시작한 정확 산술 반복에 대해서

\[
\|F(x_k,\lambda)\|_\infty\le 2rv(1+q)q^k.
\]

\(0<q<1\)이면 우변이 목표 \(\tau\) 이하가 되는 k까지의 반복으로 충분하다.
\(q=0\)이면 한 번의 갱신으로 고정점에 도달한다. k회 갱신과 마지막 잔차 검사는
k+1회의 F 및 P 작용을 사용한다. 정상 완료한 전체 호출 수는 각 station의
\(k_j+1\)의 합이다. F와 P 작용의 비용이 각각 \(C_F(n),C_P(n)\)이면
작업량은 \(O(\sum_j(k_j+1)[C_F(n)+C_P(n)+n])\)이다.
준비 비용은 이 합 밖에 별도로 더해야 한다. 이 상계는 q와 P가 유효한 patch에
조건부이며, 모든 nonlinear root 문제의 복잡도 감소 명제가 아니다.

## 2. 반올림과 원래 잔차

callback이 반환한 계산 벡터를 \(\widehat f\), \(\widehat d\)라 하고

\[
\|\widehat f-F(x,\lambda)\|_\infty\le e_F,
\qquad\|\widehat d-P\widehat f\|_\infty\le e_P,
\qquad\|P\|_\infty\le u
\]

를 provider가 보장한다고 하자. 코드의 두 수용·전달량은

\[
R=\|\widehat f\|_\infty+e_F,
\qquad
E=\frac{\|\widehat d\|_\infty+e_P+u e_F}{1-q}.
\]

R은 원래 F의 잔차 상계다. 또한
\(\|x-x_*\|\le\|x-T(x)\|/(1-q)=\|PF(x)\|/(1-q)\)이므로 E는 근 오차
상계이다. 두 계산은 기존 `rodas5p_core::directed`의 outward 연산을 사용한다.
callback이 오류 상계 0을 반환하려면 그 평가가 정확해야 한다. 코드가 임의의 opaque
callback에 대해 오차 상계를 만들어주는 것은 아니다.

업데이트 자체는 binary64로 수행하고 매번 새 F와 P를 평가한다. 새 iterate가
domain 밖이면 즉시 거절한다. 누적 업데이트 반올림을 무시한 선험적 k 계산으로
성공을 선언하지 않는다. 실제 R만 최종 수용에 사용한다.

두 번째 수정에서는 domain 검사 비용을 줄였다. \(d=|\operatorname{fl}(x_i-c_i)|\)와
표현 가능한 r에 대해 \(d<r\)이면 정확 차이도 r보다 작고, \(d>r\)이면 정확
차이도 r보다 크다. 이는 standard binary64 round-to-nearest의 단조성에서 따른다. 이 경로에는 fast-math를 사용하지 않는다. 경계 \(d=r\)만
outward subtraction으로 판정한다. 예를 들어 \(x=1,c=-2^{-53},r=1\)에서는
nearest subtraction이 1을 주지만 정확 차이는 r보다 크다. 양·음 경계의 이 사례를
별도 테스트하여 fast path가 잘못된 내부 판정을 하지 않음을 확인한다.

## 3. patch 연결과 기하학적 의미

이전 patch의 끝에서 근 \(x_*\)를 담는 \(B(\widehat x,E)\)가 있고 다음 patch가
\(B(c_{next},r_{next})\)라 하자. 코드가

\[
\|\widehat x-c_{next}\|_\infty+E\le r_{next}
\]

를 outward 계산으로 확인하면 이전의 참 근이 새 domain 안에 있다. 새 patch에서도
동일한 F의 근이 유일하므로 두 patch는 공통 매개변수에서 같은 근을 가리킨다.
이 포함 검사는 branch ID의 문자열 일치를 수학적 근거로 사용하는 것을 피한다.
각 patch의 근은 F의 매개변수 연속성과 공통 수축성으로 연속이며, 따라서 이 연결은
동일한 연속 근 가지를 추적한다.

`two_step_normal_reflection_walls_minor_revision_v0_1.pdf` §2의 regular carrier,
§6의 threshold 밖 continuation과 `curved_mirror_integrated_research_report_v13.pdf`
의 carrier degeneration / critical projection / target collision 구분을 이 계약에
연결하였다. 반사 문제의 구체적인 fold 정리를 ODE에 그대로 이식한 것은 아니다.
원래 방정식, 정규 domain, 근 가지를 별개로 관리하는 구조를 수치 API로 옮겼다.
예를 들어 \(F=x^2-\lambda\)의 fold에서 모든 유한 P에 대해 \(T_x=1\)이므로
fold를 포함하는 honest uniform-contraction patch는 만들 수 없다. 이는 이 provider의
적용 경계이며, arc-length 등 다른 continuation 방법의 불가능성 주장이 아니다.

## 4. API, 실패 처리, 검증

| 구현 대상 | 실제 계약 |
|---|---|
| `DeclaredFixedPointPatch` | q, beta, radius, P/P-inverse norm bounds와 parameter domain |
| `DeclaredFixedPointAtlas` | 정확히 [0,1]을 덮는 인접 patch, dimension과 branch 일치 |
| `FixedPointHomotopyProvider` | pure F/P callback 및 각각의 evaluation error upper bound |
| `continue_fixed_point` | λ=0 보정, 제한된 station 진행, overlap, 최종 F(x,1) 수용 |
| `FixedPointContinuationWork` | 실패도 포함한 누적 F/P 호출·갱신·station·overlap 횟수 |

모든 P identity를 첫 F/P 호출 전에 확인한다. 각 callback 뒤에도 epoch와 identity를
다시 확인한다. partially written output은 NaN poison으로 거절한다. callback 실패,
비유한 계산, domain 이탈, 예산 소진, branch overlap 실패는 서로 다른 오류이다.
어떤 실패에서도 caller 출력은 바뀌지 않는다. 성공 때만 마지막 내부 상태를 복사한다.

첫 루프의 12개 테스트는 모두 통과하였다. 독립 검토는 whole-domain 선언이 참이라는
조건 아래 contraction, error transport, overlap proof와 실제 코드 경로를 대조하였다.
두 번째 루프에는 norm-product 모순 거절, rounded boundary, P callback 실패·부분 쓰기·
epoch 변경 검사를 추가하여 15개 테스트가 모두 통과하였다. 원래 로그와 소스 해시는 `evidence/n4-*`에 보존한다.

## 5. 경량 비용 비교와 로컬 후속 측정

실험은 분리 가능한 monotone cubic
\(F_i=x_i+\kappa x_i^3-[1/4+\lambda/2+i/(128n)]\)를 사용한다.
\(\kappa\in\{1/64,1/16,1/8\}\), \(n\in\{1,128\}\)이며 P=I이다.
\(B(c=1/2,r=1/2)\)에서 \(q=3\kappa\), \(\beta\le0.27\)이므로 위 조건을
직접 확인할 수 있다. 독립 참조는 더 작은 잔차 목표의 bisection이다.

비교군은 동일한 최종 root만 푸는 vector bisection이다. 새 경로는 λ=0과 λ=1을
모두 풀므로 그 추가 비용도 포함한다. 두 풀이의 allocation을 포함하고 immutable
model/atlas 구성은 제외하였다. 각 조건에서 두 방법의 순서를 교대하여 101회 측정했다.
첫 결과에서 F 호출은 새 경로가 11–21회, bisection은 29–34회였으나, 값싼 polynomial
callback에서는 domain 검사의 비용 때문에 bisection이 더 빨랐다. 그 원인을 반영한
두 번째 구현 결과는 `N4_FIXED_P.json`의 수치와 최종 raw stream에서 확인할 수 있다.

이 비교의 범위는 bounded nonlinear root client이다. local full ODE campaign에서는
F/P 준비 비용, 실제 expensive callback, 원래 corrector의 동일 residual/branch 조건,
실패 비용과 patch 전환 비용을 함께 측정해야 한다. 현재 기본 ODE dispatch는 바꾸지 않는다.

최종 측정 요약은 다음과 같다. 앞 두 시간 비율은 각각 수정 전 고정-P 시간/최종 고정-P
시간과 최종 bisection 시간/최종 고정-P 시간이다. 수정 전·후의 호출 수와 잔차는 동일하다.

| 차원 | κ | 이전/최종 | bisection/최종 | F 호출: 고정-P / bisection |
|---:|---:|---:|---:|---:|
| 1 | 0.015625 | 1.491 | 0.355 | 11 / 32 |
| 1 | 0.0625 | 1.517 | 0.246 | 16 / 29 |
| 1 | 0.125 | 1.546 | 0.202 | 20 / 30 |
| 128 | 0.015625 | 5.313 | 1.565 | 12 / 34 |
| 128 | 0.0625 | 5.421 | 1.102 | 17 / 34 |
| 128 | 0.125 | 5.475 | 0.892 | 21 / 34 |

128차원에서 앞 두 κ 조건은 최종 bisection보다 각각 약 1.56배와 1.10배 빨랐다.
κ=1/8과 모든 scalar 조건에서는 bisection이 더 빨랐다. 값싼 scalar callback의
sub-microsecond 측정은 특히 계측·할당 비용의 영향을 크게 받는다. 독립 프로세스 반복과
실제 provider 준비 비용을 포함한 local 측정 전에는 이 수치를 넓은 성능 결론으로 사용하지 않는다.
