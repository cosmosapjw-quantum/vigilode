# 원래 raw-U 단계식의 인과적 소거와 Newton 없는 보정

## 결과와 구현 위치

`vigilode-coupled`에 기존 기본 동작을 유지하는 `coupled_step_causal`,
`RawStepSession::new_causal`, `integrate_causal_observed`를 추가했다.
`CausalSchedule::Forward`는 작은 환경의 한 스레드 경로,
`BatchedPicard`는 독립 단계 RHS를 동시에 평가하는 선택 경로다.
두 경로 모두 원래 full-space residual 검사와 basis 확장을 그대로 사용한다.
적응형 wrapper는 같은 raw-U 순차 fallback과 물리 스텝 수락 정책을 유지한다.

이 변경의 직접 기여는 **현재 단계의 도함수 평가와 Newton 보정을 제거**하는
것이다. 기존 공유 basis 구성과 원래 공간의 잔차 검사에는 frozen JVP가 남는다.
작은 투영 행렬도 유지한다. 이 구현 전체를 완전 derivative-free 또는 matrix-free
알고리즘으로 분류하지 않는다. 일반 RHS-only 경로와 연결할 때에는 이 비용을
명시적으로 선택하는 adapter가 필요하다.

## 식과 인과성

정규직교 공유 basis를 Q, 기준 Jacobian을 J0, m차원 좌표를 z_i라 하자.
코드가 실제로 검사하는 raw-U 식의 투영은

\[
W_mz_i=h\gamma Q^Tf(y+Q\sum_{j<i}a_{ij}z_j)
 +\gamma\sum_{j<i}c_{ij}z_j,\qquad
W_m=I-h\gamma Q^TJ_0Q.
\]

8단계 중 i의 비선형 인수에는 z_i가 없다. W_m가 가역이면 첫 단계부터
차례로 계산해서 유일한 투영 root를 얻는다. `Forward`는 W_m를 한 번
분해하고 여덟 번 작은 선형계를 푼다. 단계 0은 기존 f0를 재사용하므로
새 RHS는 7회다. basis 확장이 없는 한 전체 후보의 RHS는
기준 1 + 단계 7 + 원래 공간 검사 8 = 16회다. 현 단계 JVP는 0회다.

실제 소스의 raw A,C 상삼각 및 대각 성분을 정확히 0인지 검사한다.
0이 아닌 값은 크기와 상관없이 `Unsupported`로 거절한다. 따라서
표현 변환 후 누출이 존재하는 K-coordinate 식을 임의로 자르지 않는다.
기존 transformed-K 연구 경로는 그대로다.

## 동시에 평가하는 Picard 경로

M의 대각 블록을 W_m, 하삼각 블록을
−(γc_ij I+hγa_ij J_m), b_i=hγQ^Tf0로 두고,

\[
r_i(z)=Q^T[f(y+Q\sum_{j<i}a_{ij}z_j)-f_0-J_0Q\sum_{j<i}a_{ij}z_j]
\]

로 정의하면 z⁺=M⁻¹(b+hγr(z))다. M⁻¹가 하삼각이고 r_i는 이전
단계에만 의존하므로 보정 함수의 i번째 성분 역시 z_<i에만 의존한다.
처음 M⁻¹b는 첫 단계가 정확하다. 귀납법으로 k번의 갱신 뒤 앞 k+1개
단계가 정확해진다. 따라서 정확 산술에서는 7번으로 모든 단계가 정해진다.
전역적인 수축 상수 <1은 이 유한 전파 명제의 가정이 아니다.

실제 구현은 최대 8번 갱신과 마지막 투영 잔차 검사를 허용한다.
수렴을 보이면 조기에 종료한다. W_m 분해를 전체 보정 동안 재사용한다.
단계별 RHS는 동일한 이전 iterate를 받아 독립적으로 평가하고, 합산 순서는
고정한다. 스레드별 완료 순서가 결과의 순서를 바꾸지 않는다.
부동소수점의 큰 중간 증폭이나 tight tolerance로 잔차가 닫히지 않으면
`RootIteration`으로 남기며 스윕 횟수만으로 성공 처리하지 않는다.

## 작업량, 저장량, 선택

직접 경로의 m×m 분해 비용은 O(m³), 단계 풀이는 O(sm²),
상태 lift/projection은 O(snm), s=8이다. 공유 basis 구성 비용은 별도다.
Picard는 p번 보정에서 p배의 단계 RHS 묶음을 지불하므로 작은 RHS와
한 스레드에서는 Forward가 합리적인 기본 후보다. Picard의 병렬 처리
가능성과 실제 elapsed speedup은 구별해 측정한다. 두 경로는 본래
반복의 현재 단계 Jacobian 블록, 재선형화, Newton 선형계 분해를 생략한다.

`Forward`는 lambda=1 목표를 직접 소거한다. 엄밀히 causal한 목표에서
homotopy의 중간 lambda들은 root를 얻기 위한 필수 계산이 아니다.
기존 general/augmented homotopy 구현은 유지하여, 이 특수한 인과적
소거의 조건을 만족하지 않는 목표에 연결한다. 이 선택은 사용자에게
고유한 목표식을 보존하며 불필요한 보정 계산을 줄인다.

