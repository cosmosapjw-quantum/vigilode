# Adapter/switcher: 정확한 복귀 판정과 불리한 선택의 원인

## 구현 결과와 적용 결정

PR91 이후 새 PR이 없다는 부모 조사에 따라 PR91의 구현·원자료를 기준으로
연구를 이어갔다. 다음 두 옵션을 구현했으며 둘 다 기본값은false다.

* `strict_return_work_improvement`: 같은 checkpoint와 끝 시각의 실제
  작업량을 checked u128로 비교한다. 완료된 explicit 후보의 비용E가
  implicit 비용I보다 작을 때만 복귀한다. 미완료 implicit은 endpoint
  자격이 아닌 이미 청구된 비용 하한만 제공한다.
* `reuse_observed_entry_step`: 비용에서 졌거나 미완료였던 nonfatal
  entry pilot도 accepted prefix가 있으면 그 평균 step을 다음 pilot의
  초기 제안에 사용한다. 이전 상태는 재사용하지 않으며 새로운 checkpoint,
  기존 tolerance, native 수용 검사, 정수 비용 판정을 그대로 거친다.

설정의 정확한 보존을 위해 `DerivativeFallback`, `EntryProbeAdaptation`,
`SwitchingConfig`에 Deserialize를 추가했고 두 구조체의 미지 필드는 거부한다.
새 직렬화 필드를 누락한 과거 설정을 암묵적으로 바꾸지 않는다.

**Production의 일반 기본 경로는 persistent RHS-only로 유지한다.** 새
observed-step 옵션은 fixture에 따라 작업량을 줄였지만, 아래 성능 분석에서는
일반적인 실행 시간 이득을 확인하지 못했다. 선택적 JVP는 문제별 자격 확인을
거친 opt-in 경로로 제공한다. 정확한 비용 판정과 callback 중단 경계는 코드
계약으로 사용할 수 있고, 미래 구간의 비용 예측은 추가 이론 과제다.

## 임의 margin을 대체한 수학적 경계

원래 작업량을 그대로 사용한다.

\[
W=N_f+N_{Jv}+N_{f_t}+nN_J+N_{\mathrm{solve}}.
\]

같은 구간의 완료된 후보끼리는 E<I가 strict 감소의 정확한 조건이다.
동률은 incumbent를 유지한다. Implicit의 nonfatal 미완료 prefix비용L이
있으면 E<L≤I로 완료 explicit의 우세가 성립한다. 따라서 완료 explicit의
비용이E일 때 implicit comparator의 callback cap은 정확히E다. Guard가
crossing callback을 먼저 과금하고 L_cb>E에서 멈추므로 추가 margin이나
부동소수점 rounding slack이 필요하지 않다.

생산적 sentinel은 implicit window를 먼저 완료해 비용I를 얻는다. Explicit
challenger의 cap은I−1이고, 이후 완료 비용E<I인 경우만 채택한다. Native
DOPRI 한 attempt는 최대7회의 RHS를 사용하므로 I≥1에서

\[
I+E\le 2I+6.
\]

이 경계는 현재 implicit window를 기준으로 한 탐색 비용 상한이다. Implicit
자체가 비싼 regime에서 더 싼 RHS-only 전체 궤적에 대한 경쟁비를 뜻하지 않는다.
전체 증명과 실제 callsite 대응은 `THEORY.md`의R1–R4에 있다.

## 먼저 보존한 음성 결과: 새 정확한 복귀만으로는 충분하지 않았다

최초 고정20-arm 실험은 모든 정확도·회계 검사를 통과했지만, 선택적 경로의
전환 횟수가 모두0이었다. Nonlinear clock-augmented 문제에서 implicit의
작은 Krylov 풀이 비용이 늘어, 완료 pilot204가 explicit192보다 비쌌다.
이전 pilot이 이미 평균0.00252의 accepted step을 얻어도 다음 pilot은
explicit가 제안한 약0.00032에서 시작했다. 따라서 복귀 정책을 바꾸기 전에
implicit 진입 자체가 이루어지지 않았다.

이 결과와 초기 branch-coverage 테스트2개의 실패를 `PASS0_RECEIPT.json`과
`raw/pass0_*`에 보존했다. 테스트가 기대한 sentinel이 실행되지 않은 것이
원인이며, 수치해의 실패로 바꾸어 설명하지 않는다.

수정1에서 accepted-step 제안 전달을 추가하고16개 진단 arm을 실행했다.
작은 점검 과정에서 기존 private memory fixture에 새 필드가 빠져 lib test
컴파일이 실패했다. 수정2에서 해당 초기값None을 보완했고 native runtime
알고리즘은 바꾸지 않았다. 마지막 production CI가 지적한 두
`collapsible_if` 항목은 수정3에서 같은 평가 순서의 let-chain으로 합쳤다.
관측 step 계산은 두 선행 조건이 참일 때만 한 번 호출하고, 설정 검증의
각 OR 조건도 원래 순서대로 평가한다. 정책·상수·fixture는 바꾸지 않았다.
이 lint 정리까지 보수적으로 포함해 **수정 횟수는3/3회**다.

