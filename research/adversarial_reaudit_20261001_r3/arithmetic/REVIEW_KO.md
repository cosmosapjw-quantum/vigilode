# R3 arithmetic·φ·budget 재감사

검토 source는 `cc2cd041737e7ff543624d1b59893a3b4397369f`, tree는 `182fa306067b84ee10ec1f1b1e5fbad9e769aae1`이다. production 파일을 수정하지 않았다. 과거 기록의 PASS를 물려받지 않고 이전 probe의 소스 바이트를 R3 dependencies에 연결하여 새로 실행했다. 수학 참조값은 표현된 binary64 입력을 정확한 유리수로 바꾸거나 Python 표준 라이브러리 Decimal 110자리로 계산했다.

**기존 PHI-R1, PHI-R2, R2-POL-01의 원 재현은 닫혔다.** 다만 가중치 일부의 underflow를 단순 횟수로만 기록하면 비정규 연산자가 소실 성분을 증폭하는 경우를 막지 못한다. 이를 독립적인 유한 입력 반례로 확인했다. 새 finding은 **R3-ARITH-01, P2**, 극단적인 유한 동적 범위에서 원 입력의 φ action에 대한 부정확한 convergence 판정이다. 보통 크기의 ODE 적분이 실패한다는 주장이나 실제 campaign 오염 주장은 하지 않는다.

## 1. 수정 효과와 실행 범위

| 대상 | 새 실행·독립 기준 | 결과·한계 |
|---|---|---|
| PHI-R1, `h=±1e-100,b4=1e300` 및 `h=1e100,b4=1e-300` | native dense/fused 6행; exact `Fraction(h)^4 Fraction(b4)/24` | 최대 상대오차 `8.4984e-16`; 원 반례 해결 |
| PHI-R2, `A=-1,h=0.1,w1=c` | native dense/fused 20행, `c=1e-200`…`1e300`; Decimal closed form | 최대 상대오차 `3.9271e-16`; 원 진폭 의존 오류 해결 |
| R2-POL-01 | exact represented-input rational과 native public policy 비교 | 예산 `0.9999888671826829`, 상대오차 `1.5582e-16`; WRMS 5 거절 |
| epsilon_ref=0와 극소 step term | 실제 public policy | 각각 예산 0, WRMS 5 거절 |
| `times_power` 경계·부호·subnormal | 고정 seed 512개와 구조적 8개, exact Fraction | 520개에서 올바른 finite/overflow/zero 범주; 유한 결과의 correctly-rounded 참조 대비 최대 2 ULP. 모든 binary64 입력의 보증은 아님 |
| 과거 φ probe 전체 | 104행: production 91행, 과거 standalone candidate 13행 | 새 실행 exit 0. 104개를 독립 production test 수로 세지 않음 |

기존 repository의 `r2_phi_range_contracts`와 `r2_output_budget_contracts`는 runtime lane이 실행한다. 본 lane의 native evidence와 전체 suite의 테스트 수는 중복 합산하지 않는다. PHI-R1 prefix 경계의 기존 회귀도 해당 suite의 실제 실행 기록을 참조한다.

`binary_split`/`binary_power`/`binary_scale`로 곱의 mantissa와 exponent를 분리한 수정은 적절하다. 단, subnormal에서 상대오차 `O(k ε)`라는 함수 설명은 그대로 읽으면 너무 강하다. 참조값이 최소 subnormal에 접근하면 상대오차 대신 절대 half-ULP 오차 항을 포함해야 한다. 본 sampling은 문서 전체의 수학적 보증이 아니며, 예산의 directed-rounding certificate도 아니다. closure 문서에 이미 명시된 `error_upper <= budget_lower` 미구현은 그대로 남는다.

## 2. R3-ARITH-01 — 부분 가중치 소실 후 원 문제 convergence 판정

입력은 다음과 같다. 실수 시간 convention에서 `h`는 시간, `A`는 역시간 단위이며 아래 예는 수치 단위를 고정한다. 음의 h도 public φ API가 허용하여 별도 실행했다.

\[
A=\begin{pmatrix}0&10^{308}\\0&0\end{pmatrix},\quad
h=\pm10^{-8},\quad b_0=(10^{-300},0)^T,\quad b_1=0,\quad b_2=(0,10^{-310})^T.
\]

모든 입력은 유한한 binary64이다. `A²=0`이므로 급수는 정확히 종료한다.

\[
e^{hA}=I+hA,\qquad \varphi_2(hA)=\tfrac12I+\tfrac16hA,
\]
\[
u=e^{hA}b_0+h^2\varphi_2(hA)b_2
=\begin{pmatrix}b_{0,1}+h^3 A_{12}b_{2,2}/6\\h^2 b_{2,2}/2\end{pmatrix}.
\]

