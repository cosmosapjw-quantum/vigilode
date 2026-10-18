# DAG continuation 정책과 이전 중단의 원인 분석

## 왜 이전 continuation이 멈췄는가

이전 턴은 bounded implementation 단위의 코드·테스트·독립 검토·ledger·push를 마친 뒤, 결과 보고를 전체 작업의 종료처럼 취급했다. 사용자가 요청한 목표는 production-audit DAG 전체의 완료였는데, 실행자는 이를 “현재 가능한 scoped unit을 완료하고 다음 HOLD를 보고”로 축소했다.

구체적 오류는 다음과 같다.

1. **범위 축소:** scoped PASS를 해당 acceptance 전체의 전진으로 기록했지만, 해당 하위 범위와 전체 노드의 acceptance criteria를 계속 대조하지 않았다.
2. **HOLD 오판:** A01/A04/A05/A07의 HOLD는 수학·산술·코드 수정으로 해결할 연구 과제였는데, 외부 immutable input 부재처럼 후속 실행을 막는 상태로 취급했다.
3. **스케줄러 부재:** 결과 commit/push 뒤 live DAG를 다시 계산하고 다음 READY 노드를 실행하는 continuation 루프를 완료 조건으로 만들지 않았다.
4. **보고와 달성 혼동:** 보고서에 정확한 다음 실행점을 적은 사실을 실제 다음 실행과 혼동했다. “다음에 할 일”은 실행 증거가 아니다.
5. **DAG 표현 결함:** A01의 outward proof는 조건부로 이미 여러 항을 닫았지만 DAG에는 남은 certificate/work/size 의무가 구체적으로 분해되지 않았다. 이 모호성이 과도한 HOLD를 정당화하는 듯 보이게 했다.

이는 과학적 실패나 사용자의 입력 부족이 아니라 작업 관리/연속성 실패다. 이전 source와 evidence는 보존한다. 이 문서는 그 실패를 면책하거나 과거 scoped PASS를 broader PASS로 재해석하지 않는다.

## 필수 실행 정책

사용자가 DAG 전체 continuation을 요청하면:

1. 원래 objective와 모든 required-node acceptance criteria를 종료 시점마다 점검한다. scoped PASS는 그 scoped contract만 닫는다.
2. 각 bounded unit 이후 HEAD, remote ref, worktree, ledger 및 DAG를 다시 읽고 의존성을 재계산한다. identity가 유지된 완료 근거만 재사용한다.
3. 성공한 prerequisite가 만든 후속 READY와 무의존 READY를 포함해 다음 우선순위 READY 노드를 실제 실행한다. 남은 READY가 있으면 진행 없는 상태 보고로 턴을 종료하지 않는다.
4. 모든 HOLD를 수학/증명, 산술·반올림, 코드·계약, 입력, 환경으로 분류한다. 저장소 안에서 해결 가능한 항목은 counterexample 또는 missing-term falsifier를 만들고 research loop로 해소한다. 외부 immutable input이 필요하다는 증거가 있을 때만 그 exact input을 요구하며, 독립 READY는 계속한다.
5. 사용자가 고성능 연구 검토를 요청하면 요청된 model/effort로 연구/검토 계획을 수행하고 requested identity와 observed runtime identity를 구분한다. 관측되지 않은 runtime을 관측된 것으로 쓰지 않는다. 증명 검토와 구현은 분리하고, 구현 변경은 독립 검토 후 scoped admission한다.
6. 실패한 최초 반례·로그를 보존하고, tolerance/gate를 완화하지 않는다. 코드·증거·입력·명령·환경 identity를 checkpoint하고, 관련 범위만 다시 실행한다.
7. 전체 목표는 모든 required node의 acceptance와 review가 완료되고 required outputs가 검증될 때만 종료한다. typed HOLD/BLOCKED, unresolved scientific obligation, 미계측 비용, unverified environment가 남으면 전체 완료를 주장하지 않고 continuation을 유지한다.
8. ledger atomic order, repository gates, final push/ref verification을 적용한다. 게시 성공과 과학적 승격은 별도로 판정한다.

## 이번 재개의 적용

현재 재개에서는 A01 proof-term 검토, A01 구현, A04 additive-underflow, A07 accuracy-failure 원인 분석을 동시에 진행한다. A08/A09는 실제 dependency가 해결될 때까지 차단한다. 이번 continuation의 중지 조건은 status report나 증거 commit이 아니라, DAG의 모든 required acceptance가 검증된 상태다.