## 전체 궤적 작업량과 정확도

문제는 n16,[0,1],rtol1e-7,atol1e-10,h0=1e-4,
max_steps100000,8관측 window다. 모든 arm의 모델에는 supplied JVP만
있으며 RHS-only는 JVP0이다. FD, dense Jacobian, f_t 호출은0이다.
모든 failing/discarded pilot와 return probe를 합산했다.

| 문제 | RHS-only | PR91 | observed step + 기존 복귀 | observed step + 정확한 sentinel |
|---|---:|---:|---:|---:|
| 쉬운 cubic |79|79|79|79|
| 강성 cubic |2347|2958|2634|2634|
| 강성→쉬움 nonlinear |10375|11407|16331|12195|
| 강성→쉬움→강성 nonlinear |17113|18600|27546|24440|

강성 cubic의 PR91 대비 계산량은**10.953% 감소**했다. Relaxing/reentrant
문제에서 sentinel은 observed-step+기존 복귀보다 각각**25.326%/11.275%**
줄였지만, RHS-only보다 각각**17.542%/42.815%** 비싸다. Exact ordinary와
기존 ordinary, exact sentinel과.8 sentinel은 이 cohort에서 같은 실제
결정을 내렸다. 따라서 이 수치를 임의 margin 제거만의 추가 속도 개선으로
귀속하지 않는다. Margin 경계의 차이는 정수 ±1 및2^53 이상 테스트에서
구별되며, 현재 물리 cohort의 효과와 구분한다.

36/36 arm은 원래 cap을 통과했다. Cubic은1e-6, 기존 oscillatory return
계열은1e-5 cap이다. 강성 cubic의 최대 오류는 약4.8315e-10, relaxing은
6.5337e-8, reentrant의 새 selective 경로는6.1365e-8이다. Native counter와
실제 callback atomics가 모두 일치했다.

독립 Decimal90 계산은 **17440개 반환 상태 성분**을 정확해와 비교했다.
보조 분석은 표준 라이브러리만 사용하며, Machin공식의π와 Taylor sin/cos,
Decimal exp/log/sqrt를 사용한다. 이 계산은 고정밀 수치 검산이고 interval
인증과 별도로 기록한다. Exact Fraction Laurent polynomial 연산으로
비선형 exact solution 변환과 analytic JVP의 chain rule 항등식도 확인했다.
실제44개 pilot에서 초기 제안이 과거 accepted mean과 자신의 checkpoint
proposal에서만 구성되었는지 bit 단위로 검산했다.

## Profiling: 작업량 감소와 실제 시간은 달랐다

마지막 lint 정리 이전의 같은 측정 binary에서5개의ABBA/BAAB paired block을
사용했다. 이 binary의 SHA-256은
`8671afe3561e0b658c857ca7fe4ee5f892781fcadac69b2d491c21c595680702`이고,
당시 switching 소스는
`ebea1dd7b76da45def13f6d9e1137f693786f23308f2e0cc589debc82b658347`이다.
두 identity와 원자료를 보존했으며 최종 코드의 새 시간 측정으로 표기하지 않는다.
강성 cubic은
process당64회, reentrant는16회의 전체 solve를 반복했다. Model/result할당과
native 계산이 timer에 포함되고 정확해·JSON 출력은 이후다. Callgrind는4회와
12회의 instruction 차이를8로 나누어 startup·JSON 고정 비용을 제거했다.

| 비교 | 작업량 변화 | trajectory당 명령 수 | paired 시간 중앙 speedup [관측 범위] |
|---|---:|---:|---:|
| 강성 cubic PR91→observed |−10.95%|7,799,712.6→7,839,131.3 (+0.505%)|1.0106 [0.8981,1.4469]|
| Reentrant RHS-only→observed strict sentinel |+42.81%|39,519,807.5→96,831,168.8 (+145.02%)|0.3404 [0.1265,0.5046]|

강성 cubic의 wall time은 안정적인 개선을 확인하지 못했다. Reentrant의
새 selective 경로는 모든 paired block에서 더 느렸다. 범위는 관측된5개의
block 범위이며 신뢰구간으로 표현하지 않는다.

Adverse12회 profile에서 `drive_windows`의 self instruction은전체의
약0.0225%다. 반면 `safe_l2`9.77%, pivoted QR3.72%, two-pass MGS3.53%,
GMRES2.63%가 관측되었다. Self attribution만으로 호출한 모든 하위 작업을
분리했다고 주장하지 않으며, 현재 증거는 다음 두 원인을 구별한다.

1. **Adapter 선택/거주 비용:** 짧은 prefix의 strict 승리를 미래 regime에
   이어 쓰면서, 비싼 implicit window를 여러 개 실행한다.