표현된 입력에 대한 exact Fraction 결과에서 첫 성분은 `h>0`일 때 `1.66666666666666169779916068…e-27`이다. 둘째 성분은 `4.9999999999999849…e-327`로 최종 binary64에서 0으로 반올림된다. 첫 성분은 정상 범위에 있으며 충분히 표현 가능하다. `h<0`이면 주요 첫 성분의 부호가 바뀐다.

native fused와 prefix는 두 부호 모두 첫 성분 약 `1e-300`을 반환하면서 `converged=true`, `convergence_basis=invariant-subspace`, `error_estimate=0`을 보고했다. `phi_weight_underflows=1`은 실제로 기록된다. dense wrapper는 `Ok([0,0])`을 반환했다. 오차가 있는 것은 최종 표현 불가능한 둘째 성분이 아니라 **표현 가능한 첫 성분**이며 상대오차는 사실상 1이다.

원인은 `h² b₂≈1e-326`을 먼저 binary64 0으로 만든 후, 원래 `φ₂(hA)`가 그 성분에 가할 증폭을 잃는 데 있다. Arnoldi가 이후 정확하게 포착한 불변 부분공간은 이미 변경된 가중 입력의 부분공간이다. 원 입력에 대한 오차가 0임을 뜻하지 않는다. `A=0` 대조군은 같은 부분 underflow를 겪지만 첫 성분의 상대오차가 약 `1.66e-16`이고 최종 둘째 성분은 실제로 표현 불가능하다. 따라서 소실 횟수만으로 오류의 중요도를 판단할 수 없으며, 무조건 거절은 안전하지만 보수적이다.

관련 위치:

- `crates/rodas5p-core/src/binary_scaling.rs:136–155`: partial loss를 허용하고 횟수 반환; 전부 소실될 때만 오류.
- `crates/rodas5p-integrators/src/exponential.rs:1356–1360`: 횟수만 WorkCounters에 합산하고 가중 벡터 반환.
- 같은 파일 `1551–1566`, `1912–1946`: 원 입력 변환 오차와 무관하게 fused/prefix convergence 보고.
- `crates/rodas5p-core/src/matrix_functions.rs:252–253`: dense wrapper가 weighting-loss 정보를 버림.

이 문제를 이전에 정상 작동하던 입력의 회귀라고 판정하지 않는다. R2 원 반례는 닫혔고, 새 부분 소실 처리의 correctness 범위를 확장 검토하면서 드러난 잔여 gap이다. dense가 여기서 `b0`마저 0으로 만드는 것은 매우 큰 비정규 행렬의 기존 Padé 한계도 섞여 있다. 핵심 finding은 dense 오차의 단일 원인 추정에 의존하지 않으며 native fused/prefix와 exact nilpotent oracle만으로 성립한다.

## 3. 수학·코딩 연구 루프: 무엇을 고쳐야 하나

정확한 가중치를 `w_k=h^k b_k`, 저장된 가중치를 `w̃_k`, 소실·반올림 차이를 `δw_k=w_k-w̃_k`라 두자. 선형성에 따라 물리 출력의 변환 오차는

\[
e_{\rm transform}=\sum_{k=0}^{p}\varphi_k(hA)\,\delta w_k.
\]

따라서 Arnoldi truncation·projection 오차와 별개로

\[
\|e_{\rm transform}\|\le\sum_k C_k\|\delta w_k\|,\qquad
C_k\ge\|\varphi_k(hA)\|
\]

를 예산에 넣거나, 그런 상계를 얻지 못한 입력을 거절해야 한다. `δw_k` 자체가 binary64 최소 양수보다 작을 수 있으므로 그 bound도 mantissa/exponent 또는 구간·확장 정밀도 형태로 보존해야 한다. 변환 오류를 다시 0에 저장하면 같은 결함이 반복된다.

Euclidean operator norm에서는 검증된 logarithmic norm 상계 `μ≥μ₂(hA)`가 있다면 `k≥1`에 대해 적분 표현으로 `∥φ_k(hA)∥≤φ_k(μ)`를 얻어 `C_k=φ_k(μ)`를 선택할 수 있다. 이는 충분조건이다. 강한 비정규 행렬에서는 지나치게 보수적이며, 본 nilpotent 예에서 `μ₂(hA)`는 매우 커져 실용적이지 않다. 구조적 상계나 입력별 action bound가 필요한 이유다. 이를 일반 matrix-free 코드에 구현하거나 directed rounding으로 검증한 것은 아니다.

실제로 실행한 두 standalone 후보는 다음과 같다.

