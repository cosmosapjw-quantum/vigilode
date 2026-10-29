# 새 관측 정책 실험의 최종 독립 검토

실행 source `5913fe062706c820e768b9e150e5cd1c274f7bf9`, tree
`7a70caf85569dbe6c77ecb9a53fe666cd40ae618`의 결과를 검토했다. 같은 소스의
DOPRI5 landing·dense 관측 방식을 같은 host에서 비교한 실험이다. 이전 source의
실행 시간이나 새 외부 solver의 실행 시간을 섞지 않았다. 이 검토에서는 solver를
다시 실행하지 않았고, 원래 analyzer를 import하거나 그 식을 호출하지 않았다.

**판정은 제한된 범위의 감사·게시 통과다.** 현재 결과와 source에 직접 연결된 기능의
채택 근거는 충분하지만, 범용 production 사용이나 보편적인 속도 우위의 근거로
확장하지 않는다. broad production HOLD는 유지한다.

## 독립 확인

raw gzip archive를 순서대로 한 번 읽고, 별도의 Python/NumPy 해석해 계산으로 저장된
모든 상태를 재채점했다. archive 전체 해시와 각 stdout/stderr 해시, receipt와의 연결,
요청·반환 시각, 초기값, 최종 상태, RHS/FSAL 작업량도 확인했다.

| 검토 항목 | 결과 |
|---|---:|
| raw 파일 크기·해시 | 576/576 일치 |
| 독립 full-state 재채점 | 288/288 통과 |
| 정확한 configuration/phase/repeat 집합 | 288개, 누락·중복 없음 |
| 설정별 median·오차 요약 | 72/72 일치 |
| dense 관측 밀도 변경의 작업량·최종 상태 | 24/24 동일 |
| 공통 오차 상한의 최속 설정 선택·비율 | 24/24 일치 |
| 반복 사이 모든 저장 상태 및 작업량 | 72개 설정 모두 동일 |
| compiled validation log 해시 | 12/12 일치 |
| 고정 source의 실제 통과 시험 | Rust 67개, Python 7개 |

상세 결과와 재현 코드는 `evidence/FINAL_RAW_REPLAY_AUDIT.json`,
`evidence/audit_final_benchmark.py`다. compiled closure는
`evidence/FINAL_COMPILED_REVIEW.json`에 별도로 남겼다. 앞선 pending 상태의
독립 검토 receipt를 덮어쓰지 않았다. 새 receipt가 cross-zero query 회귀 시험,
zero-span 초기 physical lift 검사, 부분 관측 transaction, malformed callback 및
subnormal 증가량 회귀 시험의 실제 통과 log를 연결한다.

## 성능과 정확도의 해석

공통 오차 상한 24개 비교 중 22개에서 두 정책 모두 적격 설정이 있었다. 그중 20개는
dense의 median 시간이 짧았고, 2개는 landing이 짧았다. dense가 모든 경우에
우세하다는 결론은 틀리다.

| 남겨야 할 반대 증거 | 정확한 해석 |
|---|---|
| damped, cap=1e-8: landing/dense median 약 0.712 | 이 설정 선택에서는 dense가 느렸다. min–max 비율 범위 0.105–1.146으로 노이즈도 크다. |
| 4096 vector, cap=1e-8: 약 0.889 | tight 목표에서 dense가 느렸다. 범위 0.693–1.054는 확정적인 우열을 뒷받침하지 않는다. |
| driven, cap=1e-8: 비교 불가 | dense의 최소 관측 오차가 약 1.0634e-7이다. landing은 목표를 충족했다. |
| rapid, cap=1e-8: 비교 불가 | landing·dense의 최선 관측 오차가 각각 약 2.2826e-7, 2.4805e-7로 둘 다 미달이다. |

`WORK_PRECISION.png`와 `RHS_WORK.png`를 실제로 열어 확인했다. 첫 그림은 achieved
sampled error를 가로축으로 사용하고 loss·큰 시간 변동을 숨기지 않는다. 두 번째 그림은
같은 nominal rtol에서의 실제 RHS 작업량을 보여 준다. 이것을 같은 실제 오차에서의
작업 비교라고 읽으면 안 된다.

