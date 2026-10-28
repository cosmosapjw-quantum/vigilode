# N03/N04/N05 bounded client benchmark

`reaudit_bound_clients`는 새 수학·정책 API를 실제 실행하는 별도 example이다. `--case CASE --arm ARM --threads N`으로 고정 요청 하나를 실행하며 `--list`로 범위를 조회한다. source identity와 변경할 수 없는 budget을 JSON으로 내보낸다. 모든 arm의 전체 process 비용은 NCP runner가 별도로 기록하며 내부 `elapsed_ns`는 solver transaction이다.

| case | arm | 실제 작업 | 권한 |
|---|---|---|---|
| diagonal | bounded / resolvent / diagonal | 동일32차원 diagonal IVP를 실제 GMRES lagged RODAS5P driver로 적분하고 독립 exp oracle로 요청 endpoint 검증 | endpoint L2 `1e-7`; full curve 아님 |
| global | provider / replay | 동일한 sealed PR101(-16,1)와 curve에서 provider만 또는 추가 global replay를 실행 | 원래 absolute L2 `1e-4`의 전체 curve와 저장된 query center |
| allocation | uniform / optimized | 동일 A=[1,9],c=[1,1],p=1,budget=0.5에서 integer counts를 outward 검증 | algebraic inequality와 model cost ceiling; ODE 권한 없음 |

N05의 diagonal 비교는 예측된 q의 숫자만 비교하지 않는다. 실제 driver가 같은 RHS, Jacobian, 초기값, tolerance, output mesh와 step cap으로 실행된다. `work.lagged`, `work.staleness`, solver counters와 driver policy ID를 보존한다. 고정 fixture에서는 fallback0과 `Diagonal.resolvent_solves=0`을 필수 witness로 둔다. Resolvent arm은 실제 inverse RHS solves가 양수여야 하며 Diagonal arm은 실제 structure check를 수행해야 한다. 모두 요청 output을 original-IVP outward oracle로 검사한다. 이 fixture 밖의 일반 production 속도는 별도 검증 대상이다.

같은 frozen matrix의 `BoundLinearStageModel`과 raw residual target planner도 setup 구간에서 실제 실행한다. 다만 기존 generic lagged driver의 diagnostic stage targets를 자동으로 교체하지 않았으며 JSON에 `stage_policy.used_by_driver=false`를 명시한다. 이 비교는 새 diagonal staleness 구현의 효과와 planner 준비 비용을 측정한다. theorem target을 그 driver에 이미 이식했다고 주장하지 않는다.

N03은 실제 sealed model run으로 source curve를 만든 후 `certify_global_run`을 실행한다. source run의 private trajectory identity를 certificate가 가리키는지 확인하며 초기 ball, curve residual, gain, jump, 저장된 center의 roundoff를 모두 포함한다. provider와 replay의 solver source·queries·budgets는 같고 replay의 추가 비용과 work를 따로 보고한다. 양쪽 solver outputs가 동일한지를 계약 시험에서 확인한다. PR101의 기존 model/provider 권한만 사용하며 임의 callback에는 full-curve 권한을 주지 않는다.

N04의40→32는 실제 ODE step을 실행한 속도 수치가 아니다. uniform[20,20]과 이론적으로 구성한[8,24]의 같은 error budget에 대한 cost ceiling이다. optimized arm은 일반 heuristic allocator도 실제 실행하고 그 결과를 별도 `heuristic_plan`에 기록하되 exact witness와 혼동하지 않는다. `actual_ode_steps_executed=0`과 `ALGEBRAIC_ALLOCATION_ONLY`를 항상 남긴다.

고정 CLI와 private model owner가 입력 변경을 차단하며 모든 refusal은 JSON에 남긴다. `full_curve_certified=true`는 global case의 sealed complete-run certificate가 성공한 경우에만 설정한다. diagonal과 allocation은 false다. endpoint, full curve, algebraic claim을 한 speed gate로 합치지 않는다. NCP의2 warmup·21 paired repeats 및 thread1/2/4/8을 사용하며 이미 발표된 과거 scientific campaign은 재실행하지 않는다.

신규 native 검증은4개 계약 시험, example build,7개 arm smoke 모두 PASS다. 세 diagonal arm의 독립 endpoint radius는 동일한3.133308233243001e-15이며 fallback은 전부0이었다. Resolvent의 추가 RHS solves32개를 Diagonal은0개로 줄이고15개 structure check/15,360개 entry 검사를 기록했다. Bounded도 이 단순 fixture에서는 추가 RHS solve가0개이므로 Diagonal이 모든 정책보다 빠르다고 결론 내리지 않는다.

PR101 provider/replay는 같은 curve와 output을 유지하면서 각각8.391931330096254e-12,9.494606810899762e-12의 full-curve/query bound를 보였다. 양쪽 모두 원래1e-4 budget 안에 있다. G2는 같은 정확한0.5 error bound에서40과32의 model work upper를 반환했다. 이 자료는 unoptimized 신규 계약 smoke이며 wall-time speed promotion은 하지 않는다. 원문은 `evidence/theory/BOUND_CLIENT_SMOKE.json`과 개별 stdout에 있다.
