# R4 다항식·산술 재감사 결과

검토 source는 `1c54194123ee6abc6daa512e8574922f510b4e2c`이고, 실행 전 사전등록은 `b2914f3e3c03eda60a8547619db6284aac8250a2`이다. 원 R3의 partial-weight-loss 결함은 이번 native 재현에서 닫혔다. 새 Chebyshev 구현은 시험한 32개 `Certified` 계산에서 모두 실제 오차를 감쌌고, Laguerre 등의 미인증 범위도 명시적으로 분리한다. 다만 새 research `transform_bound`의 일반적인 상한은 아직 sound하지 않다. 실제 생성된 저차 가중치만으로 인증 상한을 약 25.9배 초과하는 반례를 확인했다.

이번 산출물은 production 변경이 없는 standalone 감사·후보 연구다. 새 native 프로그램 78행과 독립 oracle은 각각 exit 0이다. 컴파일·실행·oracle 오류는 없었고, 공유 build lock 대기는 환경 실패로 세지 않았다. `EXECUTION_RECEIPT.json`에 실행 명령, 산출물·binary 해시를 기록했다. 새 timing이나 speedup 실험은 하지 않았다.

## 1. 원 결함의 closure와 개선 인정

원 R3 반례 A₁₂=1e308, h=±1e−8, b₀=(1e−300,0), b₂=(0,1e−310)를 재현했다. fused/prefix 각각 두 부호, 총 네 report는 `converged=false`와 `transform-error-unbounded`를 반환한다. dense 경로 두 부호 역시 typed error를 반환하며 report authority는 false다. 원 target에서 잃은 가중치를 숨기고 수렴을 주장하던 결함은 이 재현에서 해결됐다. 실제 변환오차 약 1.6666666666666617e−27은 새 helper의 원 반례 상한 약 1.6666666666666623e−27 안에 들어갔다.

현재 production fused-phi 경로가 새 transform bound를 바로 사용하지 않고 보수적 거절을 유지하는 점은 의미 있다. 다음 절의 helper 결함은 이 기존 거절의 closure를 뒤집는 증거가 아니다. `rg`로 확인한 bound helper의 consumer는 현재 contract tests이며 production integration이 아니다.

## 2. 새 반례

| ID | 우선순위·범위 | 직접 관측 | 원인·수정 |
|---|---|---|---|
| R4-ARITH-01 | P2, research bound | A=[[0,.25],[0,0]], h=1000.1, b₂=(0,1), 실제 `weight_phi_vectors` 출력에서 참 변환오차 1.50717820932065e−9, 상한 5.82076609134674e−11 | `ExpBound::ZERO`의 exponent 0이 .25의 exponent −1보다 크게 비교되어 row max를 지우고 최종 N을 0으로 만든다. zero-aware order를 한 곳에 정의한다. |
| R4-ARITH-02 | P2, generic-order research API | A=0, h=1.1, p=171의 참 오차 2.08101865254415e−317, 상한 2.77921231545613e−330. p=172도 실패 | binary64 factorial이 171에서 Inf가 되어 reciprocal bound가 잘못된다. 1/k! outward recurrence 또는 명시적 지원 order 제한이 필요하다. |
| R4-ARITH-03 | P3, 외부 입력 계약 | A=0, h=1, b₀=1, stored₀=−1에서 참 오차 2인데 상한 0과 zero-tolerance 승인. 오차 상한 1에 tolerance −10도 승인 | order 0의 signed difference와 tolerance validation이 빠졌다. 실제 weight producer는 b₀를 그대로 보존하므로 이 signed-input 반례는 producer 자체의 결함이 아니다. |

첫 반례의 수학적 확인은 oracle에 외부 행렬함수 구현이 필요 없다. A²=0이면 φ₂(hA)=I/2+hA/6이다. δ=h²−stored₂를 exact binary64 입력의 유리수 차이로 계산하면

    e = (h δ/24, δ/2),   ||e||₂² = (h δ/24)² + (δ/2)².

상한 역시 `mantissa × 2^exponent`를 정확한 유리수로 읽어 제곱 비교했다. 따라서 반례의 실패 판정은 Decimal precision이나 dense exp oracle에 의존하지 않는다. 제시한 소수는 읽기 쉬운 표시용이며 정확한 입력 bits와 분수는 `native.jsonl`, `ORACLE_RESULTS.json`에 있다.

p=171/172의 반례도 A=0이므로 e=(hᵖ−storedₚ)/p!를 정확한 유리수로 계산한다. 현 joint polynomial API의 φ₀…φ₄ 범위와 generic transform helper의 지원 범위를 혼동하지 않는다. 저차 joint action이 고차 반례 때문에 곧바로 실패한다고 주장하지 않는다.

## 3. 새 polynomial backend 검산

