# VigilODE 다음 계약: callback 효과와 유한 재시도 — R2/R3 원전·소스 조사

조사일: 2026-10-25 Asia/Seoul. 실행 범위: 문헌·소스 읽기와 구성 유도 준비.
실제 ODE/solver/test/benchmark 실행 0. 원격 쓰기 0. Desktop Commander 0.
적용: 사용자 지정 physmath-research-loop 1.0.0, R2 근거·재사용 및 R3 주장·전제 정렬.
읽은 정책: RESEARCH_POLICY.md, THEORY_LOOP.md, EVIDENCE_AND_HANDOFF.md.
이번 기록은 원전/코드의 경계를 연결한다. 구현과 실행 결과를 대신하지 않는다.

## 1. 실질 선택

기존 native source에는 이미 유한 attempt 수, 최소 시간진행, trial rollback, 예산 예약,
accepted state/출력 사전준비가 존재한다. 이 기능을 다시 설계하는 노드는 중복이다.
현재 확인한 최소 새 의무는 **callback이 소유하는 가변 상태와 accept 시 내보낼 효과를
어느 범위까지 transaction에 포함할 것인가**이다.

추천 방향은 기존 Transaction/Trial 및 output prepare/commit을 재사용하고,
trial 전용 독립 상태와 staged effect outbox를 명시하는 좁은 adapter다.
아직 source owner/reservation/live dirty 연결이 완료되었다고 판정하지 않는다.
그 연결과 실제 삽입 위치는 native inventory 담당의 최신 근거를 따른다.

## 2. 읽은 source identity

| 대상 | 고정 identity | 확인 |
|---|---|---|
| D3 science가 재사용한 admission | VigilODE 02060cc9dc4551dc5cf6ec86ad9b53b697a2c3ce, research/d3_dense_certificate_20261024/source/admission.rs | begin_to의 cumulative cap, no-progress Clock, lease identity, abort_stage, pending/halting 상태 |
| D3 driver | 같은 commit의 source/campaign_dense.rs | state/E0/cursor를 prepare 완료 뒤 commit; external_effects_rolled_back=false; I/O 실패는 process종료 |
| native frontier | research/local-frontier-20261024, aaf1f834d4c589e734ca7851a56c11bb44f7047a | native 조사자가 실제 재조회 |
| native Transaction | blob 1cfb7ef5dba52edafc2221dc4a074e3379f2f9fe; SHA256 eb4ade404b3e34d1a4c9e781eaa727b1b77736a22a5ec83b78518ec2b1827adf | S:Clone으로 trial 만들고 work는 rollback시에도 보존; Drop도 rollback계수에 반영 |
| native problem | blob 8f5926440d99204fc271553b3c0907c3880747f1; SHA256 890aa2218931a12ec7fcea7c72a0fb25eb588e7984a04ad2c5d89b4d5e95e066 | RHS/Jacobian/JVP가 Arc<dyn Fn...>; closure identity와 model_epoch hook |
| native generic_verified | blob 4d8e906f687dae338f3cec031e59e458d1b417b9; SHA256 b8944a94d0909e4746f080abebc8aa33d43c850d704c4c13c2f4ed0f9fcffb1a | immutable capability owner, max_attempts, min_step, candidate/queries 준비 후 state/radius/cursor/history commit |

native output.rs의 prepare_dense_interval_with_radii/commit_prepared 존재는 native
inventory 담당의 읽기 보고와 SOURCE_PINS.json에 연결한다. 이 문서에서 직접 모든 output
함수를 별도 정밀 감사한 것으로 세지 않는다.

## 3. 원전과 적용

### SUNDIALS CVODE

공식 매뉴얼:
https://sundials.readthedocs.io/en/latest/cvode/Usage/index.html
관련 절: 3.4.3.5 solver call, 3.4.3.7 optional inputs, 3.4.4.1 CVRhsFn,
3.4.4.6 CVLsJacFn.

