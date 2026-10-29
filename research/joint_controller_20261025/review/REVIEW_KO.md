# 독립 검토 기록

구현을 작성하지 않은 별도 검토자가 최신 기준 commit `4fe67602502440d76aa9dd245c6b1d67558c14bf` 이후의 방법 선택, 실행 선택, 인과적 단계 오차 예산을 검토했다. 검토 대상은 새 opt-in 경로와 그 수학·pilot의 대응이다. 전체 범용 solver의 production 승격은 이 검토의 결론에 포함하지 않는다.

## 확인한 수학과 코드 연결

엄격한 하삼각 전파의 유한 경로합은 수축률이 1보다 작지 않아도 성립한다. 전진 대입은 단계 오차를, 역방향 adjoint 대입은 원래 출력에 대한 잔차의 영향을 계산한다. 구현의 비음수 입력 검사와 outward 연산은 이 조건부 추정을 보존한다. 입력 잔차·역원·Lipschitz 상계와 적용 영역의 타당성은 모델 제공자의 계약으로 남는다. `lambda=1`, 실제 represented step, accepted state 및 provider/norm/chart 버전이 일치해야 한다.

비용 비교 함수는 단위·정확도 자격·동일한 물리 범위를 먼저 검사한다. 미래 전환 비용을 분할하고 후보 비용을 위쪽으로, 현행 비용의 허용 문턱을 아래쪽으로 반올림한다. 이 충분조건의 수학적 보장은 실제 비용 구간이 유효할 때 적용된다. 실제 세 번의 시간 표본은 경험적 envelope로 명시되어 있으며, 코드가 이를 미래 실행시간의 확률적 보장으로 바꾸지는 않는다.

새 방법 선택은 최근 작업량과 step 제안·거절 추세를 함께 사용해 오래된 음성 비교를 만료시킨다. 작은 step 자체는 강성 판정으로 사용하지 않는다. 기존 lifetime 횟수 제한이 끝난 뒤에도 적격 pressure batch에서 재탐색할 수 있으며, 모든 시도는 기존 전체 attempt 한도에 합산된다. 명시법으로 복귀하면 이 기억을 초기화한다.

실행 adapter는 변경된 물리 identity의 비교 자료를 되살리지 않고, nested 호출을 외부 calibration에 섞지 않는다. 실제 dispatch의 비용이 반복적으로 이전 envelope를 벗어나면 다음 선택부터 자료를 만료시킨다. 이미 계산한 결과를 몰래 다시 실행하지 않는다.

## 수정한 지적

1. **결합 오차 예산:** 초기 causal pilot은 `max(|b_i|,|b_i-bhat_i|)` 가중치로 principal 출력과 embedded 차분의 교란을 각각 상계했다. 이 상계 하나만 `|embedded_candidate|`에 더하면 두 교란의 합을 모두 예약하지 못한다. 구현을 outward `|b_i|+|b_i-bhat_i|`와 두 reconstruction 반올림 오차의 합으로 고쳤다. 일반 `assess_causal_budget` 함수 자체의 오류는 아니다.
2. **pilot의 interval 일관성:** domain closure의 행 합을 directed 연산으로 계산하고, embedded 계수 차분의 반올림을 interval reconstruction에 포함했다.
3. **보장 범위:** sentinel의 최대 간격은 다음 적격 pressure batch에서 재탐색할 자격을 복구한다. physical-time 탐지 지연에 대한 무조건적 상한으로 표현하지 않도록 고쳤다. 별도로 유도한 prepaid 탐색비 credit 상계는 현재 구현된 기능으로 기록하지 않는다.

4. **복귀 비교의 범위:** 초기 변경에는 implicit에서 explicit으로 복귀할 때 직전 window의 비용률을 실제 결정에 사용하는 경로가 남아 있었다. 새 opt-in 경로에서는 과거 비용률을 탐색 trigger로만 쓰고, 같은 checkpoint와 같은 window의 RODAS replay를 실제 비교에 사용하도록 고쳤다. 성공한 RODAS 비교가 더 유리하면 그 결과를 생산적 진전으로 채택한다. 치명적 실패·비교 예산 소진은 기존 committed state를 보존하며 모든 시도를 합산한다. 새 복귀 계약시험과 최종 façade 회귀시험이 통과했다.

