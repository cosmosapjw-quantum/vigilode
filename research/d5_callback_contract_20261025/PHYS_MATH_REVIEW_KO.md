# D5 물리·수학 보완 검토 — R6

검토자: next_contract_sources (실제 분담 agent).
검토 방법: 고정 계약·이론·gateway 직접 읽기, 손대수, 앞서 읽은 원전/기존 source 경계 대조.
한 번의 보완 검토 pass에서 구체적인 finding의 교정 부분만 추가 확인했다.

**현재 판정: PASS_SCOPED_DERIVATION_AND_SOURCE_BOUND.**
새 model의 도함수·Taylor 항·합 변화·단순체 논증, represented FD의 의미, source와 인터페이스 명제의 대응이 일치한다.
아래 4개 finding은 실제 문서/source 교정 뒤 닫았다. 수학적 blocking finding은 0이다.
Root가 실행한 Wolfram의 실제 source·raw·추출 JSON을 같은 pass에 합류해 대조했다.
12개 exact residual group의 스칼라 zero-check는 모두 true이며, exact sample은 위임 검토자의 손대수 값과 일치한다.
저장된 point/stencil 원자료, exact oracle source·보고서·로그 및 Rust 시험 로그도 같은 pass에 합류했다.
대표점·경계·전체 FD/refusal·actual wrong-J 기록의 손대수/source 대조와 데이터 identity 검토를 마쳤다.
이 판정은 R6 범위의 PASS이며, 별도 C9·게시·production gate를 대신하지 않는다.

이 R6 검토자의 신규 solver/test/compile/CAS/benchmark/NCP 실행 0, 원격 게시 0.
파일 읽기·review 문서 작성·identity 기록 외의 실행을 하지 않았다.
independent personnel=NOT_RUN; actual delegated agent review=RUN.

## 1. 직접 검토한 입력 identity

| 파일 | SHA256 |
|---|---|
| CONTRACT.md | 11b3a7e8a1ba721a950ccaa6df42ccb6b6804adb9912b9c971d8a7a2925d315a |
| PREREGISTRATION.md | fca5617f0daea740853323e2279013288996e625d7241900e6211e54ff194a76 |
| THEORY.md | 5b2ad00d527d74760f2302057d177552349b4ac9e85077d92f8d6a106376795f |
| source/gateway.rs | c4ae170271fdae67bab2edfddb16ea149bccfc77f60f59e161347cbcfee697ee |

입력 경로는 /workspace/scratch/6a9ff47f24f5/next_20261025/d5/ 아래다.
위 source는 실행 전 bit-domain 교정 이후 identity다.
evidence/preexecution_bit_domain_correction/RECEIPT.json은 이전 gateway의
3eef1557d87c4567b412f1c1feae0143ad022d9140ab14fba99e6bde8ba49d0f 및 이전 tests identity와
과학 실행 0을 보존한다. 이전 실패 가능성과 교정 이유를 지우지 않았다.

## 2. 제조 transfer/loss 모델의 손대수

상태를 z=(x,y)로 쓰면
\[
k(t)=k_0+\eta t^2,\qquad
f(t,z)=
\begin{pmatrix}
-\lambda x+k(t)y^2\\
\lambda x-\mu y-k(t)y^2
\end{pmatrix}.
\]
시간은 고정된 기준 시간으로 무차원화되고, 이 모델은 제조 입력이다.

### Jacobian과 시간 편미분

각 상태 성분을 직접 미분하면
\[
D_z f=
\begin{pmatrix}
-\lambda&2k(t)y\\
\lambda&-\mu-2k(t)y
\end{pmatrix},
\qquad
\partial_t f=
\begin{pmatrix}
2\eta t y^2\\
-2\eta t y^2
\end{pmatrix}.
\]
Root의 식과 일치한다. 고정값 lambda=3, mu=1, k0=2, eta=1/2에서
\[
J=
\begin{pmatrix}
-3&(4+t^2)y\\
3&-1-(4+t^2)y
\end{pmatrix},
\quad
\partial_t f=(t y^2,-t y^2).
\]

