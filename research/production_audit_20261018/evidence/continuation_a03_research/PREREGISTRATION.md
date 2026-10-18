# A03-LOCAL-DAMPED-CONTRACT-01 preregistration

본 bounded unit은 **read-only source intake와 수학 유도, 다음 실행 계약 작성**이다.
코드, 테스트, 빌드, benchmark, ledger, 상위 DAG는 수정·실행하지 않는다.
결과 판정은 별도 `RESEARCH_DECISION.json`에 두고 이 문서에 실행 결과를 붙이지 않는다.
새 식은 독립 reviewer가 검토하기 전 scientific admission되지 않는다.

## 고정 목표와 현재 범위

- Target: 현재 generated rotating-frame collective cubic 모델, exact stored S와
  ω_j=width*xi_j, 공통 κ≥0, 기존 S_hi E_+≤1 domain.
- 기존 κ0 compiler의 κ>0 refusal은 새 route가 닫힐 때까지 그대로 둔다.
- local inclusion과 physical tolerance acceptance를 구별한다. 본 연구는 실행을
  승인하지 않고 next subnode contract를 reviewer에게 전달한다.
- source binding: HEAD 7e2873887ed6038b47b667c976435810851c5dba. 입력 hash는
  `RESEARCH_DECISION.json`의 source_identity에 있다. 사용자/다른 owner dirty 작업을 보존한다.

## 다음 smallest unit: A03-SCALAR-RATE-01

제안 interface: rate interval A, horizon interval T, integer r,d, absolute width
limit, preallocated scratch와 work를 받아 Q_{r,d} interval 또는 typed refusal을
반환한다. normalized damping a=2(q+1)κh가 exact product enclosure로 전달된다.
외부 arbitrary certificate/interval coefficient를 인증된 것으로 받아들이지 않는다.
private 생성자와 provenance로 원래 inputs를 묶거나, 입력 interval의 포함성
책임을 명시한 lower-level arithmetic API와 generated high-level API를 구분한다.

Endpoint hull은 `lo=Q(a_hi,t_lo).lo`, `hi=Q(a_lo,t_hi).hi`; t_lo=0은 exact0이다.
순서/유한성/비음수/shape/order 검증 후 kernel 호출을 charge한다. final outward
width≤원래 max_width일 때만 admitted다. 한 endpoint가 실패하면 다른 값으로
대체해 PASS하지 않는다. 기존 backward recurrence의 large-width에서만 optional
positive dyadic fallback을 사용한다.

초기 고정 cap은 core r,d≤64, dyadic fallback depth≤20, leaf≤4096,
attempted cell evaluations≤8191, work budget≤10^7 charged scalar categories다.
Fallback scratch와 table은 setup에 사전 할당한다. cap을 넘으면
`MomentWorkBudgetExceeded` 또는 `MomentWidthExceeded`다. 높은 order의 지원은
별도 cap/메모리 계약을 새로 검토하기 전 `UnsupportedMomentOrder`다.
단일 조회의 warmed allocation=0은 실제 allocator 양성 대조군 이후에만 인정한다.

Counter 의미도 먼저 고정한다. `validation_attempts`, `endpoint_attempts`,
`width_checks`, `fallback_attempts`, `cells_attempted`, `query_refusals`는 각 해당
단계에 들어가기 전에 1을 더한다. 별도의 `interval_primitive_attempts`는 실행한
Interval add/sub/mul/div, directed add/sub/mul/div, sqrt, interval endpoint exp
호출마다 1을 먼저 charge한다. min/max/정수loop/index/validation guard는 이
arithmetic field에 섞지 않는다. exp 내부instruction은 이 field의 단위가 아니다.
`coefficient_items`, `output_items`, `scratch_bytes`, `allocation_requests`도
독립 field다. refused attempt에서 이미 charge한 값을 되돌리지 않는다.
10^7 cap은 `interval_primitive_attempts`에 적용하고 wraparound 전 typed refusal한다.

제안 typed refusal은 `InvalidInput {field,index}`, `BindingMismatch {field}`,
`ScratchTooSmall {required,provided}`, `UnsupportedMomentOrder {required,supported}`,
`UnsupportedFrameCertificate {frame}`, `ArithmeticRefusal {stage,source}`,
`MomentWidthExceeded {width,limit}`, `MomentWorkBudgetExceeded {used,cap}`,
`TubeNotClosed {upper,limit}`, `ToleranceRejected {time,mode,required,allowed}`다.
이는 제안 이름이며 현재 enum이 아니다. caller scratch의 중간값은 바뀔 수 있지만
refusal에서 certified view를 노출하지 않고 source owner/accepted state는 보존한다.
성공·거절 경로의 formatting/String allocation도 allocation 계측 범위에 명시한다.

