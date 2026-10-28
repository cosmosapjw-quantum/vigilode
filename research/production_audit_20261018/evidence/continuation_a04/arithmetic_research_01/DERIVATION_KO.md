# A04 현재 산술 경로와 T2 꼬리 부등식

이 문서는 현재 `leja_action.rs`의 위임받은 읽기 전용 연구 결과다.
Rust 구현과 기존 증거를 바꾸지 않는다. source byte identity와 exact
계산 결과는 인접 script 및 result/run record에 결속한다.

## T2의 정확한 함의

L >= 0, v_p=L^p e_col/p!, C >= ||L||_infty,
q=C/(N+1)<1이라 하자. 모든 r>=1에 대해

    ||v_(N+r)||_infty <= q^r ||v_N||_infty.

이는 매 단계의 분모 N+r가 N+1 이상이고 induced infinity norm의
submultiplicativity가 성립하기 때문이다. 비음수 꼬리 벡터 T_N에 대해

    ||T_N||_infty <= ||v_N||_infty q/(1-q),
    0 <= (T_N)_i <= ||v_N||_infty q/(1-q)  (각 i).

upper interval vector v_N^+와 upward C^+를 사용하면
max_i(v_N^+)_i 및 q^+/(1-q^+)를 outward 계산한 동일 scalar가 모든
component의 상한이다. 현재 source 1156–1163행이 바로 이 형태다.
code의 small-term 조건은 종료를 고르는 조건이고, 실제 포함성은 q<1과
이 기하급수 꼬리 상한에서 나온다.

역사적 `LEJA_TOTAL_BOUND.md` 110–114행은 올바른 1-norm 부등식에서
componentwise T_N <= v_N q/(1-q)를 결론 낸다. norm 부등식으로부터
이 vector 부등식은 나오지 않는다. 4x4 lower shift, e0, N=2이면
v_2=(0,0,1/2,0), v_3=(0,0,0,1/6), v_4=0, C=1, q=1/3이다.
역사적 마지막 component 상한은 0이며 실제 꼬리는 1/6이다. 올바른
uniform 상한은 (1/2)*(1/2)=1/4로 모든 component를 감싼다.

따라서 그 역사적 componentwise lemma만 `SUPERSEDED`다. 동일 문서
120–122행의 uniform infinity-norm recipe 및 현재 구현의 수식은 이
반례로 반박되지 않는다. 이 예는 native 종료 규칙이 N=2를 선택한다는
주장도 아니다. 과거 원본은 그대로 보존한다.

## 현재 total-error 분해와 코드 대응

대상은 represented finite symmetric A, verified Gershgorin spectrum,
represented h>=0 및 represented input vectors/scales의 fused phi action이다.
외부 입력 변환 오차는 이 raw API가 자동으로 인증하지 않는다.

    X=(hA-cI)/gamma, spec(X) subset [-2,2],
    rhat_(l+1)=(X-xi_l I) rhat_l + delta_(l+1),
    |d*_lk-dmid_lk|<=rad_lk.

matvec replay 678–700행은 exact represented A*rhat를 감싼다.
recurrence replay 706–725행의 composition은 h, center, scale, node
산술의 additive underflow까지 포함한다. local defect는 actual rounded
rhat와 그 interval의 endpoint 거리이며 2237–2262행에서 적용한다.
scaled outward l2는 658–673행이다. 이미 검토된 recurrence slice와
현재 byte가 같은 setup/report code를 반복 구현할 필요가 없다.

S-T1에 따라

    T_(i,m,k)=sum_(l=i+1)^m (|dmid_lk|+rad_lk) B[i+1][l],
    E3=sum_k |s_k| sum_(i=0)^(m-1) T_(i,m,k)||delta_(i+1)||2,
    E2=sum_k |s_k| sum_(l=0)^m rad_lk||rhat_l||2.

B[a][b]는 [a,b)의 실제 연속 suffix product 상한이다. 2327–2354행은
올바른 suffix와 coefficient-radius 교차항을 포함한다. 분해
d*r* - dmid*rhat=(d*-dmid)rhat+d*(r*-rhat)가 E2/E3를 정당화한다.

