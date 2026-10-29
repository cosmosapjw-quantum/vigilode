# 독립 결정 검토: 인증 셀·몫대수·다항식 action

작성일: 2026-10-11. 독립 reviewer: `/root/review_algebra`.
대상 source: `8ce9bda0d72d308facd615ac42ec137560fd68c6`.
역할: 후보 생성 및 검증 설계를 수행하지 않은 분리된 decision reviewer. 후보와 production source를 수정하지 않았다. 이 검토는 현재 두 이론 문서에 대한 한 차례 집중 검토이며, 과거 연구 campaign 및 리뷰의 리뷰를 재실행하지 않았다.

**판정: A1–A4, P1–P3 및 P-cost는 명시된 가정 아래의 유도 정리와 구현 계약으로 PROMOTE한다. 현재 production 실행, 전체 ODE 전역 정확도, 측정 속도 승격은 HOLD다.** 보편적 black-box 가속 또는 Laguerre 우월성으로 확대하는 해석은 REJECT한다.

## 검토 범위와 고정 증거

- `CERTIFICATE_ATLAS.md`: 전체 A1–A5.
- `POLYNOMIAL.md`: 전체 P1–P3, finite precision, cost gate, code contract.
- `POLYNOMIAL_CLAIMS.json`: 명제와 가정의 기계 판독 표현.
- `research/promotion_theory_20261011/check_exact.py`, `EXACT_RESULTS.json`: 신규 유리수 예제의 코드와 기록을 읽었다. 검토자가 재실행하지 않았다.
- `POLYNOMIAL_EXACT_CHECK.py`, `POLYNOMIAL_EXACT_CHECK.json`: 신규 6개 예제의 코드와 기록을 읽었다. 유한 예제를 일반 정리의 증명으로 사용하지 않았다.
- 관련 실제 source: `routing.rs::verify_declared_band`, `outward_certificate.rs::InverseWitness::approximate`, `rodas5p-core/src/polynomial_action.rs`의 authority 상태 및 공개 entry points.
- AS05/PY01 preregistration과 `SOURCES_SYNTHESIS.md`의 이식 경계를 확인했다. 과거 source 전체에 대한 재감사 또는 모든 원 논문의 재검증은 아니다.

확정된 문서 SHA256과 항목별 결정은 `REVIEW_ALGEBRA.json`에 기록한다. hash는 검토 대상을 고정하며 수학적 진리 또는 구현 성능의 증거가 아니다.

## 항목별 판단

| 항목 | 결정 | 승인하는 정확한 범위 |
|---|---|---|
| A1 | PROMOTE | 전체 매개변수 box의 verified Bernstein coefficient norm이 q<1일 때의 full-space 역연산자/잔차 correction/반복 오차 정리 |
| A2 | PROMOTE | 같은 셀·계약을 N회 재사용하고 모든 setup/실패 비용을 포함한 명시적 arithmetic work model의 엄밀한 amortization 부등식 |
| A3 | PROMOTE | 제시한 triangular family의 nonempty domain, q=2/5, inverse 및 2회 exact correction 종료 |
| A4 | PROMOTE | verified local expression graph의 whole-domain band support와 row-local bounded circuit 조건의 구조 setup O(nb) 정리 |
| P1 | PROMOTE | Hermitian 정규화 연산자와 full-space near-annihilator witness의 phi interpolation 오차, finite precision 조건부 총 action bound |
| P2 | PROMOTE | exact polynomial action의 quotient compilation 및 정규화 Hermitian 경우의 near-annihilator majorant |
| P3 | PROMOTE | nilpotent/near-nilpotent full-space 구조 아래 종료하는 shifted inverse와 잔차 기반 correction bound |
| P-cost | PROMOTE | 동일한 고정 operator 단위 비용 모델에서 모든 부대 비용을 포함한 충분 work 감소 조건 |
| production/전역 실행/실측 speed | HOLD | 새 이론 계약을 만족하는 native 구현·전체 stage/global 전달·동일 정확도 full-cost 실행 증거가 없음 |

### A1: full-space 인증과 유한정밀도

Bernstein weights는 셀 전체에서 비음수이고 합이 1이다. 따라서 norm convexity는 각 계수의 outward 상계를 모든 theta로 올바르게 확장한다. PB=I−E가 invertible이면 square P와 B 각각도 invertible이므로 B^-1=(I−E)^-1P 및 correction bound가 성립한다. 반복 오차의 E^m 항과 local defect의 geometric convolution도 맞다.

