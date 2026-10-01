# 비-Arnoldi 다항식과 homotopy의 후속 연구 방향

이 문서는 새 성능 실험 결과가 아니라, 현재 소스·사전등록 결과·원문에서 도출한 설계 제안이다. SciSpace는 문헌 후보 발견에 사용했고 수식/구현 권위는 원문 및 독립 유도에 둔다. Wolfram의 이번 계산은 사전등록된 5/4, 31/32, 123/128, 유한 다항식 곱을 확인한 보조 대수 검산이다. 부동소수점 인증이나 iid 가정을 대신하지 않는다.

## 1. 알고리즘을 나누는 기준

사용자가 요청한 “Krylov 대신 Laguerre 등”은 구현상 **Arnoldi/Lanczos 기저 구성 및 직교화 없이 다항식 recurrence로 action을 계산**하는 방향으로 해석하는 것이 정확하다. p_m(A)v 역시 수학적으로 span{v,Av,...,A^m v}에 속한다. 같은 다항식 공간을 사용한다는 사실과 Arnoldi 직교화 비용을 지불한다는 사실은 별개다.

| 후보 | 먼저 허용할 domain | 비용/이득 | 새로 확보해야 할 권위 |
|---|---|---|---|
| 현재 joint Chebyshev | 검증된 대칭 비양 연산자 | 짧은 recurrence, 여러 phi 계수 공유, frozen h/operator의 cache | large-scale matrix-free domain 증명, 입력 weight 형성 오차와 총오차 합성 |
| 현재 joint Laguerre | 같은 domain, scale beta>0 | [0,+infinity) 직교계, exp(-aX) 양의 계수 표현 | recurrence rounding propagation, beta-degree 공동선택, 큰 exp(rho/(2 beta)) 제한 |
| Leja/Newton | spectral/norm 정보를 제공하는 real/complex domain | 점을 늘리는 nested interpolation, 직교화 없는 action | divided difference 안정성, scaling/degree/point-domain 선택, nonnormal error control |
| scaled Taylor/block phi | 일반 matrix-free operator를 연구 대상으로 | block matvec, trace/shift 및 scaling과 degree 선택 | backward/forward error 권위 구별, nonnormal/overflow guard, 전체 phi 조합 오차 |
| Faber 또는 rational 후보 | 추가 domain/witness 확보 뒤 | 복소 spectrum 또는 매우 stiff 구간을 겨냥 | contour/resolvent 또는 shifted-solve 보장; 이번에 구현·실행하지 않음 |

Chebyshev/Laguerre의 대칭 증명을 eigenvalue만 비슷한 비정규 A에 그대로 적용하면 안 된다. 캐시는 operator epoch, h의 bits, enclosure, basis, scale, degree를 묶어야 하며, setup 재사용 기간이 짧으면 이득이 사라진다. “warm” 수치는 coefficient 생성비만 제외한 정확한 조건을 표시하고, certification 비용과 캐시 무효화 빈도를 포함한 전체 적분 비교를 다음 별도 사전등록에 둔다.

## 2. 읽은 원문과 활용 범위

- Al-Mohy–Higham (2011), *Computing the Action of the Matrix Exponential, with an Application to Exponential Integrators*, DOI 10.1137/100788860, https://eprints.maths.manchester.ac.uk/1591/1/alhi11.pdf . Scaled truncated Taylor와 block action, phi 조합의 augmentation을 후속 baseline 설계 근거로 삼는다. 논문의 실험을 VigilODE 실험으로 전용하지 않는다.
- Caliari–Kandolf–Ostermann–Rainer, *The Leja method revisited: backward error analysis for the matrix exponential*, https://arxiv.org/abs/1506.08665 , 확인한 PDF는 v2 (2016-04-01), DOI 10.1137/15M1027620. Scaling·degree·interpolation interval을 함께 정하는 backward-error 설계가 핵심 참고사항이다. 현재 프로젝트의 forward certificate가 자동으로 성립한다는 뜻이 아니다.
- Al-Mohy, *Computing Linear Combinations of phi-Function Actions for Exponential Integrators*, https://arxiv.org/abs/2509.26475 , https://arxiv.org/html/2509.26475v1 . 조회된 식 (1.2), §§2–4는 stage abscissa t_i와 weight alpha_i를 분리한 block 계산 방향을 제공한다. 논문 자체도 shift/scale 목적함수만으로 truncation tolerance가 보장되지 않는다고 명시한다. 따라서 residual 두 항이 작다는 stop rule을 엄밀한 인증으로 그대로 이식하지 않는다. arXiv 제출 표시는 2025-09-30이고 HTML 본문 날짜는 2026-08-24로 달라 조회 버전/표시를 그대로 기록한다. 저자 성능 주장은 이번에 재현하지 않았다.
- NIST DLMF §10.35, https://dlmf.nist.gov/10.35 및 §18.14.8, https://dlmf.nist.gov/18.14.E8 . Bessel/Chebyshev expansion과 Laguerre bound의 출발점만 제공한다. 유한 정밀도 계산 오차는 별도 유도·검산한다.

위 문헌은 2026-10-01에 원문 위치를 확인했다. SciSpace 검색 결과의 초록만으로 정리의 전제나 최신성을 확정하지 않았다.

## 3. 순차 의존성을 줄일 수 있는 부분

현재 homotopy는 decoupled lambda=0에서 출발해 coupled lambda=1의 stage target을 예측·보정한다. 병렬성은 공통 W에 대한 여러 stage RHS를 동시에 풀 때 생긴다. lambda continuation을 더 세분화하면 각 lambda 내부는 병렬화할 수 있지만 continuation 자체의 의존성과 총 반복 수가 늘어난다. 단순히 loop를 parallel로 바꾸는 패치로는 이 의존성이 사라지지 않는다.

엄밀 하삼각 H에 대한 유한 경로합 doubling은 의존 깊이를 줄이는 정확한 대수다. 그러나 H^2,H^4를 만드는 비용·메모리·rounding bounds를 함께 계산해야 한다. 조밀 (sn)x(sn) 행렬을 만드는 구현에서는 깊이 O(log s)가 wall-time 개선을 뜻하지 않는다. 이번 diagonal-component factorization 후보처럼 구조를 보존하여 storage와 연산량을 함께 줄이는 것이 먼저다.

권장 순서는 (1) persistent worker pool과 preconditioner/W reuse, (2) 같은 operator의 block matvec 및 다중 RHS batching, (3) diagonal/banded certificate 표현과 성분별 closure, (4) 안전한 사전 reject로 fallback 낭비 제한, (5) q1의 operational gate와 q2의 certificate를 명확히 분리한 matched-accuracy 전체 적분 비교다. 출력 격자, 실패·거절·speculation, certificate 준비비, 정확도와 memory를 모두 같은 회계에 넣는다.

시간 구간 전체를 병렬화하는 방법은 stage homotopy와 다른 연구 문제다. 특히 oscillatory/non-normal 문제에서 coarse propagator의 안정성과 반복 횟수에 대한 별도 검증 없이 현재 단계 인증을 시간 병렬 정확도 인증으로 확대할 수 없다. 이번 작업에서 시간 병렬 solver를 구현했다는 주장은 하지 않는다.