2. **개별 implicit mode 비용:** 단위 callback proxy가 작은 Krylov norm,
   orthogonalization, pivoted QR와 workspace 작업을 충분히 반영하지 못한다.

원자료에는 설치된 Valgrind engine 경로를 잘못 지정한 첫 profiler 실행
실패도 보존했다. Engine이 실행되지 않은 환경 실패이며, 올바른libexec경로로
교정하면서 이미 끝난 timing을 재사용했다. 별도로 mpmath/SymPy가 없었던
분석 환경 실패를 보존하고 표준 라이브러리 검산으로 완료했다.

## Native 검증과 production 인계

서로 다른 native 계약**35개PASS**: switching lib16, comparator3,
entry/checkpoint5, 기존 sentinel7, 새 strict-return4다. 새 계약은 정확한
정수 동률/한 단위 차이, overflow·endpoint 자격, 평균 step의 유효성,
직렬화 미지 필드 거부, Forbidden의 JVP0, sentinel 비용 상한,
fatal challenger 이후 productive implicit prefix 보존을 포함한다.

수정3 이후 같은 35개 계약이 소스가 고정된 각 실행에서 다시 통과했다.
최종 36-arm 실행도 모두 PASS이며, 이전 자료의 시간 측정값을 제외한 모든
필드가 정확히 일치했다. 여기에는 **17440개 상태 성분**, 반환 시각,
모든 segment의 결정·counter·작업량이 포함된다. 최초20-arm 설정에서
없었던 observed-step 필드만 당시의 실제 값false로 정규화했다.
따라서 기존 Decimal90와44개 seed 검산을 동일한 입력에 대한 근거로
재사용한다. 새 독립 고정밀 검산을 추가 실행했다고 세지 않는다.
최종 benchmark 실행 파일의 SHA-256은
`c8462ad3b7f83b683a8d0ab01bfe1a361ccd4367375611f1046a9f7b8dc53e62`이다.

공동 CI에서 다른 example의 lint 정리가 겹친 두 실행은 native 종료값0에도
recorder가 `stale_source`로 올바르게 거부했다. 이 기록을 지우지 않고 소스
고정 후 해당 명령을 재검증했다. `PASS3_RECEIPT.json`과 `FINAL_AUDIT.json`에
그 구분, 최종 identity, 비교 범위가 있다. 알고리즘의 네 번째 수정이나
새로운 paired 시간·profiling 실험은 수행하지 않았다.

원자료 및 재현 명령은 `EXECUTION.json`, `RESULTS.json`, `PERFORMANCE.json`,
`run_performance.py`, `analyze_results.py`에 연결했다. 현재 관측에서 승리한
work proxy를 CPU speedup으로 바꾸어 보고하지 않는다. 추가 local PR은
`LOCAL_TASKS.json`에 수학·구현·성능 완료 기준을 각각 담았다.

## PhysMath R1–R10 / C1–C10 수행

| 단계 | 실제 산출물/근거 |
|---|---|
| R1/C1 | PR91 baseline와 fixed nonlinear 정확도·resource 계약,PROTOCOL.md |
| R2/C2 | PR91 exact entry, PR90 callback lower bound, native return/sentinel/fatal 경로 재사용 |
| R3/C3 | 완료된 cost와 incomplete prefix의 자격 분리; checked integer boundary와 causal proposal 계약 |
| R4/C4 | Ordinary exact return, productive sentinel, whole-session transfer의 병렬 대안 비교; 공동 build/profile lock |
| R5/C5 | THEORY.md R1–R4와 실제 flags/helpers/callsites/Deserialize 구현 |
| R6/C6 | 35 native 계약, exact Fraction chain rule, 초기 실패 원인 교정, 세 번째 CI lint 정리 |
| R7/C7 | 최초20+진단16 nonlinear arm, 모든 반환 성분 정확해 검사 |
| R8/C8 | 17440 Decimal90 상태,44seed검사,5pairedblocks×2비교,8Callgrind profile |
| R9/C9 | 실제 불리한 결과와 mode/adapter 원인 분리; 부모의 별도 diff/증명 검토에 제공 |
| R10/C10 | 기본RHS-only·선택적JVP opt-in 결정, 실행 identity와 local PR 계약 보존 |

R9/C9에서 부모 agent가 실제 diff와 R1–R4를 별도로 검토했고 현재 범위의
새 correctness finding을 발견하지 않았다. 해당 source hash와 판단은
`../theory/ADAPTER_REVIEW.json`에 있다. 이 검토는 세 번째 lint 정리 전
알고리즘 소스에 대한 것이다. 최종 두 분기의 동치와 실행 근거는
`raw/PASS3_SOURCE_EQUIVALENCE.md`와 `PASS3_RECEIPT.json`에 따로 기록했다.
같은 모델이 실행한 검산을 서로 다른 모델의 검토로 세지 않는다.