## 강건성 계약

- 기존 autonomous/identity-mass/supplied-JVP 지원 조건을 유지한다.
- 입력 상태를 변경하지 않고 원래 공간 잔차가 통과한 후보만 반환한다.
- 투영 공간의 성공만으로 원래 공간의 성공을 선언하지 않는다.
- 실제 callback 진입과 실패를 세고, 연산 예산은 스케줄 전에 검사한다.
- 원래 모델 epoch가 바뀌면 후보를 거절한다.
- 적응형 경로는 수락된 상태에서만 진행하며 실패/거절 작업도 보존한다.
- raw target residual은 물리 시간 적분 오차와 별도다. embedded error 및
  조건부 root-error 정책은 기존 `RawStepSession`을 통해 계속 적용한다.
- 큰 비정규 결합의 부동소수점 증폭은 남는다. 직접 소거는 이를 숨기지
  않으며 검증 실패 시 같은 목표의 독립 fallback이 실행된다.

## 검증과 벤치마크

실제 결과와 실행 명령은 `../results/causal_*`에 저장한다. 새 계약 검사는
독립 성분별 raw-U 기준, callback 측정으로 확인한 현재 단계 JVP 부재,
스레드 수에 대한 재현성, 예산 실패, 불완전 basis 거절, 정확 Riccati 해에
대한 적응형 trajectory를 포함한다. `causal_bench`는 같은 바이너리에서
기존 residual-first와 두 새 경로를 비교하고, 모든 후보를 독립 full-space
raw-U 기준으로 채점한다. 한 스텝의 embedded 값이 1을 넘는 사례도
보존하여 algebraic root 성공을 시간 스텝 수락으로 바꾸지 않는다.

실행된 crate 회귀 검사는 66개 모두 통과했다. 작은 선형계 실제 solve 호출은 새 `small_block_solves`에 실패 진입까지 기록하고, 기존 `small_solves`는 완결된 전체 투영 solve 횟수로 유지한다. 적응형 공통 work의 `direct_solve_calls`는 실제 block 횟수를 사용한다.

작업 스케줄 진단에서 `stage_tasks_completed`는 Forward에서는 성공적으로
끝난 단계 solve, 병렬 묶음에서는 결과를 회수한 task(실패 결과 포함)의 수다.
서로 다른 스케줄의 실패 시 task 수를 동일한 과학 작업량으로 비교하지 않는다.
실제 callback 비용의 공통 비교에는 `rhs_attempted`와 `jvp_attempted`,
실제 block back-solve 진입에는 `small_block_solves`를 사용한다.

최종 고정 source `548526add83a38a996875b3bd2f3cd3357465ed6`에서 CPU 0에 고정한 단일
스레드 측정은 36/36 후보가 원래 공간의 independent raw-U 기준과
최대 4.5299e-14 이내로 일치했다. 96차원 weak nonlinear
사례의 작업은 기존 residual-first의 RHS 33/JVP 59/분해 3에서 Forward의
RHS 16/JVP 11/분해 1로 줄었다. 현재 단계 JVP 48회가 제거되고, frozen
basis 3회와 원래 공간 검사 8회는 유지된다. 이 사례의 시간비
(residual-first 시간 / Forward 시간)는 3.0446다.

12개 문제/차원 조합에서 Forward의 시간비 범위는
1.1717–5.2803, median은 2.4211다.
Picard는 0.7821–1.5947, median은
1.1743다. 시간비 1 미만의 결과도 포함한다.
5회 반복 각각은 30번의 완전한 후보 계산을 측정하며, 독립 oracle과
JSON 직렬화는 타이머 밖이다. 이 반복의 분산을 신뢰구간으로 해석하지 않는다.
원자료는 `causal_benchmark.json`, 정확한 실행 source/binary는
`causal_benchmark_receipt.json`, 집계는 `causal_benchmark_summary.json`이다.
최종 원자료의 SHA256을 집계에 기록했다.

전체 36개 방법별 행 중 embedded≤1은 6개다.
대부분의 고정 h 사례는 시간 스텝의 물리 수락 기준을 넘으므로 위 시간비의
대상은 동일 algebraic stage root다. 실제 수락된 trajectory의 성능비로
옮기지 않는다. 물리 시간 적분 검사는 별도의 adaptive Riccati 테스트가
담당하며, 동일 물리 오차의 장시간 trajectory 벤치마크는 이후 단계다.

다른 컴파일 작업과 겹쳤던 첫 진단 raw/receipt는 `*_initial.json`,
그에 대응하는 최초 집계는 `causal_benchmark_summary_preliminary.json`에
보존했다. 최초 진단에서 관측한 96차원 oscillator의 Picard 시간비 약
0.452는 그 실행의 결과이며, 최종 실행의 숫자와 혼합하지 않는다.