시간을 고정하고 상태 increment d=(d_x,d_y)를 대입하면 x의 항은 1차에서 끝나고
(y+d_y)^2=y^2+2y d_y+d_y^2이므로
\[
f(t,z+d)-f(t,z)-Jd=(k(t)d_y^2,-k(t)d_y^2).
\]
이는 fixed-time state Taylor의 정확한 2차 remainder다.
시간 increment까지 포함한 다변수 Taylor remainder로 확대하지 않는다.

### transfer 합과 단순체

\[
f_1+f_2=-\mu y,\qquad
(1,1)J=(0,-\mu),\qquad
\partial_t f_1+\partial_t f_2=0.
\]
Transfer 항은 상쇄되며 두 species 합은 loss 항만큼 감소한다.
mu>0에서 두 species 합 자체를 보존량이라고 부를 수는 없다.

lambda, mu, k0, eta>=0이면 x=0에서 f1=k(t)y^2>=0이고 y=0에서 f2=lambda*x>=0이다.
Polynomial vector field의 local uniqueness와 경계의 inward/tangent 방향으로 비음수 orthant가 불변이다.
그 안에서 d(x+y)/dt=-mu*y<=0이므로 x(0)+y(0)<=1이면 exact solution은 단순체를 벗어나지 않는다.
유한한 시간 구간의 이 compact set에서는 vector field가 유계이고 local solution이 이어진다.
이것은 exact manufactured ODE에 대한 논증이다. D5가 검사하는 전체 box [0,1]^2의 임의 모델 불변성,
수치 trajectory의 비음수성 또는 actual BASS 영역을 이 논증으로 주장하지 않는다.

### determinant와 해석 한계

직접 전개하면
\[
\det J=\lambda(\mu+2ky)-2\lambda ky=\lambda\mu,
\]
\[
\det(I-hJ)=(1+h\lambda)(1+h\mu+2hky)-2h^2\lambda ky
=1+h(\lambda+\mu+2ky)+h^2\lambda\mu.
\]
h>=0 및 위 비음수 조건에서 두 번째 determinant는 1 이상이다.
이는 해당 shifted 2×2 행렬의 exact nonsingularity 확인이다.
Determinant만으로 norm condition number, floating solve error 또는 inverse enclosure를 얻지는 않는다.
THEORY의 기존 “conditioning” 표현은 “sign/nonsingularity”로 교정돼 이 한계와 일치한다.

### 독립 수동 witness

t=1/2, x=1/4, y=3/4를 직접 대입하면
\[
k=17/8,\quad f=(57/128,-153/128),
\]
\[
J=\begin{pmatrix}-3&51/16\\3&-67/16\end{pmatrix},
\quad \partial_t f=(9/32,-9/32),
\quad J(1,-1)=(-99/16,115/16).
\]
합 f1+f2=-3/4와 detJ=3이 각각 독립적으로 맞는다.

같은 점에서 d=(1/8,-1/8)이면 상태 Taylor remainder는 (17/512,-17/512)다.
이 값들은 손대수 witness이며, 실행한 point grid의 관측치로 세지 않는다.

## 3. frozen dyadic grid와 represented FD

### 729 point 입력의 exact oracle 조건

세 축이 독립적으로 i/8, i=0..8인 9×9×9 grid에서 입력은 모두 dyadic rational이다.
t^2, y^2와 k(t)y^2의 공통 분모는 최대 8192이고,
이 고정 계수/box에서 numerator 크기는 binary64의 53-bit 유효 정밀도보다 훨씬 작다.
따라서 이 직접 polynomial 식·Jacobian·J(1,-1)에 exact Fraction equality를 요구하는 사전 기준은 합리적이다.
이 논증은 모든 임의 입력이나 다른 산술 형태의 callback에 적용하는 uniform accuracy 보장이 아니다.

### represented 점에서의 정확한 secant

t_minus와 t_plus를 실제 binary64 값으로 고정하고 d=t_plus-t_minus를 실수의 정확한 차로 두면
\[
\frac{f(t_+,z)-f(t_-,z)}{d}
=\bigl(\eta(t_++t_-)y^2,-\eta(t_++t_-)y^2\bigr).
\]
중앙 t의 도함수와 차이는
\[
\bigl(\eta(t_++t_--2t)y^2,-\eta(t_++t_--2t)y^2\bigr).
\]
이는 real secant에 대한 항등식이다. 실제 gateway는 Delta=fl(t_plus-t_minus)를 사용하므로
일반 경우에는 denominator rounding과 callback/subtraction/division rounding을 별도로 포함해야 한다.
현재 THEORY가 “in real arithmetic”과 실제 fl 연산을 나누고 estimate만 주장하는 것은 적절하다.

