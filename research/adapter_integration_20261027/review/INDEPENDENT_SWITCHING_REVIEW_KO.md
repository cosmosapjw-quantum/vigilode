# Switching 통합의 독립 소스 리뷰

검토 일자: 2026-10-27. 구현 담당과 별도의 에이전트가 native `e8572eac` 및 joint `9b062e81` 양쪽과 현재 통합 소스를 비교했다. 검토한 7개 파일의 SHA256과 세부 항목은 [INDEPENDENT_SWITCHING_REVIEW.json](INDEPENDENT_SWITCHING_REVIEW.json)에 있다. 이 검토에서는 컴파일·solver 실행을 하지 않았으며 실제 실행 확인은 상위 작업의 validation 기록으로 판정한다.

## 결론

이번 소스 검토에서 새로 도입된 상태 채택·fatal callback 처리·공유 attempt 예산의 중대 결함은 발견하지 못했다. 같은 구간에서 실제로 더 유리한 implicit 비교 결과를 그대로 채택하는 구현은 중복 implicit 계산을 제거하면서 기존 체크포인트 계약을 유지한다. 다음 세 항목은 문서 및 후속 시험에 반영할 가치가 있다.

1. **과거 작업률에 의한 false negative는 남는다.** adaptive 복귀는 먼저 이전 implicit 평균률을 이용해 explicit 시험을 제한하고, 이를 통과한 경우에만 동시간 구간 comparator를 실행한다. 구간 길이 1, 이전 implicit 비용 10, 현재 explicit 비용 20, 현재 implicit 비용 100, 비율 0.8이면 실제 같은 구간에서는 explicit이 유리하지만 이전 기준은 시험을 거절한다. 따라서 개선은 같은 구간 비교 없는 잘못된 복귀를 막지만 모든 유리한 복귀를 찾는 것은 아니다. “Neither bound changes a decision” 문장은 기존 선별 규칙에 대한 주장으로 범위를 한정해야 한다. 미래에는 제한된 빈도의 paired sentinel을 역사적 비용 상한 없이 실행하고, 탐색 비용을 포함한 이익을 확인할 수 있다.

2. **생산적인 comparator의 집계 테스트를 보강할 수 있다.** 새 private contract는 attempts와 RHS 합을 확인한다. 동일 fixture에서 accepted/rejected, Jacobian, JVP, df/dt, linear solves도 segment 합과 일치하는지 단언하면 완전 비용 청구에 대한 테스트 범위가 명확해진다. 공통 `charge()` 구현을 읽은 결과 해당 필드의 누락은 보이지 않았다.

3. **probe와 낭비를 같은 뜻으로 집계하지 않아야 한다.** 새 implicit comparator는 `is_probe=true, committed=true`일 수 있다. 이 비용은 생산적 계산이다. 성공한 advisory entry pilot, 실제 실패한 trial, 생산적으로 채택한 comparator를 분석기에서 구분해야 한다.

## 확인한 동작

- explicit 후보와 implicit comparator는 동일한 `start`, `target`, 체크포인트 상태에서 시작한다. comparator 비용을 먼저 청구하며 successful replay를 채택할 때 다시 청구하지 않는다.
- productive replay가 채택되면 시간·상태·implicit 작업률·평균 스텝 seed·거주 창 수가 함께 갱신된다. discarded explicit 결과는 채택되지 않는다.
- fatal comparator는 두 trial의 비용을 남기고 이전 체크포인트를 보존하며 중단한다. 다른 방법으로 fatal callback을 재시도하지 않는다.
- 모든 trial은 남은 전역 attempt 예산으로 잘린다. comparator 예산이 없거나 불완전한 경우 explicit 복귀 성공을 꾸미지 않는다.
- 적응형 entry memory와 native legacy entry backoff를 동시에 적용하지 않는다. `entry_probe_adaptation=None`일 때 native 정책을 유지한다.
- native의 callback 상한, implicit coalescing, 평균 스텝 seed, 명시적 session snapshot, 정적 결정 문자열을 유지한다.
- 짧은 explicit tail 수정은 유효한 초기 제안을 미리 잘라서 무효 입력으로 만드는 문제를 없앤다. 사용자 입력은 여전히 맨 앞의 `validate()`에서 검증된다.
- 유한차분·제공된 미분의 권한을 추가하지 않는다. Forbidden 경로는 경쟁을 우회한다.

검토의 판정은 소스 수준 통합에 한정된다. 기존 공개 실험의 wall-time drift, 정확도 실패 셀, method별 실제 비용 가중치와 탐색 비용은 별도 성능·정확도 검증 항목으로 남는다.