### Scalar fixture identity와 판별

아래 값은 `f64::from_bits`로 생성한다. literal→f64 이후 exact input을 유지한다.
reference는 각 binary64의 `as_integer_ratio`로 만든 exact Fraction interval Taylor
적분을 사용한다. 분수에서 intermediate float로 돌아가지 않는다.

| ID | 입력 | 예상 branch / 요구 결과 |
|---|---|---|
| S01 | κ=0 bits0000000000000000, h=0.5 bits3fe0000000000000, q=0, r=2,d=3, τ=1 | exact beta boundary 포함 |
| S02 | κ=minsub bits0000000000000001, h=0.5, q=0, r=d=0, τ=1 | exact positive 2κh를 zero damping으로 치환 금지; 포함 또는 typed width refusal |
| S03 | κ=0.1 bits3fb999999999999a, h=1 bits3ff0000000000000, q=2, r=0,d=1, τ=1 | exact 6κ가 rounded point와 다름; endpoint hull 포함 |
| S04 | κ=0.1, h=10 bits4024000000000000, q=0, r=1,d=0, τ=0.5 | exact normalized product의 branch crossing 포함 |
| S05 | κ=1e6 bits412e848000000000, h=1, q=0,r=d=0, τ=1 | large rate: narrow result 또는 width/work typed refusal, 유한한 넓은 interval을 narrow PASS로 표기 금지 |
| S06 | κ=max bits7fefffffffffffff, h=2 bits4000000000000000, q=0 | exact product arithmetic overflow typed refusal |
| S07 | κ=0.25 bits3fd0000000000000,h=0.5,q=1,r=0,d=1, T=[0.5,next_up(0.5)] | exact rate/horizon interval monotonic hull |
| S08 | κ=0.25,h=0.5,q=1,r=65,d=0 | UnsupportedMomentOrder(required65,supported64) |
| S09 | NaN bits7ff8000000000001, +Inf bits7ff0000000000000, negative κ bitsbfc0000000000000; inverted/NaN interval; wrong scratch size | 각 입력별 typed input/shape refusal, owner 보존, attempted work 유지 |
| S10 | S03에서 max_width=0 | 정확히 point임을 증명하지 못하면 width refusal; threshold 변경 금지 |

S01–S04/S07의 요청 max_width는 exact binary64 `2^-40`; S05는 `2^-35`다.
S02의 positivity가 반환 interval의 strict positive lower endpoint를 뜻하지는 않는다.
underflow 포함성과 strict positivity 증명은 다른 조건이다.

Fraction oracle 초기 exponential degree 128, 추가 degree 64씩, cap2048이다.
small/large interval rate 및 horizon endpoint 각각의 remainder를 exact rational로
계산한다. S05의 큰 음의 exponent는 alternating Taylor를 2e6차까지 전개하지
않고 exact 식 Q_00=(1-exp(-a*t))/a와 양의 exponential partial sum
`0≤exp(-x)≤1/(Σ_{k=0}^N x^k/k!)`를 조합한다. inverse-positive-series branch의
degree도 같은 128..2048 cap이다. 원래 scalar integrator와 다른 닫힌식 oracle다.
oracle bracket가 native interval 안에 완전히 포함되면 PASS, native와
분리되면 FALSIFIED, 겹치기만 하면 INCONCLUSIVE 후 oracle degree만 증가시킨다.
cap에서 INCONCLUSIVE는 HOLD이며 solver tolerance/width를 완화하지 않는다.
이 oracle는 formula 포함성을 판별하며 full packet accuracy를 뜻하지 않는다.

Allocation 측정은 같은 실행 binary/allocator에서 3개의 Vec clone positive control을
검출한 뒤, 사전 할당된 endpoint/fallback 조회의 successful/rejected paths를 각각
계측한다. positive control 실패는 ALLOCATION_UNAVAILABLE이고 0 allocation PASS가
아니다. 오류 String 생성 등 실패 경로에 allocation이 있다면 그 경로의 수치를
그대로 보고한다. VG-1 주장 범위와 일치시키고 성공 경로 밖 비용을 숨기지 않는다.

## 다음 local certificate unit의 필수 acceptance matrix

아직 stable representation/API가 없으므로 아래는 implementation READY 판정이 아니다.
새 source/config을 고정하고 독립 contract review를 통과한 뒤 다음만 실행한다.
기존 허용오차는 `rtol=1e-8` bits3e45798ee2308c3a,
`atol=1e-11` bits3da5fd7fe1796495로 유지한다. test gate의 norm은 mode complex
modulus와 returned mean modulus 각각에 대해 radius≤atol+rtol*|returned center|다.
그 gate는 inclusion 증명에 추가되는 acceptance 조건이며 두 개를 혼동하지 않는다.

