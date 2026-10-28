# 동일 관측점의 지속 세션 측정

한 호스트에서 일정별 단일 process, 방법별 31개 cyclic grouped samples를 사용했다. 표의 배수는 observed median 비율이며 독립 process 신뢰구간이 아니다. 모든 방법이 동일한 관측점과 최대 절대오차 `1e-7` 기준을 만족했다. native/one-shot/persistent의 출력·작업은 동일하다.

| 관측점 | 문제 | persistent 시간 (µs) | restart / persistent | persistent / one-shot | RHS restart → persistent |
|---:|---|---:|---:|---:|---:|
| 65 | diagonal-decay-32 | 118.413 | 2.305 | 1.010 | 1600 → 781 |
| 65 | rotation-frequency-3 | 88.697 | 1.584 | 1.065 | 2338 → 1759 |
| 65 | scalar-decay | 42.854 | 2.524 | 1.101 | 1600 → 781 |
| 1025 | diagonal-decay-32 | 972.971 | 2.751 | 1.044 | 13312 → 6151 |
| 1025 | rotation-frequency-3 | 379.828 | 3.083 | 1.167 | 13312 → 6151 |
| 1025 | scalar-decay | 370.890 | 3.073 | 1.158 | 13312 → 6151 |

관측점마다 재시작하는 호출에는 측정된 이득이 있지만, 이미 전체 구간을 한 번에 적분하는 호출은 더 빠르다. 따라서 지속 API는 재개·관측 전달 기능의 비용을 줄이는 선택으로 채택하며, 기본 one-shot의 대체 가속이라고 표현하지 않는다. 65개 관측 scalar의 persistent max/min 비는 약2.02다. 원시 samples와 각 방법의 dispersion은 `../evidence/runs/session-65.stdout` 및 `session-1025.stdout`에 남아 있다.

이진 검색만의 전후 timing ablation은 수행하지 않았다. 검색 복잡도 정리와 지속 실행 전체 비용 측정은 구별한다. 새 외부 솔버 비교는 수행하지 않았다.
