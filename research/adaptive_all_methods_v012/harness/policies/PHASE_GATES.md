# Phase gates

- 현재 phase의 completion bar를 충족하기 전 다음 phase로 넘어가지 않는다.
- candidate producer는 최종 promote 결정을 하지 않는다.
- evidence audit에서 `UNSUPPORTED` 또는 `MISATTRIBUTED`인 claim을 사실 전제로 사용하지 않는다.
- fatal validation issue가 있으면 formalization으로 승격하지 않는다.
- promote되지 않은 hypothesis를 확정된 결과처럼 서술하지 않는다.
- phase 종료 때 `RESEARCH_STATE`, 필요한 ledger, `DECISION_LOG`를 갱신한다.