고정한 행렬은 0, −2I, diag(−1,−2), [[−2,1],[1,−2]]이며 h∈{0,1e−12,.1,1,10}이다. 서로 다른 다섯 벡터의 joint action을 Chebyshev/Laguerre로 실행해 총 40개 결과를 얻었다. oracle은 입력 binary64 값을 정확히 Decimal로 옮기고 140-digit exp 및 알려진 exact spectral projector를 썼다. production Bessel/hypergeometric/dense-exponential 구현을 재사용하지 않는다.

- 40개 모두 등록된 절대오차 목표 1e−10을 만족했다. 최대 관측오차는 4.73850883134441e−14다.
- 32개 `Certified` total bound 모두 관측오차를 감쌌다. 나머지 8개는 Laguerre recurrence의 `EstimateOnly`다.
- 동일 벡터 경로의 동일 요청 재사용은 cache hit, h 또는 degree를 바꾼 요청은 miss였다. current source fingerprint는 operator/enclosure bits를 함께 포함한다.
- nonnormal matrix는 거절했고, scalar라고 거짓 선언한 off-diagonal matrix도 거절했다. 검증되지 않은 잘못된 spectral 선언은 `EstimateOnly`였으며 `Certified`로 승격되지 않았다.
- 일반 recurrence의 unbounded 경로는 bounded 경로와 같은 output bits였고 `EstimateOnly`를 반환했다.

이 결과는 작은 symmetric toy corpus에서 implementation evidence를 준다. 모든 차원·입력·플랫폼에 대한 형식 증명, nonnormal-domain 지원, 속도 우위는 아니다. 소스 수준의 계수·tail·Green-U propagation 유도는 `MATH_REVIEW_KO.md`에 따로 썼다.

## 4. 실패가 아닌 제한과 다음 후보

아주 작은 입력 1e−300에서 현재 norm은 제곱의 subnormal 상한 때문에 4.596430877315509e−162라는 과도하게 느슨한 total bound를 돌려주었다. 1e300에서는 제곱 중간값 overflow로 명시적 거절이 일어났다. 이는 false certificate로 세지 않는다. 다만 물리량 단위 변경·amplitude scaling에 대한 실용적 가용성이 좋지 않다.

이를 겨냥한 standalone power-of-two scaled norm 후보는 [1e300,1e300], [1e−300,1e−300], 최소 subnormal, 큰/작은 혼합, 영벡터의 다섯 exact-square 검사에서 모두 상한을 유지했다. 유도와 구현 검산을 확보했으나 action 전체에 이식하지 않았으므로 1e300 입력의 전체 polynomial 경로가 해결됐다고 말할 수 없다.

역팩토리얼 후보는 normalized mantissa/exponent 상태에서 양의 정수로 반복 outward 나눗셈을 했다. p=4,30,50,170,171,172의 여섯 경우 모두 exact 1/p! 및 실제 weight-error를 감쌌다. 이식 시 nilpotent series 전체도 factorial을 만들지 않는 term recurrence로 바꾸고 모든 비교를 zero-aware로 통일해야 한다.

`budget=1e−30` 요청은 truncation budget을 만족하지만 `Certified` total bound는 약 9.16e−15였다. 이 자체는 결함이 아니다. 함수 계약이 budget을 exact-arithmetic truncation budget이라고 명시한다. 후속 consumer에는 `Certified && bound<=total_tolerance`를 검사하는 명시적 admission 함수를 추가하는 편이 안전하다. `EstimateOnly`가 같은 gate를 통과하지 않게 한다.

추가 analytic-only 연구는 `ANALYTIC_ADDENDUM_KO.md`에 있다. Laguerre의 scale β와 degree m를 함께 선택하는 tail 최소점을 유도했고 독립 검토가 확인했다. 새 수치실행은 하지 않았다. 이 최소점은 recurrence roundoff 또는 runtime의 최적점이 아니며, 현재 L≤16 cap을 근거 없이 풀지 않는다.

## 5. 개발 순서와 claim gate

구체적 계약은 `NEXT_STEPS.json`의 8개 task에 있다. 먼저 ExpBound 비교와 factorial의 soundness를 복구하고, signed-input/tolerance의 fail-closed 동작을 정리한다. 그 다음 scaled norm과 total-error admission을 넣는다. Laguerre roundoff certificate와 joint scale selection은 그 뒤 연구 과제로 둔다. 이 순서는 일반성 확대보다 기존 인증의 의미를 먼저 지킨다.

성능 비교는 certified/unbounded, cold/warm, distinct/same-vector 경로를 구분해야 한다. 현재 benchmark가 `joint_phi_action_unbounded`를 사용하므로 그 속도를 certified implementation 속도로 재해석하지 않는다. coefficient setup, 재사용 조건, block/vector product 수와 certificate 비용을 함께 기록하고, timing 통계 authority gate가 해결된 후 별도 사전등록 실험을 한다.

최종 승격은 root의 실제 독립 decision reviewer가 소유한다. 이 lane의 owner는 `CLAIMS.json`의 status와 근거만 제출하며 production-ready 또는 speedup을 자체 승인하지 않는다. 추가 동일 audit를 반복하지 않고 위 구현 task로 이동하는 것이 다음 생산적 단계다.