5. **비교 자료의 시간적 위치:** 첫 실제 case5 pilot은 새 정책 2,463 RHS로 기존 정책 1,887 RHS보다 비쌌다. 초기 과도구간의 불리한 비교가 과도성분 소멸 뒤까지 재사용되었다. 작업률 감소·제안 스텝 증가를 함께 감지한 첫 교정도 방금 종료된 과도구간을 재비교하여 2,763 RHS로 악화했다. 최종 교정은 수락 endpoint에서 비교를 예약하고, 다음 적격 prefix를 새 checkpoint부터 비교한다. 예약은 비교 자격이 없었던 batch에서 소모되지 않는다. 같은 임계값과 정확도 목표를 유지한 실제 계약시험 10개 및 library 시험 17개가 모두 통과했다. 첫 두 결과는 그대로 보존했다. 최종 실제 pilot에서 같은 case5의 작업량은 1,311 RHS로 줄었고, 모든 반복의 endpoint 오차가 같은 사전 선언 상한을 만족했다. 이 경우 wall-time 중앙값은 기존 369.166 μs, 새 정책 369.637 μs로 사실상 같으므로 작업량 감소를 실행시간 개선으로 바꾸어 해석하지 않는다.

## 남은 개발상 한계

- 방법 선택의 작업량 점수는 제공자별 wall-clock 비용을 보장하지 않는다. 원래 물리 오차를 만족한 동일 구간의 비용 계측과 전환 뒤의 RODAS 세션 유지가 다음 대상이다.
- scalar causal pilot은 병렬 Jacobi round를 순차적으로 모사한다. 내부 풀이 정밀도의 과잉 지출을 줄이는 실험이며 병렬 속도 향상의 측정으로 사용할 수 없다.
- callback·chart·norm의 숨겨진 변경은 caller가 epoch를 바꿔야 감지할 수 있다. timing drift는 변경된 첫 실행을 미리 예측하지 못한다.

## 독립 실행 검산

`independent_probe.rs`는 실제 native 함수의 binary64 입력·출력을 내보내고, `check_probe.py`는 별도의 `Fraction` 정확 유리수 연산으로 전진·adjoint 상계와 비용 선택 충분조건을 검사한다. 96개의 지수 범위가 다른 단계 문제와 256개의 비용 경계·명확한 이득 문제를 실제 실행했다. 994개 정확 유리수 검사가 통과했으며, 그중 native 함수가 후보를 선택한 34개 모두 정확한 충분부등식을 만족했다. 최초 경계 corpus는 후보 선택이 0개였으므로 이를 보존하고 명확한 이득 사례를 추가하여 비어 있는 검증을 피했다. 실행 코드·rlib·바이너리·원시 행의 해시는 `INDEPENDENT_EXECUTION.json` 및 `EXACT_ORACLE_CHECK.json`에 있다.

최종 판정은 변경된 소스·독립 상계·비용 산술과 제한된 native pilot에 대한 **pass_scoped**이다. 최종 façade 검증은 29개 suite의 234개 시험이 통과했고, 실행 중 소스 변경은 없었다. 방법 선택 pilot의 9개 사례에서 warmup을 포함한 162개 실제 실행이 모두 목표 시각·유한 상태·선언된 endpoint 오차 상한을 만족했다. 현재 소스 해시가 해당 검증 manifest와 일치함도 별도로 확인했다. 수정된 causal pilot의 실제 실행도 확인했다. 같은 256-step mesh에서 RHS와 interval verifier 평가 합은 예산 기반 10,513회, blanket polishing 16,818회였다. 예산 기반 경로의 해석적 endpoint 오차는 약 2.03e-9, 최대 국소 단계 교란 상계는 9.85e-10으로 1e-9 예산 안에 있었고 fallback은 0회였다. 순차 native stage oracle은 1,792 RHS이므로 이 결과는 scalar 문제에서 병렬 후보를 자동 선택할 근거가 되지 않는다.

최종 방법 선택 실험은 비교 비용과 처리량의 차이도 드러냈다. 완화 문제에서는 더 강한 동일-window 복귀 비교로 RHS가 749에서 813으로 늘었다. 회전에는 직접 DOPRI가, case5에는 직접 RODAS가 여전히 유리하다. 새 전이 위치 0.43·0.87은 같은 문제군 내부의 이동 검증이며, 0.47·0.91은 회귀 대조군이다. 이 관측을 일반 문제군의 오탐률이나 모든 solver에 대한 우월성으로 환산하지 않는다.