실제 enlarged support의 upper endpoint를 u라고 하면 derivative factor는
exp(max(0,u))이다. scalar/T1 code가 이 factor를 포함하므로 원래
support가 음수라는 이유로 이를 증명에서 제거하면 안 된다.

    E1=exp(max(0,u))*M_(m+1)*gamma^(m+1)
       *sum_k |s_k| ||w_k||2/(m+1+k)!.

2283–2304행이 E1이다. scalar branch는 b-a와 phi derivative 상한으로
center 선택 오차를 더한다(2080–2107행). sum replay 785–808행 및
output fusion 815–860행은 represented dmid*rhat 합과 실제 반환 fused
값의 거리를 직접 감싼다. 이를 E4라 하면

    ||F_true-F_returned||2 <= E1+E2+E3+E4.

이것은 코드에 대응하는 조건부 proof map이다. 기존 nearest-rounded
dense reference 비교만으로 새 total-authority admission을 대신하지 않는다.

## 남은 실제 의무와 다음 실행

현재 status 946–971행은 의도대로 항상 EstimateOnly다. A04 total은 HOLD다.
다음 단위는 `READY_FOR_BOUNDED_PROOF_ORACLE_REPAIR`다. 먼저 수식을 바꾸지
않는 독립 coefficient/fused-output oracle을 고정하고 실행한다.

1. T2의 잘못된 vector lemma를 이 문서의 norm proof로 대체하는 현재-use
   binding을 검토한다. 역사 문서를 고쳐 오류를 숨기지 않는다.
2. coefficient oracle은 Opitz engine과 다른 표현을 사용한다. exact
   x_i=c+gamma*xi_i에 대해 complete homogeneous polynomial h_r를 써서

       d*_(j,k)=gamma^j sum_(r>=0) h_r(x_0,...,x_j)/(r+j+k)!.

   R=max|x_i|이면 |h_r|<=binom(r+j,j)R^r다. r>M 꼬리는 첫 절대항
   binom(M+1+j,j)R^(M+1)/(M+1+j+k)!에 1/(1-R/(M+2))를 곱해
   감싼다(M+2>R). 다시 |gamma|^j를 곱한다. 이 식은 confluent nodes도
   포함하며 Fraction으로 실제 coefficient radius를 slack 없이 검사한다.
3. fused oracle은 exact Fraction matrix Taylor

       F=sum_k s_k sum_(q>=0) (hA)^q w_k/(q+k)!

   를 사용한다. B=||hA||_infty, W_k=||w_k||_infty이면 차수 M 뒤 꼬리는
   sum_k |s_k| W_k B^(M+1)/(M+1+k)! /(1-B/(M+2+k))로 감싼다.
   이 식을 사용하기 전에 **모든 active k에서 M+2+k>B**를 exact
   Fraction 비교로 assert한다. active란 |s_k|W_k>0인 열이다. q=M+1인
   첫 생략항 이후의 절대항 비는 B/(q+k+1)<=B/(M+2+k)<1이므로 이
   기하급수 remainder가 유효하다. 비활성 열의 꼬리는 정확히 0이다.
   B=0이면 M>=0에서 꼬리도 정확히 0이며 0^0을 계산하지 않는다.
4. fixed fresh fixtures는 scalar/zero/newton, normal/subnormal input,
   subnormal operator product, cancellation, SameVector signed scales와
   Distinct inputs, successful/degree-cap report를 모두 포함한다. degree cap에
   도달해 예산을 못 맞춘 경로는 `Ok(report)` 및 `converged=false`이며
   typed nonfinite/overflow `Err`와 구별한다. 기존 physical budget을 유지한다.
5. forged-cache/report compile-fail, failed-call last-report/work preservation,
   cold/warm setup receipt parity를 기존 source identity가 유지되는 한
   재사용한다. 새 oracle 및 변경된 경로만 표적 실행한다. 새 allocation
   실행은 같은 thread/계측 방식의 positive control을 먼저 검출하고 warmed
   success에서 0을 확인한다. calibration이 실패하면 결과는
   `ALLOCATION_UNAVAILABLE`이며 zero-allocation PASS를 내지 않는다.