이 결과의 authority는 projected Hessenberg 또는 샘플 JVP가 아니라 실제 B(theta) 전체다. 현재 residual 및 B_true의 assembly perturbation을 각각 감싸고 q_total<1을 확인하는 조건이 문서에 있다. P가 바뀌거나 metric/model epoch/parameter interval이 셀 밖으로 나가면 재사용할 수 없다. arbitrary approximate JVP로부터 자동으로 delta_B를 얻는다고 주장하지 않는다. 이 경계가 유지되는 경우에만 PROMOTE다.

실제 저장된 float q가 1보다 작아 보이는 것만으로 충분하지 않으며, coefficient enclosure, norm accumulation 및 1−q의 양의 하계까지 verified여야 한다. 문서의 outward contract는 이를 요구한다. **이 구현이 이미 존재한다는 승인은 아니다.**

### A2–A3: 양의 비용 감소 및 예제의 정확한 의미

W_old−W_new=N(C_full−C_member)−C_compile−C_extra이므로 stated threshold는 필요 조건을 갖춘 정확한 sufficient condition이다. N=100 예제의 100000→11000은 gain-verification 부분의 명시적 cost model이며 전체 solver 시간 관측값이 아니다. lookup/epoch/member 검사는 공짜가 아니고 실패 비용을 C_extra로 포함한다. 반드시 실제 보장 또는 실현된 재사용 횟수를 사용해야 한다.

A3에서 E(s)^2=0, inverse row 1의 합은 4/5+2s/15이며 s=2에서 16/15이다. 4/3 상계는 유효하나 비최적이다. 2회의 correction으로 exact arithmetic에서 종료한다. 직접 triangular solver 역시 O(n)이므로 이 예제는 그것을 이겼다는 증거가 아니다.

### A4: 구조 증명의 내용과 source 경계

DAG의 변수 support 합집합은 미분 dependency의 보수적인 상계다. 미분 가능한 primitive의 chain rule 귀납법으로 선언된 band 밖의 derivative가 whole-domain identity로 0임을 증명한다. 한 점의 2개 probe와 다르다. row-local reverse AD의 전체 circuit 길이 L, 출력 길이 nb에 대한 O(L+nb), bounded local stencil에서 O(nb)는 타당하다. 공유 graph 전체를 output마다 순회하는 일반 경우까지 확대하지 않았다.

고정 b일 때 source graph 처리·support 검증도 linear implementation을 사용해야 이식된 전체 setup의 O(n) 주장이 성립한다. generic symbolic simplification, root isolation 또는 quantifier elimination 비용을 숨겨서는 안 된다. 이 정리는 그러한 전역 기호 연산을 요구하지 않는 local primitive/support kernel에 한정된다. 일반적인 b에 대한 support-set union 비용은 별도 구현 회계가 필요하다.

초안의 source 서술은 수정됐다. 현재 `verify_declared_band`는 JVP callback이 있으면 그것을 사용하고, 없을 때만 dense Jacobian 및 두 dense reference products를 만든다. 따라서 quadratic→linear 주장은 **dense-reference comparator**와 sealed generated-model route 사이의 구조 setup에만 적용된다. 이미 저렴한 JVP route 또는 banded direct backend에 대한 추가 우위가 자동으로 생기지 않는다.

### P1: phi 함수의 remainder와 finite input rank

k는 0 이상의 정수다. k>=1의 integral representation을 d회 미분하고 beta integral을 적용하면 sup|f_k^(d)|/d! <= (hs)^d/(d+k)!가 맞다. k=0도 exponential derivative로 동일하다. real repeated-node Hermite remainder와 Hermitian spectral theorem을 연결하면 full-space norm(q(X))를 곱하는 상계가 성립한다. 반복 node의 multiplicity를 버리거나 nonnormal real spectrum에 같은 spectral norm 논리를 적용하면 성립하지 않는다.

low-rank input factorization은 정확해야 하며, 근사 factorization이면 원래 입력과 축약 입력의 차이에 Σ||e_k||/k!를 추가해야 한다. 축약 입력에 해당하는 E_alg와 이 항을 함께 쓰는 문서의 구분은 맞다.

Horner의 coefficient 오차와 local defect는 각각 Σ||v|| epsilon_i 및 Σeta_i로 감싼다. 이는 ||X||_2<=1에서만 이 형태로 증명된다. callback/normalization/underflow/AXPY/output 합산을 포함하는 실제 defect enclosure가 없으면 action admission은 HOLD다. polynomial coefficient가 커질 수 있다는 사실을 숨기지 않았으며 scalar compilation cost와 representability 실패를 따로 처리한다.

### P2: Laguerre와 quotient compilation