직접 확인한 계약:
- RHS 반환값 0은 성공, 양수는 복구 가능 실패, 음수는 복구 불가능 실패를 뜻한다.
- recoverable RHS 오류에 대해 nonlinear solve 재시도나 step축소를 할 수 있으며,
  첫 RHS 호출·일부 order-one error 상태·이미 승인된 상태에서의 문제 등에는 복구 한계가 있다.
- 반복 error-test/convergence/recoverable-RHS 실패는 typed failure로 반환한다.
- maxsteps는 다음 요청 output time까지의 step 수이고 기본 500; 음수는 검사를 끈다.
  따라서 기존 VigilODE의 전체 session 누적 attempt cap과 같은 범위라고 취급하지 않는다.
- error-test failure 제한과 minimum step option이 각각 있다.
- Jacobian callback도 success/recoverable/unrecoverable 반환 의미가 있다.
- RHS/Jacobian 사이 재사용 가능한 user_data cache가 허용되며, 매뉴얼이 user_data의
  임의 외부효과를 자동 rollback한다고 약속하지 않는다.
- 입력 y를 callback 안에서 바꾸어 물리적 위반을 숨기는 방식을 피하라고 명시한다.

이 자료는 기존 retry/failure API의 선행구현 근거다. 새 termination 원리를 주장할 근거가 아니다.

### Boost.Odeint

공식 Controlled Stepper 개념:
https://www.boost.org/doc/libs/latest/libs/numeric/odeint/doc/html/boost_numeric_odeint/concepts/controlled_stepper.html

실패한 try_step에서 x와 t를 그대로 유지하고 다음 dt를 줄인다는 의미를 직접 확인했다.
이 보존 계약의 대상은 수치 state/time이다. 사용자 functor가 다른 객체나 I/O를 수정하는
경우까지 포함한다는 약속은 이 계약에 없다. VigilODE의 numerical rollback과
callback effect rollback의 범위를 구분하는 비교 원전으로 사용한다.

### SciML

공식 options:
https://docs.sciml.ai/DiffEqDocs/stable/basics/common_solver_opts/
공식 callbacks:
https://docs.sciml.ai/DiffEqDocs/stable/features/callback_functions/

dtmin 및 maxiters 한계, force_dtmin을 켜면 최소 step에서 tolerance를 무시하고 계속할 수
있다는 옵션 의미를 확인했다. 현재 VigilODE 고정 정확도에서 force_dtmin 방식으로
성공을 만드는 변경은 동등한 계약이 아니다.
DiscreteCallback의 condition은 step 끝에, ContinuousCallback은 구간 내부 root를 탐색하여
실행한다. 이 API는 event 의미와 사용자 affect를 제공한다. 일반 RHS의 임의 외부효과
rollback과 같은 개념으로 합치지 않는다.

SciML FAQ의 dtmin/positivity 실패 설명은 검색 결과에 포함됐지만, 특정 solver 실행의
실패만으로 모든 수치 알고리즘·모델에서 positivity가 불가능하다고 판정하는 강한
no-go 근거로 채택하지 않았다.

### Rust Clone

공식 Clone:
https://doc.rust-lang.org/std/clone/trait.Clone.html

Arc/Rc 및 공유 reference의 Clone은 같은 underlying data를 가리킬 수 있으며,
Arc<Mutex<T>>를 가진 struct의 derive(Clone)도 가변 상태를 공유한다.
따라서 Transaction<S:Clone>의 S 복제는 자동으로 deep/independent snapshot이 되지 않는다.
OdeProblem에 담긴 Arc<dyn Fn...> 자체를 clone하는 것으로 callback capture를 격리할 수 없다.

### 학술 문헌 조사

SciSpace 두 질문의 원문/응답을 별도 JSON에 보존했다.
- finite rejection/minimum floatingpoint step/recoverable callback: 10 records.
- SUNDIALS recoverable RHS/failure limits: 10 records.

직접 관련되는 scholarly pointer:
Gardner et al., Enabling New Flexibility in the SUNDIALS Suite of Nonlinear and
Differential/Algebraic Equation Solvers, ACM TOMS, DOI 10.1145/3539801.
SciSpace abstract는 확인했지만 원문 fulltext의 세부 알고리즘을 직접 읽은 근거로 세지 않는다.
이번 callback 계약의 load-bearing 근거는 실제 공식 매뉴얼과 source다.