고정 49개 stencil은 t=i/8 (i=1..7), eps=2^-p (p=3..9)다.
두 점은 [0,1] 안에 있고 서로 다르며 정확히 대칭이다. 해당 dyadic 점과 Delta도 정확히 표현된다.
따라서 real secant는 exact partial_t f와 같다. 이 grid에 대한 64*epsilon*max(1,abs(reference))
gate는 실행 전 고정돼 있으며, 판정이 나온 뒤 tolerance를 바꿀 이유가 없다.

수동 예: t=1/2, eps=1/8이면 t_minus=3/8, t_plus=5/8, Delta=1/4.
x=1/4,y=3/4에서 두 f1 값의 차는 9/128이므로 secant는 9/32다.
경계 t=0/1 및 sub-ULP eps는 예상된 refusal control이고 실패를 성공으로 보정하지 않는다.

## 4. gateway와 인터페이스 명제의 대응

### Domain preflight와 finite bit 순서

교정된 BoxDomain의 finite_key는 finite binary64에서 숫자 순서를 보존한다.

- 양수는 sign bit를 뒤집으므로 magnitude의 exponent/fraction 순서가 그대로 유지된다.
- 음수는 전체 bits를 반전하여 숫자 순서와 반대인 magnitude 순서를 되돌린다.
- +0과 -0에만 같은 key를 부여한다. query 자체의 bits를 덮어쓰지 않는다.
- NaN/Inf는 is_finite의 선행 short-circuit에서 거절된다.
- subnormal도 같은 bit 순서에 속하므로 domain comparison이 FP rounding/DAZ 비교 의미에 의존하지 않는다.

finite interval endpoint 검증과 query 비교가 같은 key를 쓰므로 inverted bound를 허용하지 않고
signed zero는 numeric equality로 처리한다. 이는 새 numerical projection이 아니다.
D2의 기존 bit-domain 패턴을 재사용한 국소 교정이다.

before는 ready → domain → witness → checked charge 순서다.
따라서 domain 거절에서 실제 RHS/Jacobian 진입은 0이다.
FD는 stencil 산술 전에 witness를 확인하고 두 represented query 모두를 검사한 뒤에만 첫 RHS에 진입한다.
Invalid second query 때문에 첫 callback을 이미 소비하는 경로가 없다.

### Output, pair, 작업량과 실제 과학 state

eval_rhs와 eval_jacobian은 각자 독립 NaN scratch를 가진다.
Callback과 post witness가 성공하고 모든 항이 finite일 때만 owned array를 반환한다.
never-written slot은 NaN으로 남으므로 strict full-write 성공값으로 나갈 수 없다.
이것은 native fixed-sparsity/previous-matrix contract와 별개의 opt-in 의미다.

prepare_pair는 첫 RHS 결과를 local에 보존하고 Jacobian도 성공해야 Prepared를 반환한다.
반환 전에 둘 중 하나가 실패하면 caller의 이전 publication을 쓰는 reference가 없어 그 값은 보존된다.
Array/Prepared 반환 자체를 scientific Session acceptance와 동일시하지 않는다.
교정된 pair counter 설명처럼 pairs는 성공한 Prepared 반환 횟수이며,
caller assignment 또는 accepted solver step을 관측하지 않는다.

Callback count는 진입 전에 증가하므로 callback Err/처리된 panic에도 남는다.
Preflight refusal은 callback work가 아니다. recoverable Err에는 retry loop가 없고 호출자의 정책만 허용한다.
Post witness failure는 callback status보다 앞서 판정한다.
Finite 검사는 성공 status의 출력에 적용하며, recoverable Err의 버려진 scratch를 별도의 성공 출력으로 분류하지 않는다.

D5 Gate가 Session을 재시작하거나 M을 늘리는 code는 없다.
기존 Session의 begin/abort/finish를 유지한다는 조건 아래 outer attempts<=M 논증은 보존된다.
RHS/Jacobian 수, successful pair 수, accepted step 수는 서로 다른 계수다.

### Witness, FP와 panic의 조건

