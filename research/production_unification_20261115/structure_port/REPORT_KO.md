# U08 구조 인증 경계 — 2026-11-16

판정: **U08 PARTIAL**. 계약의 "주장한 기제마다 native port 하나와 원 방정식 oracle" 조건을 **quotient trace 한 기제**에 대해서만 채웠다. 나머지 기제는 `DICTIONARY.json`에 원천→대상 대응과 미이식 상태로 기록했다. allowlist에 library 파일이 없어, 인증 경계는 test 수준의 prototype이다. library 타입은 후속 작업이다.

## PORT_TRACE (L06/L07) — `crates/vigilode-coupled/tests/structure_certificate_transfer.rs`
- 인증서 필드: source id(`TRACE_TARGET_ID`), 매개변수 epoch(a, b의 정확한 bit), 유효 영역(점), 표현된 불변량(`QuarticInvariant`), 독립 oracle의 원 방정식 잔차, 재구성 사상 `q2 = a − q1²`.
- 독립 oracle: test에서 원 방정식 q1²+q2=a, q2²+q1=b로부터 소거식을 유도했다. Durand–Kerner(동시 갱신, 정체 시 종료)로 근을 구하고, 모든 근을 **두 원 방정식** 모두에 대입해 잔차 ≤1e-9를 확인했다.
- s1: 6개 매개변수 점 × 8개 불변량에서 oracle 합(중복도 포함)이 native 구간 안에 있다. native counter는 근 열거·Jacobian·JVP·AD·Newton이 모두 0이다(제거된 작업). 정확한 삼중근 점은 oracle 조건수가 나빠(동시 갱신도 반올림 잡음이 누적됨) 근처 단순근 점 (0.75, 0.7501), (0.7501, 0.75)로 비교했다. 이 조정은 test 구현 과정에서 실패를 보고 한 것이며 기록으로 남긴다.
- s2 반례: a=b=3/4에서 근 1/2이 세 번 나온다. 중복도 합 Σq1=0과 서로 다른 근의 합 −1은 다른 목표다. Σq1은 두 매개변수 점에서 같지만(0) Σq2²=4b는 다르다. 즉 저차원 불변량만으로는 횡단 좌표를 잃으므로 재구성 사상이 필요하다. (주의: ΣQ2 = 4a − Σq1² = 0으로 모든 (a, b)에서 같다. 첫 시도의 "ΣQ2=4a" 주장은 틀렸고 test 실패로 드러나 고쳤다.) 실근만의 합(a=0, b=1)은 전체 복소 합과 다르다. SelectedReal/RealOnly 의미는 평가 전에 거절되고 의미 거절로 계수된다.
- s3: 인증서가 epoch 1 ulp 변화(a 또는 b)를 거절한다. 불변량 Q1²+Q2의 합은 원 방정식 1에 의해 정확히 4a=3이며 native 구간 [3, 3]과 일치한다.

## PORT_TOPOLOGY prototype (주장하지 않음)
t1: 팔면체(구)와 3×3 삼각분할 토러스에서 D1·D0 = 0(정수 정확)이고, Betti 수는 (1,0,1)과 (1,2,1), Euler 2와 0이다. L0의 영공간은 상수(닫힌 영역 질량 보존)이고, Hodge 1-Laplacian의 영차원 = b1이다. solver backend가 아니다.

## 검사와 한계
4/4 통과(`logs/u08_green.log`), fmt·진단 clippy 통과(변경 파일 경고 0). 시간 적분을 거친 오차 전파(trace_certificate의 slab 인증)는 NOT_RUN이다. 21개 archive 핵심 member 비교도 NOT_RUN이다. 기호 itinerary, scan, homotopy, cofactor chart는 이식하지 않았다. 신규성 주장은 없다.