다수 search 결과는 step controller/SDC/RIDC의 효율성 주제였으며 외부효과 rollback을
입증하지 않는다. 관련성 낮은 최신 논문을 수량을 채우기 위해 채택하지 않는다.
신규성·strong no-go 채택은 하지 않는다.

## 4. source가 이미 만족하는 유한 attempt 명제

**명제 F.** cap M이 유한 정수이고, attempts A가 0으로 초기화된 하나의 session에서
유일한 begin 성공이 A를 정확히 1 올리며 A>=M이면 다음 begin을 막는다고 하자.
각 begin 뒤의 유한 구성 호출이 모두 반환하고, driver가 각 lease를 한 번 resolve하며,
retry 중 새 session을 만들거나 counter를 reset하지 않으면 성공한 begin 수는 M 이하이다.

증명: A=0. k번 성공 후 A=k라는 귀납 불변식이 유지된다. k=M에서는 다음 begin 전
guard가 거부한다. reject/rollback은 A를 줄이지 않는다. 따라서 각 유한 반환의
합성으로 이루어진 driver는 최대 M개의 trial 뒤 성공 또는 terminal failure를 반환한다.
이 명제는 한 callback이 영원히 반환하지 않는 상황을 중단시키는 wall-time 보장이 아니다.

D2/D3 Session::begin_to는 cap guard가 increment보다 앞에 있고 retry는 같은 session이다.
native generic_verified도 while 안에서 max_attempts를 검사하고 시도마다 증가시킨다.
해당 기존 훅을 그대로 재사용하면 되는 부분이다.

**명제 P.** 표현 가능한 endpoint next가 finite이고 next>t일 때만 lease를 발행하면,
승인된 time은 엄격히 증가한다. D2/D3는 next<=t를 Clock으로 막는다.
h반감만으로는 callback 실행시간이나 admissible step의 존재를 보장하지 않으며,
cap과 progress는 별개의 종료 경로다.

## 5. 실질 새 의무의 구성형 계약

관리 대상 committed state를 X=(t,y,radius,output_cursor,history,callback_context,outbox)라 두자.
work/attempt ledger W는 rollback 대상에서 분리한다.

가정:
A1. trial snapshot C(X)는 이후 trial mutation과 alias하지 않는 독립 저장소를 가진다.
A2. callback의 rollback 대상 effect는 trial callback_context나 staged journal에만 기록된다.
A3. 모든 callback와 검사, allocator/환경 동작은 계약상 반환한다. panic/abort/I/O의 취급은 별도 명시한다.
A4. 승인에 필요한 후보·domain·norm·dense·query 준비는 publish 전에 완료된다.
A5. commit은 이미 준비한 단일 owning capsule을 옮기는 연산으로 구성하고,
    외부 sink acknowledgement는 별도 상태로 관리한다.

**보존 정리 T.** A1–A5 하에서 reject/abort/drop이 staged capsule을 폐기하면,
관리 대상 X는 bitwise/structural 의미상 이전 값이고 W는 이번 work만큼 누적된다.
accept가 prepared capsule을 교체하면 newstate와 승인된 journal이 같은 epoch를 가진다.

증명은 dataflow로 가능하다. prepare 동안 write 가능한 저장소는 C(X)와 W뿐이다.
A1–A2에 의해 X의 저장소에는 write edge가 없다. reject는 C(X)를 drop하므로 X'=X다.
accept는 하나의 prepared tuple을 move하므로 각 구성요소가 같은 trial에서 나온다.
이 증명은 immutable Rust borrow만으로 모든 숨은 effect를 봉쇄했다고 가정하지 않는다.
Fn/Clone에는 A1–A2가 자동 함의되지 않으며, trusted adapter나 실제 owned data shape로
의무를 확인해야 한다.