Epoch equality만으로 model immutability를 증명하지 않는다.
최신 THEORY/CONTRACT는 honest read-only probe, model no-ABA/no-unobserved-mutation,
checked arithmetic 중 관측되지 않는 FP 변화/FP-state ABA가 없다는 가정을 명시한다.
Pre/post 두 번의 관측만으로 실제 사이의 모든 상태를 연속 감시했다고 주장하지 않는다.

Gateway의 forbid(unsafe_code)와 외부 연구 fixture의 read-only MXCSR instruction은 서로 다른 경계다.
Gateway source에 unsafe를 추가하지 않았지만 dependency 전체가 unsafe-free라고 주장하지 않는다.
MXCSR 관측은 compiler flags의 증명도 아니다.

Callback/probe의 명시적으로 잡힌 Rust unwind에 대해서만 typed failure를 반환하며,
필요한 destructor와 panic-payload drop은 정상 반환해야 한다.
panic=abort, process abort, nonreturning callback, foreign exception, double panic, allocator/stack failure까지
Result로 바꾼다는 주장은 없다. 교정된 CONTRACT는 THEORY의 이 전제와 일치한다.
Captured external effect 및 panic hook의 효과는 owned scratch 폐기로 되돌려지지 않는다.

## 5. Pointwise 성공과 진짜 Jacobian의 구별

J_bad=J+I는 모든 frozen point에서 유한한 full-write 행렬이다.
게이트가 성공해도 analytic J와 diagonal이 다르다.
t=x=y=0에서
\[
J=\begin{pmatrix}-3&0\\3&-1\end{pmatrix},
\quad
J_{\rm bad}=\begin{pmatrix}-2&0\\3&0\end{pmatrix}.
\]
이 한 점만으로도 “finite full-write이면 derivative가 맞다”라는 좁은 주장은 반박된다.
Transpose 역시 이 점에서 off-diagonal을 바꾸므로 비자명한 검출 입력이 존재한다.
이들은 실행한 mutation count가 아니라 사전 손대수 witness다.