1. **부분 소실의 명시적 실패:** `lost>0`이면 일반 convergence 승격 전에 오류를 반환한다. 세 입력 모두 거절한다. 무증거 성공을 막는 최소 수정이나 `A=0` 대조군도 거절하므로 범위 확대를 위한 최종 방법은 아니다. 이번 probe에서는 production 호출 결과와 후보의 admission 결정을 함께 기록했으며 production 소스는 바꾸지 않았다.
2. **nilpotent 구조를 이용한 재배열:** `A12*b2`를 먼저 안정적으로 형성하고 `h³`을 적용하여 정확히 종료하는 다항식을 계산한다. 양·음 h에서 참조 대비 첫 성분 상대오차 `6.53e-17`, `A=0`에서 정확한 첫 성분을 얻었다. 이는 이 2차원 구조에서 가능한 계산 순서의 증명·검산이며, 일반 φ backend 교체가 아니다. 이 결과는 tiny coefficient를 먼저 버리지 않는 polynomial/action 설계가 실제로 필요한 경우를 제공한다.

향후 하네스는 `error_estimate`를 계산한 문제의 identity를 분리해야 한다. 원 `(A,h,b)`와 변환 `(M_hat,q)`의 연결이 exact 또는 bounded인지 상태를 남기고, 변환 오차가 미평가이면 불변 부분공간 성공만으로 원 문제의 tolerance 달성을 승인하지 않아야 한다.

## 4. 기존에 명시된 dense oracle 한계의 정량 확인

`h=1`, `A=diag(700,-700)`, `w0=(1e-300,1e300)`에서 Decimal 참조는 약 `(10142.32054735,9.85967654376e-5)`다. native dense report는 `(0,9.85967654376e-5)`를 반환하면서 `mixed_range=true`, `output_below_input_half_precision=true`를 표시했다. 이는 **이미 보고서가 경고한 범위의 실증**이며 별도 신규 finding으로 세지 않는다. `dense_fused_phi_action`처럼 라벨을 제거하는 wrapper를 precision oracle로 쓰려면 소비자가 라벨 또는 독립 reference를 필수로 받아야 한다. G3 기본 입력에서 실제 오판정이 발생했다고 주장하지 않는다.

이 fixture는 weight-relative normwise 설명이 실제 출력의 상대정확도와 다르다는 점도 보여준다. 입력 scale에 비하면 1e4 오차는 작아도 해당 물리 출력은 거의 전부 틀릴 수 있다. 단일 공통 amplitude normalization만으로 이 문제를 완전히 해결할 수 없으며 성분별 scaling, 구조별 분해, cancellation-aware 합산 또는 더 높은 정밀도 oracle을 별도로 검토해야 한다.

## 5. 구체적인 다음 개발 계약

`NEXT_STEPS.json`의 의존성·수락 조건을 사용한다. 우선 partial-underflow 상태를 실제 action report와 dense reference consumer에 전달하고, 증폭 bound가 없는 경우 명시적 미평가 또는 오류로 닫는다. 그 뒤 weight rounding의 mantissa/exponent bound를 정의하고 출력까지 전파한다. 원 R2 closures, 양·음 h nilpotent 반례, `A=0` 대조군을 같은 checkout에서 실행해야 한다. 동적 범위 라벨을 버리는 oracle consumer도 fail-closed 또는 독립 high-precision 검증으로 바꾼다. budget은 범위 평가 수정과 별개로 directed lower bound가 필요한 certificate API를 분리한다.

독립 reviewer가 아직 연구 후보를 production으로 승격한 것은 아니다. 본 lane의 결론은 원 반례 closure와 새 native correctness witness, bounded standalone repair evidence까지다. 실제 일반 비정규 인증, 전체 입력 범위의 올바른 반올림, 실제 ODE 성능·안정성·속도 향상은 미확립이다.

## 재현·실행 provenance

`probe/src/legacy_closure.rs`는 R2 probe 원본과 byte-identical이다. 새 source를 검사하는 R3 패키지로 의도적으로 연결했으며 R2 immutable runner를 수정하거나 우회 실행한 것이 아니다. `probe/src/main.rs`가 새 native counterexample·candidate·power sampling, `exact_oracle.py`가 Fraction/Decimal 독립 계산, `analyze_closures.py`가 closure 집계를 담당한다. raw native 529행, legacy 104행, native tableau 1행은 각각 `NATIVE_EXECUTION.json`의 실제 exit 0과 연결된다. 표준 라이브러리 oracle·closure 분석도 exit 0이다.

첫 build는 작성한 probe manifest의 상대경로 오류로 exit 101이었으며 수정 전 로그를 보존했다. 두 번째는 공유 target의 zero-length object 파일로 exit 101이었다. runtime lane이 생성된 cache를 복구하고 incremental을 끈 뒤 같은 production source로 25.05초 build exit 0을 얻었다. 이 환경 오류를 알고리즘 실패로 세지 않았다. 자세한 source/probe/output 해시는 `EXECUTION_RECEIPT.json`에 있다.
