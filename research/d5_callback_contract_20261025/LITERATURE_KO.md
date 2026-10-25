# D5 다음 계약의 원전·소스 조사와 수학적 경계

조사일: 2026-10-25 (Asia/Seoul).
담당: next_contract_sources, 1회 조사·유도 준비 pass.
상태: R2/R3 분담 완료; 다음 scope는 root가 D5 strict generic callback/Jacobian staging/domain contract로 선택했다.
이 문서는 새 실행 결과를 주장하지 않는다. 이 분담의 solver/test/benchmark/NCP/과거 검산 실행은 모두 0이고, 원격 mutation 및 Desktop Commander 호출도 0이다.

## 1. 선택과 이유

다음 구현 의무는 **RHS/Jacobian 호출 직전의 입력 domain 검사와 동일 model epoch 확인, 전용 scratch의 출력 검사, 성공한 출력만 게시하는 경계**다. D2의 제조 모델 stage guard와 D4의 outer producer journal을 재사용하되, 임의 Jacobian callback을 이미 처리했다고 넓혀 말하지 않는다.

물리 해석과의 연결은 직접적이다. 농도·분율·양의 에너지 등으로 해석하는 상태에서 callback 입력 자체가 정의역을 벗어나면, 사후 endpoint 거절만으로 그 호출을 정당화할 수 없다. Jacobian도 같은 상태와 같은 model을 선형화해야 한다. 일반 callback의 성공 반환은 행렬의 수학적 정확성이나 연속 구간의 안전성을 인증하지 않으므로, D5의 결과는 호출/게시 계약으로 한정한다.

공개 native HEAD aaf1f834d4c589e734ca7851a56c11bb44f7047a에서 source hook은 이전과 동일하다. native 담당자의 현재 owner/reservation/live dirty/completion 연결은 별도 입력 의존성으로 남아 있다. 이 조사만으로 native takeover나 production 이식을 허가한 것으로 계산하지 않는다.

## 2. 직접 읽은 source와 재사용할 hook

| 고정 대상 | 읽기에서 확인한 사실 | D5에서의 사용 |
|---|---|---|
| D4 final 3c8877ea0409495752dcfab42426e679e3ba92d3 / THEORY.md, CONTRACT.md, source/driver.rs, LITERATURE_KO.md | owned journal, pending batch, 정확한 ACK, callback capture 및 crash 복원 범위 제외 | journal/ACK를 새로 구현하지 않고 기존 scientific acceptance와 효과 경계 재사용 |
| D4 driver blob 563e2692eea01da25943214bad06cb7eec7d3f46 | advance는 private Session.begin 이후 producer 한 번; pending이면 begin 이전 거절; 성공 후 batch를 move | callback failure를 기존 producer failure/Session resolution 경로에 연결 |
| D4 inherited stage_safe blob 62b32b9e65116b8f5d9d559641299cf3bfb40282 | 각 RHS 직전 domain check, NaN 초기화 출력, finite 검사; Jacobian은 고정 Riccati 제조 모델의 analytic diagonal | RHS scratch 패턴 재사용; 일반 Jacobian 처리 완료로 세지 않음 |
| D4 inherited admission blob 0b1d4fd7be28e2d2f8b61a03ef6f5faf7dcebd0d | 누적 attempt cap, no-progress clock, lease, abort_stage, finish, accepted snapshot | 재시도/종료를 새로 설계하지 않음 |
| native problem.rs blob 8f5926440d99204fc271553b3c0907c3880747f1 | Arc callbacks, model_epoch, 호출 전 work charge, shape/일부 finite 검사, 일반 continuous authority 거절 | 기존 callback 인터페이스와 계수 의미를 참조한 opt-in 경계 |
| native generic_verified.rs blob 4d8e906f687dae338f3cec031e59e458d1b417b9 | private VerifiedModelProvider, immutable model factory, 전체 구간 tube 검증, 출력 준비 후 state/radius/cursor commit | 이미 존재하는 dense capability를 새 파생 노드로 재발명하지 않음 |

native Transaction 및 output prepare/commit의 존재와 source identity는 native intake의 SOURCE_PINS.json 및 조사 결과를 재사용한다. 이 문서 담당이 그 두 파일 전체를 별도로 재감사한 것으로 세지 않는다.

Source permalink:

- [D4 final theory](https://github.com/cosmosapjw-quantum/vigilode/blob/3c8877ea0409495752dcfab42426e679e3ba92d3/research/d4_effect_journal_20261025/THEORY.md)
- [D4 driver](https://github.com/cosmosapjw-quantum/vigilode/blob/3c8877ea0409495752dcfab42426e679e3ba92d3/research/d4_effect_journal_20261025/source/driver.rs)
- [Native problem](https://github.com/cosmosapjw-quantum/vigilode/blob/aaf1f834d4c589e734ca7851a56c11bb44f7047a/crates/rodas5p-integrators/src/problem.rs)
- [Native verified driver](https://github.com/cosmosapjw-quantum/vigilode/blob/aaf1f834d4c589e734ca7851a56c11bb44f7047a/crates/rodas5p-integrators/src/generic_verified.rs)

### 실제로 추가되는 계약

problem.rs의 eval_rhs_into는 caller의 out에 직접 쓰며, callback Err 이후 부분 쓰기 원복을 약속하지 않는다. 입력 shape를 확인하고 호출 전 work를 charge하며, 성공 시 출력 finite를 검사한다. 이 함수에서 일반적인 time/state domain 검사를 수행하는 경로는 보이지 않는다.

dense_jacobian_into의 in-place callback 분기는 work를 charge한 뒤 fill(t,y,out)의 결과를 그대로 반환한다. 따라서 이 함수 경계 자체에서 Err의 부분 쓰기나 성공한 NaN 행렬을 staged valid output으로 변환하지 않는다. explicit dense_jacobian 분기는 출력 shape를 확인하며, JVP로 구성하는 분기는 각 column finite를 확인한다. 이러한 서로 다른 low-level 계약을 모두 이미 atomic이라고 취급하지 않는다.

이는 호출자가 관리하는 low-level scratch API의 기존 계약일 수 있다. 기존 API 전체를 오류로 단정하거나 의미를 조용히 변경할 근거는 아니다. D5는 strict opt-in adapter임을 명시해야 한다.

**호환성 주의:** with_jacobian_into의 기존 계약은 이전 행렬을 주고, 고정 sparsity에서 nonzero가 될 수 있는 모든 항을 callback이 갱신하며, 한 번도 쓰지 않는 항은 항상 zero라고 전제한다. NaN scratch의 모든 항을 유한하게 만드는 full-write 계약은 더 강하다. 기존 sparse fill callback을 무변환으로 strict callback이라고 인증할 수 없다. strict callback은 zero 항까지 명시적으로 쓰거나, 고정 sparsity의 별도 검증된 adapter가 필요하다.

## 3. 세 후보 비교와 DAG 경계

| 후보 | 기존 hook | 정확히 새로 필요한 의무 | 최소 판별 입력 | 가정과 원전 한계 | 현재 disposition |
|---|---|---|---|---|---|
| A. 일반 RHS/Jacobian 입력·출력 staging | D2 pre-RHS guard, D4 owned trial, Session cap/lease, native callback/work/epoch | 각 dispatch 직전 domain+epoch, private scratch, 모든 출력 finite/complete, 성공 status 및 post epoch 뒤 게시; 오류가 accepted state로 유입되지 않음 | domain 밖 입력, RHS/J 각각 partial-write→Err, Ok+미기록 항/NaN, epoch 변경, 정상 full-write | callback 반환, domain predicate·epoch 계약; finite 값과 epoch equality는 진짜 Jacobian의 증명이 아님 | **root 선택 D5** |
| B. 일반 nonlinear dense-domain capability | native immutable VerifiedModelProvider와 모델별 factory, D3 fixed Riccati certificate | 저장된 곡선과 모델의 동일성, 전체 시간·상태 tube의 residual/Jacobian enclosure, tube closure, rounding enclosure | 점 표본은 통과하지만 사이에서 큰 residual/정의역 위반이 있는 곡선; 잘못된 model binding | point callback만으로 whole-domain 상계를 얻는다는 가정은 허용되지 않음 | D5와 분리; 기존 UnsupportedUnboundModel 유지 |
| C. durable delivery 또는 crash continuation | D4 stable pending ID/payload, 정확한 ACK, conditional sink S1–S5 | sink 단독이면 effect+ID/payload 원자 durable commit 및 동일 payload dedup. solver continuation이면 accepted snapshot, pending, attempts/work/cap, next step, terminal, 마지막 ACK/run identity의 일관된 복원까지 필요 | effect 후 ACK 유실→재전송; checkpoint 도중 crash; restart 뒤 attempt cap 재생성 여부 | durable storage/정직한 ACK/namespace 가정; DB transaction은 임의 네트워크 효과까지 원자화하지 않음 | 별도 gate; D5에 구현하지 않음 |

후보 B/C는 새 노드를 자동 생성하라는 지시가 아니다. 다음 scope를 고를 때 필요한 의존성만 남긴다. D5는 D2의 domain, D4의 owned effect, 기존 Session/Transaction/work accounting을 조합하며 새로운 norm-only 노드를 만들지 않는다.

## 4. D5 수학 계약의 구성형 초안

다음은 source에 대응시킬 **DERIVED_SCOPED** 계약이다. 아직 실행 검증된 정리로 표기하지 않는다.

고정 dimension을 n, callback 입력을 x=(t,y), 예상 model epoch를 e, 누적 work를 W라 하자. RHS 출력은 n개, Jacobian 출력은 n×n개 숫자다. 관리 대상 accepted scientific state는 S, 게시된 callback 결과는 O, trial scratch는 Q다.

가정:

1. dimension·time/state finite·물리 domain P(x)를 callback dispatch 전에 검사한다. P는 계약에 명시된 입력 판정이고 반환한다.
2. producer가 받는 accepted snapshot은 관리 scientific storage와 가변 alias가 없다. Q는 독립 소유 저장소이고 O/S를 callback에 mutable로 제공하지 않는다.
3. Q의 모든 항은 NaN으로 초기화한다. strict callback은 성공하려면 각 항에 유한 값을 쓴다.
4. callback은 정상 반환하거나 wrapper가 명시적으로 처리하는 unwind로 끝난다. process abort, nonreturning callback, allocator abort는 해당 반환 정리 밖이다.
5. 호출 전과 후의 model epoch는 e와 같아야 한다. 외부 model이 바뀌면 epoch가 반드시 바뀌며, 한 trial 안에서 ABA 재사용/overflow가 없다는 caller 계약을 둔다. 엄밀한 model immutability가 필요하면 owned immutable provider를 사용한다.
6. callback status, scratch shape/finite, post epoch를 모두 통과한 결과만 정확한 trial/lease의 준비된 결과로 게시한다. 실패를 무시해 이전/부분 결과를 Session.finish에 전달하지 않는다.
7. W는 callback 진입 전에 charge하고, 실패나 journal 폐기로 되돌리지 않는다. 입력 precheck에서 거절한 호출은 callback 실행으로 세지 않는다.
8. 게시 이후 추가 fallible callback/할당이 없도록 결과를 미리 준비한다. 여러 callback의 결과가 하나의 candidate를 이룰 때는 전부 성공한 prepared capsule만 scientific commit에 전달한다.

**명제 A1 — 금지 입력의 callback 비진입.**
dispatch가 P(x), finite/shape, expected epoch를 모두 통과한 분기에서만 발생하면 precheck 실패 시 callback 호출 수 변화는 0이다. 이는 실제 입력점의 의미이며, 그 점 사이의 dense curve가 domain 안에 있다는 결론은 포함하지 않는다.

**명제 A2 — 관리 출력의 거절 보존.**
Q에만 쓰고 모든 검사를 통과하기 전 O를 변경하지 않으면 status Err, 미완성/비유한 출력, epoch mismatch 또는 처리된 panic에서 O'=O다. accepted S를 callback 경계에서 쓰지 않고 기존 Session 승인으로만 갱신하면 S'=S도 유지된다. W는 실제 dispatch 수만큼 누적된다.

증명은 write 권한과 분기 순서로 충분하다. prepare 중 변경 가능한 곳은 Q 및 W이며, failure는 Q를 버린다. O/S로 쓰는 경로가 성공 분기에만 있으므로 failure에 그 경로가 없다. 외부 capture는 이 관리 대상에 속하지 않는다.

**명제 A3 — full-write 검사.**
Q의 모든 항이 NaN에서 시작하고 성공 게시가 각 항의 finite를 요구하면, 한번도 유한 값으로 바뀌지 않은 항을 포함한 Q는 게시될 수 없다. native fixed-sparsity의 기존 zero-default 의미를 자동으로 상속하지 않으므로 strict full-write 계약을 별도로 선언한다.

**명제 A4 — 실패 후 유한한 재시도.**
D5는 새로운 retry loop/counter를 도입하지 않는다. 한 기존 lease 안의 finite 반환 callback들을 오류 처리하고 동일 Session을 resolve한다. D4 L1의 p≤a≤M에서 p는 outer producer 호출 수이며, 내부 RHS/Jacobian 수는 별도 W에 기록한다. callback 종류가 늘었다고 p를 전체 callback 수로 다시 해석하지 않는다.

이 계약의 성공은 “이 입력에서 callback이 완전한 유한 배열을 반환했고, 관측 epoch가 일치했다”는 뜻이다. J=D_y f, ODE 해의 정확도, Bernstein/domain 인증, model의 불변성을 finite 배열 검사만으로 얻지는 않는다. epoch equality는 정직한 counter 가정 아래의 변화 탐지 hook이며 arbitrary interior mutation의 독립 증거가 아니다.

## 5. 최소 제조 모델과 판별 입력

새 장거리 trajectory 대신 callback 계약을 식별할 수 있는 작은 reversible exchange를 제안한다:

\[
f(x,y)=(-kx+qy,\ kx-qy),\qquad
J=\begin{pmatrix}-k&q\\k&-q\end{pmatrix},\quad k,q>0.
\]

이는 두 species의 보존 교환이다. \(\dot x+\dot y=0\)이고 \(m=x+y\)가 일정하다.
\[
x(t)=\frac{q}{k+q}m+
\left(x(0)-\frac{q}{k+q}m\right)e^{-(k+q)t},\qquad y(t)=m-x(t).
\]
초기 x,y≥0이면 이 식은 두 equilibrium/초기 값의 convex combination이므로 비음수다. 이 간단한 유도는 제조 callback의 값과 Jacobian을 정확히 지정하기 위한 것이며 BASS 물리 검증을 대신하지 않는다.

k=1, q=2, (x,y)=(1/4,3/4)이면 binary64로 정확한 callback 목표는 f=(5/4,-5/4), J=[[-1,2],[1,-2]]다. n=30 interface가 필요하면 첫 두 성분의 block과 나머지 zero를 명시적으로 채우는 embedding을 사용할 수 있다. 이 embedding은 actual BASS/htt 모델이 아니다.

| 판별 | 새로 구분할 행동 | 검증 대상 |
|---|---|---|
| 정상 full-write | RHS/J의 모든 항 및 zero 항을 채워 성공 | 준비된 결과와 기존 scientific commit 경로의 정상 연결 |
| 음수 state 또는 time domain 밖 | callback이 실행되면 실패하도록 설정 | precheck에서 callback 호출 0 |
| 유효 shape이나 NaN/Inf 입력 | callback이 실행되면 실패하도록 설정 | domain predicate가 NaN 비교로 우연히 통과시키지 않음 |
| RHS 첫 항만 쓴 뒤 recoverable/fatal Err | 이전 O에 구별 가능한 sentinel 저장 | O/S 보존, attempted RHS work 증가 |
| Jacobian 일부만 쓴 뒤 Err | 이전 행렬을 구별 가능한 값으로 둠 | 부분 행렬이 게시되지 않음 |
| Ok이지만 한 항 미기록 또는 NaN | callback status와 numerical output 분리 | strict output refusal |
| callback 안에서 epoch 변경 | 유효 full-write와 결합 | post epoch mismatch로 게시 거절 |
| 호출 전 expected epoch mismatch | 진입 counter를 별도 둠 | callback 진입 전 거절 |
| captured shared counter 증가 후 Err | 관리 결과는 동일 | callback 외부효과가 실제 남는 경계 확인; rollback을 주장하지 않음 |
| 한 성공 RHS 뒤 실패 Jacobian | 한 trial에 둘 다 사용 | scientific acceptance가 일부 성공만으로 일어나지 않음 |

이는 검사 설계 제안이며 실행 횟수가 아니다. root가 frozen contract에 필요한 최소 집합만 채택한다. 기존 D2/D3/D4 시험 및 finite trace corpus 재실행은 제안하지 않는다.

일반 partial_t/JVP/fallback을 지원 범위에 넣는 경우에만 추가 의무가 생긴다. 현재 native partial_t_finite_difference는 t±eps에서 RHS를 호출하므로 시간 domain도 guard되어야 한다. D5가 RHS/Jacobian만 지원하면 이 fallback 전체를 완료했다고 넓히지 않는다. JVP는 state 외 direction/output shape 및 finite 조건이 필요하다. 이들은 별도 범위 선언으로 처리한다.

## 6. 원전 조사와 주장 범위

### 재사용한 D4 문헌

D4 LITERATURE_KO.md가 이미 읽고 연결한 공식 CVODE callback success/recoverable/unrecoverable 의미, Boost의 state/time 보존, SciML의 minstep/iteration 옵션, Rust Clone의 shared storage 의미를 재사용했다. 같은 공식 문서를 새 결과 수를 채우기 위해 다시 검색하지 않았다.

특히 native CoreError에 CVODE 양수/음수 규약이 존재한다고 추정하지 않는다. 새 typed callback failure를 기존 root 계약에 맞게 명시해야 한다. Rust Clone과 epoch의 한계는 실제 Arc callback source의 설명과 일치한다.

- [D4 literature](https://github.com/cosmosapjw-quantum/vigilode/blob/3c8877ea0409495752dcfab42426e679e3ba92d3/research/d4_effect_journal_20261025/LITERATURE_KO.md)
- [CVODE official usage](https://sundials.readthedocs.io/en/latest/cvode/Usage/index.html)
- [Rust Clone](https://doc.rust-lang.org/std/clone/trait.Clone.html)

### 새로 직접 읽은 dense 경계 원전

Arnold Neumaier, *Global, rigorous and realistic bounds for the solution of dissipative differential equations. Part I: Theory*, Computing 52 (1994), DOI 10.1007/BF02276880.
[Author manuscript](https://arnold-neumaier.at/ms/ode.pdf).

직접 읽은 핵심은 Theorem 3.5의 domain/tube 이탈 방지, Theorem 4.1의 closure 조건, Proposition 4.3의 전체 시간·상태 집합 Jacobian/residual enclosure, Corollary 4.5의 residual 기반 bound다. 일정 log-norm 상계 μ와 residual 상계 ε가 해당 영역에서 유효하면 초기 반경 δ에서 δ exp(μt)+ε t exp1(μt) 형태의 bound를 얻는다. point callback의 finite 값, 유한 개 sample 또는 임의로 제공한 scalar만으로 이 전제가 충족되지는 않는다.

이 원전은 D5로 general dense certification을 주장할 수 없다는 범위 구분에 사용한다. native에 이미 있는 private immutable provider를 새 이론 발명으로 소개하지 않는다. 새로운 norm-only scheme도 제안하지 않는다.

SciSpace는 Cooper (1971), DOI 10.1007/BF01436325 및 Ernsthausen (2017) defect-control thesis도 반환했다. 이 둘은 이번에 abstract/search highlight 수준이며 full proof를 읽은 것으로 세지 않는다. 최신 PINN 등 관련성 낮은 반환 결과는 D5 load-bearing 근거에 채택하지 않았다.

### 새로 직접 읽은 durable 경계 원전

[SQLite Atomic Commit](https://sqlite.org/atomiccommit.html), 특히 hardware assumptions와 single-file commit;
[SQLite WAL](https://sqlite.org/wal.html), 특히 overview, commit 및 synchronous policy.

단일 DB transaction의 원자적 복구는 구현 가능한 durable sink의 기초가 된다. WAL 문서는 FULL일 때 매 transaction commit에서 WAL sync를 수행하고 NORMAL에서는 이를 생략한다고 설명한다. 저장장치/OS의 flush 의미 등 가정이 남으며, WAL의 여러 ATTACHed DB 변경을 하나의 전체 원자 transaction으로 취급할 수도 없다.

이는 아직 채택하지 않은 후보 C의 원전이다. SQLite가 외부 네트워크 효과, D4 live Session, user callback capture까지 원자화한다는 주장은 하지 않는다. sink receipt 구현만으로 accepted scientific state와 attempt budget의 crash continuation이 증명되는 것도 아니다.

## 7. 보존 상태와 완료 경계

- D2의 15개 새 시험+31개 회귀, 12개 제조 궤적, 15,182 stage/2,212 trial 검산과 Bernstein 증거는 재사용한다.
- endpoint-only forbidden RHS 507, guard 두 경로 0; halving 1,032 attempts/5,323 RHS와 rate cap 602 attempts/4,816 RHS는 기존 workload 비교다. wall-time speedup으로 바꾸지 않는다.
- scalar-linear [0,2] 인증과 비선형 cap의 heuristic 역할은 그대로 구분한다.
- D3 fixed-Riccati dense certificate와 D4 owned journal+ACK 완료 증거는 재실행하지 않는다.
- production forbid(unsafe_code)는 보존한다. D5가 별도 safe crate이면 inherited research MXCSR 읽기 경계를 재사용하여 분리하고 생산 unsafe 허용으로 해결하지 않는다.
- actual BASS/htt, 일반 J=Df 정합성, 일반 dense-domain, 임의 callback capture rollback, durable crash continuation, 현재 native 담당 source/예약/dirty/완료의 live 증거는 별도 gate다.
- D2/D3/D4 closed budget은 변경하지 않는다. older_remaining=null, reset=false, global completion=null, production=HOLD.
- independent personnel=NOT_RUN. 이 분담은 실제 native_subagent 작업이지만 전체 프로젝트의 과거 native_subagent 상태를 소급하여 PASS로 바꾸지 않는다.

## 8. 조사 기록과 skill_context_used

실제 조회:

- GitHub exact-ref file fetch 4: D4 THEORY, CONTRACT, driver, literature.
- native intake가 고정 ref로 읽고 저장한 problem.rs/generic_verified.rs의 필요한 실제 구간을 직접 읽음. 복원된 inherited stage_safe/admission도 직접 읽음.
- SciSpace search 1, 반환 record 10.
- Exa search 2, 요청 result slot 7.
- Exa primary fetch 1 batch, unique URL 3.
- 신규 solver/test/benchmark/NCP/과거 계산 실행 0, 원격 mutation 0, Desktop Commander 0.
- 반복 review 0. source/문헌의 한 pass이며 이 구현의 complementary review로 계산하지 않는다.

Private raw evidence는 이 문서와 분리하여 private/에 둔다:
scispace_generic_residual.json, exa_dense_residual.json,
exa_durable_checkpoint.json, primary_readback.json, d4/ exact source reads.
원전 전문을 공개 보고서에 복제하지 않는다.

skill_context_used:

- 사용자 지정 physmath-research-loop 1.0.0을 적용했다.
- package: e1/skill-6aa0657100b08191b7d67482d1f97c1d
- 읽은 reference: RESEARCH_POLICY.md, THEORY_LOOP.md (R2–R5 의무 적용), EVIDENCE_AND_HANDOFF.md.
- 적용 조항: 기존 완료 증거 재사용, 원전/유도/구현/검증 상태 분리, 실제 source hook 대조, 구성형 대안 비교, 범위를 한정한 주장, strong no-go를 새로 채택하지 않음, 닫힌 예산/원본 로그 보존.
- 해당 분담 완료: R2/R3와 R4 선택 지원 COMPLETE_SCOPED.
- R5 초안: 명제 A1–A4 및 작은 제조 모델 유도 DERIVED_SCOPED.
- 새 구현·새 실험·production 승인: 이 분담에서는 NOT_RUN.

## 9. S0 전 추가 source 근거 — C21H/C21I 재사용

후속 R2 조사에서 bass_cr PR85/86의 C21H/C21I source를 직접 읽었다.
정확한 경로·Git blob·본문 한계는 RELATED_ADDITIONAL_KO.md §4에 있다.
C21H Rust bridge는 이미 callback 입력 finite/time, RPC identity/status/shape/finite를 검사하고 완성된 Vec/행렬을 받은 뒤 반환한다.
C21I는 고정 optical model에 대해 domain·provider pre/post guard, cell-owned zero-reset Jacobian scratch, 성공 RHS의 독립 copy를 구현한다.

따라서 D5는 “callback staging/domain 검사를 처음 도입”하는 연구가 아니다.
이미 있는 패턴을 재사용하여 direct in-process arbitrary RHS/Jacobian의 strict full-write 누락 검출,
공통 witness 아래 pair 전체 성공의 prepared publication, 일반 caller domain의 양측 FD preflight라는
좁은 구성 의무를 닫는다. H/I의 cache/transport/Schur/physics 및 완료 실행은 복제하지 않는다.
이 추가 source 읽기는 과학 실행 0이며, D5 선택과 generic dense UnsupportedUnboundModel 유지 판단을 바꾸지 않는다.