General dense-domain authority에는 전체 구간/상태 영역의 residual·Jacobian 상계와 tube closure가 필요하다.
앞서 직접 읽은 [Neumaier manuscript §§3–4](https://arnold-neumaier.at/ms/ode.pdf)의 전제는
point sample과 finite array만으로 충족되지 않는다.
기존 immutable model provider와 UnsupportedUnboundModel 경계를 유지한 것은 적절하다.

## 6. 기존 패턴 재사용과 finding 처리

C21H는 RPC response 검사 뒤 owned 값 게시, C21I는 model-specific domain/provider guard와
성공값의 독립 copy를 이미 구현했다. D2는 pre-RHS guard와 NaN scratch,
D4는 owned journal/pending/ACK 및 기존 Session의 attempt 계약을 이미 갖는다.
D5의 기여는 direct in-process generic strict full-write와 pair/stencil 경계를 source에 묶는 좁은 구성이다.
첫 staging/domain 발명, 새 norm-only 결과 또는 strong no-go/최초성으로 포장하지 않는다.

| finding | 최초 관측 | 조치 | 상태 |
|---|---|---|---|
| R6-01 determinant 해석 | determinant를 conditioning check라고 부름 | sign/nonsingularity로 좁힘 | CLOSED_READBACK |
| R6-02 destructor 조건 | catchable unwind와 destructor 반환 조건이 섞임 | callbacks/probes의 caught unwind와 destructor/payload normal return 분리 | CLOSED_READBACK |
| R6-03 FP 이전 domain 비교 | generic probe 이전의 FP-dependent comparison 전제 미결속 | finite bit-key ordering과 원본 보존; read-only/no-FP-ABA 가정 명시 | CLOSED_SOURCE_AND_DOC_READBACK |
| R6-04 pair counter 의미 | pair publication이 scientific publication으로 오독 가능 | 성공 Prepared 반환으로 명시 | CLOSED_READBACK |

변경에 대한 새 numerical run을 reviewer가 수행하지 않았다.
이 finding을 닫기 위해 필요한 source/문구만 같은 pass에서 다시 읽었고 완료 과학을 재실행하지 않았다.

## 7. 저장된 결과 합류 상태

### Root의 단일 Wolfram 실행: 실제 출력 대조 완료

아래 3개 파일의 전체 내용을 직접 읽었다. 검토자가 CAS를 추가 실행하지 않았다.

| 실제 입력·결과 파일 | SHA256 |
|---|---|
| evidence/DERIVATION.wl | 5d045af0e8ede514c36dabafa09707be1197301db27e1365264460a11b97b98d |
| evidence/WOLFRAM_RESULT.json | ae9aee06de85128bae42566301a35632ebaf01d671f67e6cf7c8b3f2ca3f5303 |
| evidence/WOLFRAM_CHECK.json | 6c80ad9a33be47262bdd82c95f2e53ad076a8c0deefad3073534d37fad0b98b3 |

DERIVATION.wl은 Jacobian, partial_t, sum loss, x/y 경계식, state remainder,
detJ, det(I-hJ), represented secant, symmetric secant, J(1,-1), wrong-J deviation의
12개 residual group을 만들고, 각 residual의 Flatten된 scalar마다 TrueQ[...==0]를 확인한다.
실제 raw가 포함한 exported JSON과 WOLFRAM_CHECK.json은 12개 true, all_zero=true 및
exact sample {{57/128,-153/128},{{-3,51/16},{3,-67/16}},{9/32,-9/32}}에서 일치한다.
이 sample은 §2의 독립 손대수와 같다. 출력에는 residual 배열 자체가 없고 Boolean zero-check가 저장돼 있으므로,
“저장된 residual 배열을 재계산했다”라고 확대하지 않는다.

기록된 kernel은 15.0.1 for Linux x86 (64-bit) (July 2, 2026)이다.
원자료의 kernel wall_seconds=0.035829, cpu_seconds=0.03582899999999967을 그대로 보존한다.
이는 root의 신규 CAS 1회에 대한 kernel 측정값이며 connector 전체 경과 시간 또는 solver speedup이 아니다.
이 R6 검토자의 신규 CAS 실행은 0이다.

결과를 해석할 때 다음 조건은 유지한다.

- represented/symmetric secant의 rational identity는 각각 tp != tm 및 eps != 0을 전제로 한다. 실제 gateway는 유한 양수 represented denominator와 두 구분되는 점을 preflight한다. CAS 항등식이 일반 binary64 계산의 오차까지 없애지는 않는다.
- x/y boundary zero-check는 경계에서의 식을 확인한다. 비음성 계수, local Lipschitz, inward boundary 및 total-loss를 결합하는 §2의 단순체 불변성 논증은 별도로 필요하다.
- det(I-hJ)의 zero-check는 전개식의 일치다. h와 계수가 비음수일 때의 부호 논증을 더해야 nonsingularity 결론을 얻으며, conditioning bound는 주어지지 않는다.
- wrong_J_plus_I=true는 (J_bad-J)(1,-1)=(1,-1)이라는 알려진 비영 오차를 확인한다. J_bad가 올바른 도함수라는 뜻이 아니다.
- 이 기호 계산은 callback epoch의 정직성, full-write 계약 준수, 외부 부작용 rollback, native production admission 또는 일반 dense certificate를 입증하지 않는다.

### Point/stencil 원자료: 같은 pass의 마지막 합류 검토 완료

실제 points.jsonl의 794행·170379바이트와 SHA256을 읽기/identity 기록으로 확인했다.
REVIEW.json의 input identity가 이 파일과 일치하며, REVIEW.json과 exact_saved_point_oracle.log의 바이트 해시도 같다.
source/probes.rs, review_points.py, source/session_tests.rs를 직접 읽었고 실행하지 않았다.
초기 CONTRACT/THEORY/PREREGISTRATION/gateway identity는 §1과 그대로 일치했다.

| 실제 입력·결과 파일 | SHA256 |
|---|---|
| evidence/points.jsonl | cb64d9455289f183363749289038d6f250bf63e0ab95ea94cd51defa88d2ba21 |
| REVIEW.json | 115e3aeee8424c2e906a7ec8cad4aef3691c2716351bf754817abaa8d54dc1f7 |
| evidence/exact_saved_point_oracle.log | 115e3aeee8424c2e906a7ec8cad4aef3691c2716351bf754817abaa8d54dc1f7 |
| source/probes.rs | 1775a83dbb02fd986ac9913297f5f8e1f3ed4d67b83d43e2361c7c9b72eb8669 |
| review_points.py | 6014b5a11d179d7808f42efb30b823329682842252f3d3581eee74e4492b9c16 |
| source/session_tests.rs | b6f54cedcf623e004e7985e7ba42c96f800b0460a57d677d9d4633745c0ee23c |
| evidence/new_gateway_contracts.log | f5377e20e0fe2424cb2f62c5ccef6e6ff824f85b4a1afdc3df9235b552ae5a90 |
| evidence/new_session_composition.log | fe32466f8065d235b7d0e2a63e63cee4b2db3e2a5e00492c4f2db73b1db97729 |

원자료 대조는 729개 point 각각의 oracle를 다시 실행한 것이 아니다.
representative/boundary/symmetric-J point를 직접 읽고, 49개 FD·14개 refusal·1개 actual wrong-J·summary의 원문을 모두 읽었다.
전체 exact 비교 수치는 source를 읽어 의미를 확인한 뒤 이미 저장된 oracle 결과에 귀속한다.

| 직접 읽은 point | 실제 raw의 핵심 값 | 독립 손대수와 주장 범위 |
|---|---|---|
| i=[0,0,0] | RHS=(0,0), J=((-3,0),(3,-1)), Jv=(-3,4), mass_residual=0 | 최초 경계 입력과 비대칭 Jacobian witness |
| i=[4,2,6] | RHS=(0.4453125,-1.1953125), J=((-3,3.1875),(3,-4.1875)), Jv=(-6.1875,7.1875), mass_residual=0 | §2의 exact sample 및 Jv와 동일; RHS 합은 -0.75 |
| i=[0,8,0] | RHS=(-3,3), mass_residual=0 | y=0에서 y'=3>=0; x+y의 변화는 0 |
| i=[8,0,8] | RHS=(2.5,-3.5), J=((-3,5),(3,-6)), mass_residual=0 | x=0에서 x'=2.5>=0; RHS 합은 -1 |
| i=[8,8,8] | RHS=(-0.5,-0.5), Jv=(-8,9), mass_residual=0 | box의 위쪽 corner; x+y=2이므로 초기 단순체 x+y<=1의 궤적 표본으로 세지 않음 |

mass_residual의 source 정의는 rhs[0]+rhs[1]+y이다. 따라서 0이라는 값은 합 변화가 -y임을 뜻하며,
두 성분 합 자체가 보존된다는 뜻이 아니다. 위 표와 §2의 수식이 이 정의에 맞는다.

저장된 oracle는 binary64 값을 Fraction.from_float로 exact rational에 옮기고,
Rust 평가 순서와 다른 전개식 및 stoichiometric 표현으로 RHS/J/Jv를 비교한다.
point당 2 RHS+4 J+2 Jv+1 mass residual=9개 exact 값, 총 6561개 비교가 모두 같아 mismatch=0으로 완료됐다.
비교 assertion 8613은 값만의 수가 아니다. source상 point당 11개×729, point cardinality 1개,
FD당 11개×49, FD cardinality 1개, wrong-J 7개, input refusal 14개, FD refusal 21개,
summary 11개를 포함하여 8613이 된다. 출력의 exact-zero mismatch와 max_point_error=0은 이 고정 grid에 한정한다.

C9가 찾은 원본 oracle의 zip() 단축 위험은 별도 보완 receipt와 함께 해석한다.
원본 review_points.py의 zip 비교는 배열 길이가 잘못 짧아졌을 때 비교 수를 줄일 수 있으므로,
원본 REVIEW.json 하나만으로 일반 malformed schema의 거절을 주장하지 않는다.
Root가 같은 raw에 대해 수행한 별도 schema 검토의 SCHEMA_REVIEW.json을 직접 읽었고,
그 receipt 및 additive_saved_schema.log의 SHA256은
780ae258fea1f182402eed5f5fe65507afd702c87760c61483e62e6a104d1e06으로 같다.
기록된 validator identity는 cf56eac2fb5715dc12d0504627b833a80a2f3f3f30ed4c48557924ec22deaf23이다.
이 receipt는 같은 794행/170379바이트 raw의 exact 9×9×9 point/7×7 FD key coverage,
실제 RHS 1458/J 2916/Jv 1458/mass residual 729개=6561 값과 FD estimate 98개,
duplicate JSON key=0 및 세 가지 malformed control의 거절을 보고한다.
Root의 이 보완 검토는 model/callback evaluation=0을 기록하며 원본 oracle/result를 보존했다.
최종 근거는 원본 exact 비교 결과와 이 schema receipt의 합성이다.
R6는 receipt와 raw/original-result hash 불변만 읽기 확인했으며 validator 또는 oracle를 다시 실행하지 않았다.

49개 FD는 모두 x=1/4,y=3/4에서 대칭 dyadic 점을 사용한다.
대표 t=1/2,power=3의 raw는 minus=3/8, plus=5/8, denominator=1/4,
estimate=(0.28125,-0.28125)=(9/32,-9/32)이며 손대수와 같다.
첫 stencil은 minus=0을, 마지막 t=7/8,power=3 stencil은 plus=1을 포함하여 유효 경계까지 조사했다.
저장된 oracle의 max absolute error=0, 사전 고정 gate=1.4210854715202004e-14가 유지됐다.
이는 이 49개 대칭 dyadic 입력의 결과이며 일반 FD accuracy certificate가 아니다.

input refusal 7개는 음수/상한 초과 t·state 및 NaN/Inf에서 모두 OutsideDomain,
RHS/J/pairs=0이다. FD refusal 7개 중 left/right는 OutsideDomain, collapse/zero/negative/infinite/nan은
InvalidDifferenceStep이며 모두 RHS=0이다. left/right/collapse만 pre-arithmetic witness를 1회 실행했고
나머지는 0회다. 이 기록은 거절된 query를 바꿔 callback에 넣었다는 흔적을 보이지 않는다.

actual finite_wrong_j 행은 위 t=1/2,x=1/4,y=3/4에서 J_bad=((-2,51/16),(3,-51/16))을 기록하고 admitted=true다.
올바른 J와의 차는 정확히 I다. source는 실제 full-write J+I callback을 gateway에 1회 통과시켰다.
별도 semantic mutation은 저장된 true-J를 transpose/양쪽 off-diagonal 부호 반전/J+I로 해석한 대조이며,
compiled Rust source mutant를 729회 실행한 기록이 아니다.
검출은 각각 720/729, 729/729, 729/729다. transpose 미검출 9행 i=[0,ix,6]을 실제 읽었으며,
t=0,y=3/4에서 J=((-3,3),(3,-4))가 대칭이므로 같은 행렬이 되는 경우다.
이는 사전 “각 mutation이 적어도 한 점에서 실패” 기준과 맞는다.

callback 계수의 source상 분해도 실제 summary와 일치한다.
RHS=729+49×2+1=828, J=729+1=730,
witness=729×4+49×5+4+3=3168이다. forbidden_callback_entries=0은 이 고정 corpus의 관측이다.
Gateway의 일반 조건부 안전성 논증은 §4–5에 두며 이 유한 관측만으로 모든 callback의 정직성 또는 종료를 주장하지 않는다.

### 저장된 Rust 시험과 synthetic Session 경계

실제 로그는 gateway 30 passed, Session composition 5 passed이며 failed/ignored는 각각 0이다.
이 검토자는 이를 재실행하지 않았다. 35/35는 새 계약 시험의 저장된 결과이고 과거 D2–D4 시험의 재실행 수가 아니다.
Session source의 accepted-prefix 시험은 unchanged candidate와 exact zero defect로 만든 synthetic admission fixture다.
코드 주석도 reversible-exchange RHS를 적분한 상태라고 주장하지 않는다고 명시한다.
따라서 보존된 accepted=1, t=0.125는 이후 callback failure에서 Session snapshot이 유지된다는 계약의 근거이며
이 제조 비선형 모델의 trajectory accuracy 근거가 아니다. 실제 새 ODE trajectory는 0이다.

현재 R6 판정은 수학 유도/가정/source correspondence와 저장된 symbolic/point/FD/contract 근거의 범위 일치에 한정한 PASS_SCOPED다.
이 마지막 합류 검토에서 새 blocking finding은 없었다. 추가 CAS, solver, oracle, grid 또는 test 실행은 0이다.
C9 코드·그림 검토, 게시/백업 readback 및 production admission은 각각의 실제 근거를 필요로 한다.
global completion=null, production=HOLD, actual BASS/general dense/durable/native live-owner gate는 유지한다.