6. formula/code/oracle diff와 원시 결과를 별도 독립 reviewer에 보낸다.
   모든 실제 산술 전제가 닫힌 범위에만 이후 opt-in authority를 부여한다.
   current helper/report의 이름만으로 certificate를 인정하지 않는다.

새 oracle 구현의 기본 allowed paths는 새 core 통합 test, 새 Fraction
oracle, 이 evidence 하위 문서다. 실제 under-enclosure가 발생한 경우에만
원인을 분류해 필요한 Leja arithmetic repair를 좁혀 승인한다. 현재 연구는
full-A04 admission, routing/default 변경, speed 또는 production 승격을
허가하지 않는다.

## Oracle 포함성의 세 갈래 판정

coefficient와 fused-output 모두 정확한 rational interval
[l_i,u_i]에 실제 대상이 포함되도록 급수와 엄밀한 remainder로 구성한다.
반환 binary64 중심 y_i와 반경 b도 bits에서 exact Fraction으로 복원한다.
모든 b는 finite/nonnegative여야 한다. 그렇지 않으면 `INVALID_REPORT`다.

    a_i = max(|l_i-y_i|, |u_i-y_i|),
    z_i = max(0, l_i-y_i, y_i-u_i),
    U = sum_i a_i^2, L = sum_i z_i^2.

`U<=b^2`면 `PASS`, `L>b^2`면 `FALSIFIED`다. 나머지는
`INCONCLUSIVE`다. coefficient 검사는 dimension 1로 같은 규칙을 쓴다.
U가 b²를 넘었다는 사실만으로 실제 반례라고 판정하지 않는다.

INCONCLUSIVE에서만 Fraction 급수 차수 M을 64,128,256,512,1024 순서로
올린다. 입력/degree cap/budget/반환값/반경/인증 조건은 바꾸지 않는다.
1024에서 계속 불확정이면 그 상태로 보존하고 A04는 HOLD다. 차수별로
각 coefficient의 M+2>R 및 각 active fused k의 M+2+k>B를 assert한다.
조건이 안 맞는 순서에서는 remainder를 만들지 않고 `ORACLE_DOMAIN_UNMET`를
기록한다. 다음 사전등록된 차수로만 진행하며 cap까지 조건이 없으면 HOLD다.
oracle PASS는 그 fixture의 포함성 증거이며 full-A04 admission과 구별한다.

## 실제 binary64 입력의 의미와 고정 fixture

`RESEARCH_DECISION.json::next_action.frozen_oracle_contract`가 입력의
machine-readable 원본이다. fixture ID, shared bit arrays, h/budget bits,
degree cap, expected branch/Ok/Err와 refinement schedule을 실행 전에 고정한다.
모든 bit string은 IEEE binary64 u64 hex encoding이며 decimal 재파싱을 하지
않는다. qNaN/Inf fixture는 지정된 bits 그대로 refusal 입력으로만 사용한다.

coefficient reference는 받은 c_bits, gamma_bits, xi_bits를 각각 exact
rational로 복원한 뒤 **rational x_i=Q(c)+Q(gamma)*Q(xi_i)**를 만든다.
x_i를 f64로 먼저 계산하거나 한 번 더 반올림하지 않는다. standalone
coefficient fixtures의 xi는 manifest의 명시 배열이다. action의 derived
c/gamma 및 실제 사용 xi는 native exporter가 raw bits로 기록하며 oracle은
그 bits를 사용한다. derived 값은 새 input을 고르는 자유도가 아니다.

fused reference도 Q(h)*Q(A_ij), Q(scales), Q(vector)를 exact rational로
연산한다. SameVector에서 scales*vector를 먼저 f64로 만들어 Distinct
input으로 바꾸지 않는다. coefficient 포함성 판정과 fused-output 포함성
판정을 각각 저장하고, 어느 쪽의 INCONCLUSIVE도 FALSIFIED로 바꾸지 않는다.

Err fixture는 oracle numeric PASS 대상이 아니다. 지정된 error family와
last successful report 보존, attempted work 보존을 검사한다. budget 미달
report는 유한 output/반경에 대해 oracle triage를 수행하지만 solver의
`converged=false`를 보존한다. warmed zero-allocation 판정은 성공 경로 중
manifest가 지정한 동일 capacity/cache 조건에서 calibration 이후에만 한다.