| ID | 고정 입력 / 검사 | 필요한 증거 |
|---|---|---|
| L01 | n1,w1,xi0,width0,z=(1/4,1/8),χ=1/8,κ∈{0,minsub,1/4,1,1e6},h=1/64; t/h={0,1/4,1/2,3/4,1} | exact solution `z exp[-κt+iχ w^3 |z|² θκ(t)]`의 exact-rational series disks; κ0과 작은κ 포함, 큰κ는 finite 포함 또는 typed refusal |
| L02 | L01 χ=0,xi=3/4,width=1/8 | damped linear exact solution `z exp[(-κ+i width xi)t]`, 모든 시각 포함 |
| L03 | n3,w=(1/4,1/4,1/2),xi=(-3/4,1/4,1/2),z=((1/4,1/8),(-1/8,1/4),(1/8,-1/4)),width=1/8,χ=±1/8,κ∈{0,1/4,1,8},h=1/64,p2,Q2; t/h={0,1/4,1/2,3/4,1} | independent Fraction vector ODE Taylor with rigorous tail; every requested mode+mean disk and continuous residual cell witness |
| L04 | L03 weight[2]만 next_up(1/2), xi[0]=next_down(-1)을 각각 별도 arm으로 지정 | actual S와 spectral support를 사용. current constructor의 domain 밖이면 typed admission refusal. S=1/X=1로 치환 금지 |
| L05 | L03 η_j∈{0,2^-30}、small weight fixture w=(2^-40,1/2,1/2-2^-40) | initial uncertainty、weighted radiusとmarginal radiusの双方。S_hi E_+≤1を保持 |
| L06 | L03 h=0.1 exact bits3fb999999999999a; t=0.03 exact bits3f9eb851eb851eb8 | τ=t/hのinterval lift、実physical時刻の包含。local budgetが失敗したらtyped tolerance rejection |
| L07 | κ0,κ=minsub、および h=minsub; initial real=minsubを個別 armで指定 | exact-zeroとunderflowの区別。成功は中心と半径をexact referenceで包含、不能はtyped arithmetic refusal |
| L08 | L03のmutable input変更、candidate/descriptor/model/horizon/precision epochの混用 | private owner compile-failまたはruntime identity refusal。refusal後再query bit identityとwork増分 |
| L09 | L03 carrier metadata=0,1,1e6を変更、frame=Rotating | carrierがRHSに追加されないこと。非零Labはverified phase kernelがない限りtyped UnsupportedFrameCertificate |
| L10 | p8,Q10,D258,forcing degree774 のDW representative request | current cap64では明示的 UnsupportedMomentOrder。高order支持へ勝手に再分類しない |
| L11 | large candidate/domain overflow、NaN/Inf、wrong dimensions、rejected query後successful query | typed refusal、owner保存、attempted setup/certificate/output workの完全保持 |
| L12 | same binary allocator clone positive control=3、warmed successful/rejected queries | allocation実測。point queryのみを全setup/trajectory zero-allocationに拡張しない |

L04の二つのarmは互いに独立に実行する。L05のsmall weightsは厳密正でsum1、
η=2^-30のbitsは3e10000000000000である。L03のreference初期Taylor degree48、
step subdivisionは1,2,4,8,16の順、degree cap128とする。各substepで初期ballと
polynomial tubeの閉包をinterval/Fractionで検証し、閉じなければINCONCLUSIVE
HOLD。単なる高精度浮動結果をrigorous oracleと名付けない。

Local gate成功後の次の依存作業はκ>0のA02連結である。固定観測grid、stored
weighted seed、initial uncertainty、physical time lift、rejection transactionとwork
accumulationを別々に検証する。local testをmultiwindow testの代用にしない。

## Workと失敗分類

model/input reads, exact-rate interval operations, moment/kernel/fallback setup,
coefficient generation, homotopy products, continuous residual cells, norm/gain,
all requested reconstructions, carrier, bytes/allocations, rejected attemptsを記録する。
coarse attempt countsとarithmetic-operation countsを別fieldsにする。whole FLOPや
wall speedに変換しない。各commandのsource/tree/config/input/output hashとexit code,
最初の生ログを保持する。今回のunitでは数値command/test/buildはNOT_RUN。

失敗は INVALID_INPUT / IDENTITY_MISMATCH / UNSUPPORTED_ORDER_OR_FRAME /
OUTWARD_ARITHMETIC_REFUSAL / MOMENT_WIDTH_OR_WORK_REFUSAL / TUBE_NOT_CLOSED /
TOLERANCE_REJECTED / ORACLE_INCONCLUSIVE / ENVIRONMENT_FAILURE に分類する。
historical ledger/package hash不一致を新しい数値反例と混同しない。