**외부 송신 상태.** accepted outbox를 만든 뒤 외부 sink가 실패할 수 있다.
수치 승인을 되돌려 동일 RHS를 재실행하면 external effect를 중복시킬 수 있다.
권장 처리는:
1. accepted epoch와 순번을 가진 immutable outbox를 보존한다.
2. sink ack를 받을 때 delivery cursor만 진행한다.
3. sink가 부분 성공 후 error를 돌려준 경우에는 at-least-once 재시도 가능성을 명시한다.
4. exactly-once external effect는 sink의 idempotency/deduplication 계약과 결합할 때만 주장한다.

단순 owned Vec append는 controlled in-memory effect의 예제다.
임의 네트워크·파일·shared counter를 자동 rollback하는 일반 callback API를 이번
유도만으로 certified로 바꾸면 안 된다.

## 6. 최소 판별 검사 제안(아직 실행하지 않음)

- owned callback context를 변화시킨 뒤 recoverable refusal: committed context/state/output이 동일,
  attempted work는 증가, 다음 retry는 같은 baseline으로 시작.
- accept: context 및 staged output이 정확히 한번 해당 epoch로 승격.
- stale/double resolution: 두 번째 publish가 거부되고 outbox가 중복되지 않음.
- fatal callback: partial journal 폐기, terminal status, 다음 호출 없음.
- max-attempt exhaustion: callback 호출 수가 cap과 일치; checkpoint/yield로 budget이 새로 생기지 않음.
- no-progress time: callback 진입 전에 typed Clock/minstep failure.
- Arc<Mutex> capture를 가진 clone의 반례: 일반 Clone을 isolated snapshot으로 인증하지 않는 경계 검사.
- sink failure after partial external delivery: numerical retry 금지, pending outbox 보존 및
  acknowledgement 의미를 별도로 시험.
- accepted scientific prefix와 workledger가 서로 다른 rollback 정책을 갖는지 확인.

이들은 scope 선택 후 필요한 최소 집합으로 고정해야 하며 전체 기존 회귀 재실행 지시가 아니다.

## 7. 정책·첨부 대응

01 Low-ell audit는 실제 BASS/htt 본체 physical outputs와 solver-ready/statistics-ready를
분리하라고 한다. 이번 consumer 또는 callback contract 완료를 BASS harmonic/11family
완료로 계산하지 않는다. 04/05는 actual code-path와 reference에 따른 minimal patch,
math→code 순차 audit를 요구한다. 07/08은 실제 생성할 도표의 semantic/가독성 검토에
적용하되 이 문헌 단계 자체에 장식 plot을 추가하지 않는다.

전체 global completion은 null, production HOLD, 닫힌 D2/D3 budget은 보존한다.
새 solver 실행이나 native production 이식은 root의 frozen contract 및 source
owner/reservation 근거에 연결하여 수행한다.

## 8. 조회 기록과 한계

Exa: 4 searches, 18 result slots; 6 URL fetch requests(5 unique URLs; SUNDIALS는 긴 본문
첫35000자에서 CVRhsFn 세부가 잘려 최대250000자로 확대하여 실제 해당 절을 읽음).
SciSpace: 2 searches, 20 returned records.
GitHub: D3 admission/driver read 2 success. LLNL source의 추측 경로 2건 404는 보존;
오류를 우회한 source access로 세지 않고 official rendered manual로 필요한 사실을 확인.
native files는 sibling이 GitHub에서 고정 identity로 취득한 실제 workspace source를 직접 읽음.
원전 rawfetch JSON은 내부 추적용이며 full copyrighted text를 공개 Git 보고서에 복제하지 않는다.

원자료:
- exa_search_sundials.json / exa_search_sciml.json
- exa_search_rhs_contract.json / exa_search_boost.json
- exa_primary_readback.json / primary_boost_rust_readback.json / sundials_full_readback.json
- scispace_retry_search.json / scispace_sundials_search.json
- failed_sundials_source_reads.json
- admission.rs / campaign_dense.rs

완료 범위: R2/R3 분담 산출물 및 조건부 구성 유도 준비 COMPLETE_SCOPED.
실제 구현/실험/생산 gate 완료는 NOT_RUN. independent personnel NOT_RUN.