rtol=1e-6에서 501개 출력 시각을 요구하면 decay/logistic/driven의 landing은 RHS
3,013회를 사용한 반면 dense는 각각 55/79/85회였다. 이는 출력 시각이 단계 수를
강제하는 비용을 제거했다는 구성적 결과와 맞는다. rapid는 8,131→7,093회로 감소가
작다. 진동 자체를 해상해야 하는 작업은 보간으로 없앨 수 없다. 51개 출력의 큰 벡터는
두 차원 모두 319→103회였다.

간단한 문제에서 landing의 오차는 관측 착지 때문에 반올림 수준까지 작아진다.
따라서 dense의 이득은 동일 output 요구와 지정 오차 상한을 만족하면서 불필요하게
작은 적분 step을 줄인 결과로 해석해야 한다. 모든 설정에서 두 방법이 정확히 같은
실제 오차를 얻었다는 뜻은 아니다. 또한 이번 실험은 새 source 내부 정책 비교이므로
중복 poison 제거만의 wall-time 개선을 분리해 측정한 결과가 아니다.

관측 밀도를 5개에서 501개 또는 51개로 바꾼 dense pair 24개에서는 RHS·수락·거절·
FSAL·최종 상태 비트가 모두 같았다. 하지만 출력 array 크기와 보간 비용, 검사한 시각의
집합은 달라진다. sparse 5개 출력은 더 적은 시각을 검사하므로 공통 오차 상한 시간비
선택에서 제외했고, 그 처리는 적절하다.

## 시간·fixture 범위

process affinity는 CPU 0, 관측된 cgroup quota는 `1600000 200000`이다. 원래 runner는
문제·tolerance group과 policy 순서를 각 round에서 무작위화했다. 한 번의 warmup과
세 번의 timed 반복, 관측한 min–max 범위는 유용한 진단이지만 신뢰구간은 아니다.
전체 solver 구성·상태·workspace와 관측 결과 수집은 timer에 포함하고, schedule
생성·해석해·오차 평가·JSON serialization은 제외했다.

큰 벡터는 여전히 반복되는 대각 감쇠율의 fixture다. affine physical norm의 정확도
계약은 targeted test로 검증했으며, 이 캠페인이 full33 물리 client의 실제 생산 경로를
벤치마크한 것은 아니다. logistic·rapid는 사전에 지정한 검증 문제지만 이전 연구에서
이미 알려진 fixture이므로 새 blind 데이터라는 표현은 사용하지 않는다. NCP 실행은 없다.

## 남은 분석기 개선 두 가지

1. `analyze_bench.py` 107–124행은 gzip archive에서 무작위 receipt 순서로 파일을
   다시 읽어 반복 압축 해제가 발생한다. 이번 frozen analyzer는 정상 완료했고 수치
   결과가 잘못된 것은 아니다. 다음 하네스에서는 archive를 한 번 순회해 작은 score와
   hash만 보관하는 방식으로 바꾸면 된다. 이 감사의 stream reader가 재사용 가능한 예다.
2. 같은 파일 130–144행의 비용 자격 검사는 timed record 개수와 각 replay 통과를
   요구하지만 정확한 warmup/timed round key 집합 자체는 강제하지 않는다. 이번
   자료의 288개 key는 독립 검사로 모두 정상임을 확인했으므로 현재 비교는 유효하다.
   다음 변경에서 중복 timed key, 누락 warmup, 잘못된 config, completed/nonzero exit의
   negative test를 추가하고 해당 구성의 비용 자격을 거절해야 한다.

이 두 항목은 미래 재사용 시의 효율·방어적 검증 개선이다. 현재 정상 자료를 폐기하거나
solver 캠페인을 다시 실행해야 할 사유는 아니다. `FINAL_BENCHMARK_REVIEW.json`에
범위, 반대 증거, 지적과 다음 조치를 기계 판독 형식으로 기록했다.
