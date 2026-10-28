# N06/N07: 직접 실행 가능한 비교 client

N06에는 immutable 선형 IVP owner와 polynomial endpoint trajectory client를 구현했다. N07에는 direct `lambda=1`과 `[0,0.5,1]` continuation을 동일한 입력·원래 budget·최종 certificate로 비교하는 실행 파일을 구현했다. 설계 문서에 command를 남기는 수준을 넘어 실제 Rust client, JSON receipt, 계약 시험과 NCP runner가 연결된다. 현재 과학적 승격은 **HOLD**이며 새 code의 native 검증과 NCP 측정 결과는 별도 기록한다.

실행 파일은 `crates/rodas5p-integrators/examples/reaudit_comparison.rs`다. `--list`로 고정된 전체 grid를 조회하고 `--suite polynomial --case p4 --arm laguerre --threads 1`처럼 한 source-bound 요청을 실행한다. stdout은 JSON 한 줄이다. `--budget` 변경 기능은 제공하지 않으며 알 수 없는 case/arm, 중복 인자, thread0은 거부한다. numerical refusal도 실패 시점까지의 counters와 elapsed time을 남긴다.

| suite | 고정 case | 비교 arm | 인증 범위와 budget |
|---|---|---|---|
| polynomial | p4,p16,p64 | protected,krylov,leja,laguerre,chebyshev,router | 원래 선형 ODE의 요청 endpoint4개, L2 `1e-7` |
| homotopy | h3,h8,h16 | direct,continuation | 원래 nonlinear collective ODE의 endpoint, weighted L2 `2e-5` |
| actual-k | k2,k4 | direct,continuation | all-entry represented stage output, component radius `1e-9` |

`protected`는 이 실험에서 diagonal scalar exponential reference를 뜻한다. production의 protected RODAS5P를 뜻하지 않는다. family 이름을 가진 arm은 해당 family 우선의 guarded routing이다. 실패하면 다른 family를 시도하므로 `selected_backend_per_step`와 `fallback_used`를 반드시 함께 읽는다. Laguerre 우선 요청이 Leja를 선택했다면 그 시간을 순수 Laguerre 속도라고 보고하면 안 된다. `router`는 모든 family를 실제 실행하고 결정된 Leja-first admission 우선순위를 유지하는 CompareAll arm이다.

선형 owner는 `A=diag(-4-(i mod4))`와 고정 dyadic 초기값, `t=1/32,2/32,3/32,4/32`를 소유한다. 외부 callback이 RHS나 certificate source를 바꿀 수 없다. 각 step의 action은 `phi0(hA)y`이며 다른 phi 입력은 정확히0이다. local action budget은 실행 전에 `div_down(epsilon,8)`로 고정된다. `||exp(hA)||2<=1`이므로 이전 error radius와 새 action bound를 outward addition한 값이 다음 radius를 보장한다. 실패 이후 예산을 늘리지 않고4개 endpoint의 같은 원래 budget을 계속 적용한다.

모든 선형 arm에는 공통 독립 certificate를 추가했다. 원래 IVP의 정확한 해는 `x_i(t)=x0_i exp(a_i t)`다. exponent의 interval product가 한 점인지 확인한 뒤 outward exponential, 초기값 곱, 반환 center와의 차, 제곱·합·제곱근으로 L2 radius를 만든다. 따라서 Krylov의 `converged`가 단순 diagnostic이어도 그것을 인증 권한으로 사용하지 않는다. 실제 반환 center가 원래 IVP의 interval 해 안에 있는지를 별도로 확인한다. 이 특수 closed model에서는 certificate가 endpoint당 O(n) primitive calls다. 일반 비정규 matrix나 generic nonlinear callback에 같은 권한을 부여하지 않는다.

nonlinear client는 기존 sealed verifier가 final `lambda=1`의 original-ODE endpoint를 검사한다. schedule만 바꾸고 source bits, mesh, budget, split/correction/refinement cap을 고정한다. 모든 refinement와 실패한 candidate/verification의 native receipt를 보존한다. actual-K client는 별도 suite로 두며128개 alpha/beta entry를 유지하는 H5 witness를 보고한다. 이 witness는 ODE truncation error를 인증하지 않는다. 모든 suite의 `full_curve_certified`는 false다.

| 비용 구간 | polynomial | nonlinear homotopy / actual-K |
|---|---|---|
| setup | source/operator/workspace, dense O(n²) 생성 | source/mesh와 native coarse/Jacobian/inverse/tube setup |
| candidate | 모든 선택·실패 family, 캐시·실제 products·복사 | 모든 lambda/round/refinement/RHS와 prefix |
| certificate | 각 endpoint의 독립 exp enclosure와 norm; 별도 timer | native packet/damped/H5/inverse/leakage 및 prefix; 전체 timer에 포함 |
| 실패 비용 | raw attempt receipts와 누적 work/time | typed refusal 이전의 native work/time |
| parallel 정보 |4개 sequential action step | round depth, window tasks, prefix levels; 실제 scheduler span은 null |

`elapsed_ns`는 source identity 생성부터 source/setup, candidate, certificate, output materialization까지의 solver transaction이다. process 시작, Rayon pool 생성, 최종 JSON serialization은 제외하므로 `process_end_to_end_timed=false`다. NCP runner는 subprocess 바깥에서 측정한 전체 wall time도 기록해야 한다. native counters는 해당 알고리즘의 event/selected-operation counts이며 완전한 FLOP 수가 아니다. homotopy와 actual-K의 certificate가 native 함수 안에 결합되어 있어 분리 timer가 없으면 null로 둔다. 그 비용은 전체 transaction 시간에서 빠지지 않는다. wall time을 이론적 parallel span으로 부르지 않는다.

NCP에서는 thread1/2/4/8, warmup2, paired repeat21을 사전 고정한다. 같은 `input_sha256`, 원래 budget, 요청 output, scope를 확인하고 양쪽 certificate가 모두 pass한 경우에만 시간을 비교한다. 임의 입력 변경·실패 arm 제거·budget 완화는 허용하지 않는다. RSS를 얻지 못하면 memory gate는 HOLD다. 실제 achieved bound, backend 선택, full process 비용을 함께 보고해야 한다. 전체 grid 실행은 이 구현의 계약 검증과 별도이며 이미 발표된 과거 campaign을 다시 실행하지 않는다.

현재 N06은 이 owned linear model의 endpoint trajectory까지 구현되었다. nonlinear continuous-residual owner에 polynomial candidate를 일반적으로 이식하는 작업과 nonnormal action 인증은 후속 범위로 명시한다. N07도 endpoint client의 비교를 실행 가능하게 만든 것이며 전체 곡선 보증을 추가한 것은 아니다. 상세 고정 parameter, 비용 grid, acceptance gate와 검증 상태는 같은 이름의 JSON을 기준으로 한다.

신규 code 검증: 계약 시험4개와 example build가 통과했다. 각 suite의 가장 작은 case에서2개 arm씩 총6개 smoke가 통과했으며 pair의 input hash와 원래 budget이 일치했다. h3 direct/continuation의 endpoint radius는 각각2.6097252e-8과2.5895422e-8, k2의 stage radius는 양쪽5.0536610e-11이다. 이는 unoptimized 계약 smoke이며 속도 근거로 사용하지 않는다. 원문은 `evidence/theory/COMPARISON_SMOKE.json`과 개별 stdout에 있다. 전체 NCP grid는 실행하지 않았다.