monic q에 대한 degree<=d 다항식의 한 번 reduction과 quotient-ring recurrence 귀납법은 정확하다. streaming scalar O(md), workspace O(d), short vector d−1 applications는 degree 관계가 인증됐을 때의 arithmetic count다. rational operand bit length나 interval coefficient 폭이 일정하다고 가정한 wall-time 정리가 아니다.

near-annihilator에서 T_(j+1)=a_j T_j−b_j T_(j−1)+tau_j가 맞다. M recurrence는 ||a_j(X)||<=max_[−1,1]|a_j(x)|를 사용하므로 정규화 Hermitian X의 가정이 필요하다. exact quotient identity만은 그 가정보다 넓게 성립한다. 이 두 범위를 기계 판독 파일에서도 분리했다.

새 알고리즘이 수행하지 않는 기존 m-step vector recurrence의 defect를 그대로 더하지 않는 것은 타당하다. 대신 실제 scalar compilation, coefficient enclosure, 짧은 recurrence 및 합산의 오차를 모두 추가해야 한다. 이 방법은 Laguerre만의 장점이 아니므로 basis dominance 또는 PP08 결과의 반전을 의미하지 않는다.

### P3: nonnormal finite jets

W R=I−T^d의 직접 identity로 exact nilpotence 경우와 ||T^d||<1인 near-nilpotence 경우 모두 성립한다. ||T||<1은 필요 없다. N=N0+E에 대한 비가환 telescoping bound도 각 term의 순서를 norm의 submultiplicativity로 지우는 방식이므로 유효하다. 양의 합을 사용하면 difference-of-powers cancellation을 피할 수 있다.

이 정리는 큰 nonnormal gain을 없애지 않는다. 작은 d가 operator products를 제한하더라도 huge G, overflow, 불충분한 input precision은 별도 거절 이유다. full-space nilpotence witness를 vector samples로 대체할 수 없다. sparse triangular direct solve보다 빠르다는 주장은 하지 않는다.

### P-cost: 무엇이 엄밀히 보장되는가

동일한 C_X를 쓰는 명시적 모델에서 new work는 r(d−1)C_X+overhead이고 baseline은 적어도 rmC_X다. stated strict inequality가 정확히 new<baseline을 준다. 서로 다른 실제 operator 비용의 upper/lower bound를 쓰려면 충분조건은 overhead<rm C_old_lower−r(d−1) C_new_upper로 바뀐다. 고정 단위 비용 모델을 runtime 측정값으로 바꾸어 읽으면 안 된다.

projector direct formula는 degree 1 quotient와 동등한 최강 comparator다. 그보다 빠르다는 주장 없이 generic m-step path 대비 m→1 product 감소를 승인한다. fixed degree low-dimensional operator algebra가 없는 generic black-box operator에 같은 감소를 보장하지 않는다.

## novelty와 현재 코드 적용성

Bernstein convex hull, Neumann series, Hermite matrix functional calculus, polynomial quotient, CRT 및 finite jets는 표준 수학이다. 문서가 학술적 신규성을 주장하지 않는 것은 적절하다. repository에도 `outward_certificate.rs::InverseWitness::approximate`의 QuadraticStageProblem 전용 full-space inverse-defect witness가 이미 있다. 새로운 이식 포인트는 **uniform parameter-cell 증명의 재사용, generated-model structural binding, 기존 AS05와 polynomial authority에 대한 통합**이다. 처음으로 Neumann 인증을 발명했거나 해당 기존 witness가 없다고 설명해서는 안 된다.

유도 결과는 production으로 가는 실질적인 충분조건을 준다. 하지만 현재 branch가 private authority constructors, stale-epoch rejection, complete finite-precision receipts, all-stage propagation 및 global error mechanism을 구현했다는 증거는 아니다. 이 검토로 `TotalErrorStatus`, controller, default route 또는 wall-time label을 변경할 수 없다.

## 수정 기록과 종료

1. A4의 dense verification 비용을 JVP 유무에 따라 구분하도록 source 서술을 수정했다. 정리 자체는 바뀌지 않았다.
2. P2 JSON near-bound의 정규화 Hermitian 가정 누락을 수정하고 P-cost의 common operator cost model을 명시했다. exact quotient theorem의 수학은 바뀌지 않았다.

이 두 scope correction이 반영된 고정 파일에 대해 fatal mathematical blocker는 없다. 추가 native 증거가 없는 상태에서 같은 예제를 반복할 이유도 없다. 다음 단계는 승인된 conditional theorem을 명시된 두 structured families 및 generated-model route에 이식하고 각 authority/cost contract를 실제로 충족하는지 확인하는 것이다.
