# 현재 target을 보존하는 정규화 jet 이식

## 기존 결과에서 그대로 가져온 것

Loop11의 positive Hermitian metric H에서 dissipative한 **현재** J에 대한
R0=(I−γ0 hJ)^−1, K=h R0 J, ||R0||H≤1, ||K||H≤1/γ0 및
δ 기하급수의 꼬리 경계는 source audit에서 조건부로 수용했다. 이번에는 그
520개 exact inequalities나 네 Fourier trajectory run을 다시 실행하지 않는다.
archive 4550경로 hash 확인은 원본 식별일 뿐 이 수학의 재증명이 아니다.

## 새 computational form: 정규화와 인증의 분리

z=(γ−γ0)/γ0, T=γ0 hR0J=R0−I라 두면

    (I−γhJ)^−1B = (I−zT)^−1 R0B,
    W0=R0B, W(k+1)=R0Wk−Wk,
    U_p(z)=Σ(k=0..p) z^k Wk.

이 등식은 P=I−γ0hJ와 J가 가환한다는 사실을 사용한다. dissipativity에서
||T||H≤1이므로 ||Wk||H≤||R0B||H. 원래 K^k의 표현은 γ0가 작을 때
γ0^−k를 만들 수 있지만 정규화 recurrence에는 이 불필요한 크기가 없다.
이는 정확 산술의 scale 안정성이고, f64 recurrence가 모든 parameter에서
상대적으로 정확하다는 주장은 아니다. 특히 R0≈I이면 뺄셈 cancellation이
남는다. 실제 stored candidate에 대한 별도 residual 검사가 이 위험을 감싼다.

새 native 범위에서는 H=I만 지원한다. J의 대칭부 S=(J+J^T)/2에 대해
모든 i의 Sii+Σ(j≠i)|Sij|를 outward upper로 계산하고 ≤0일 때만
J+J^T≤0를 인정한다. 이 충분조건에 실패해도 J가 비dissipative라는 뜻은
아니다. 고유값의 실수부만 음수인 nonnormal 행렬은 이 계약을 대신하지 못한다.

실제 유한 f64 후보 U를 정확한 binary rational 값으로 해석하고

    r=B−(I−γhJ)U

를 입력 J,h,γ,B에서 interval 연산으로 다시 만든다. LU의 backward error,
recurrence truncation, Horner와 ratio rounding을 각각 추측해서 더하지 않는다.
그 영향이 stored U에 포함되어 있으므로 독립 exact target residual에 남는다.
||e||2≤||r||2≤||r||1이고, 각 RHS column마다 이 upper≤abs_tol일 때만
Certified다. 이는 ODE local/global error나 dense trajectory certificate가 아니다.

## 비용과 병렬화의 실제 범위

n차원, m개 target, r개 **제공된** RHS columns, d=p+1이라 두자.
독립성/수치 rank를 자동으로 인증하거나 줄이지 않는다. 공유 방식은 중심 LU 1회,
dr개 scalar RHS solves, d의 recurrence depth, mprn개 Horner coefficient
multiply-adds, mrn²개 directed residual matrix terms가 필요하다. 최소 jet
저장은 drn개, 출력은 mrn개다. 실제 allocator/LU workspace와 certificate
report memory는 별도다. 단순 m회 LU 비교만으로 속도 승격을 하면 안 된다.

    C_jet = F(n)+d r S(n)+m p r n E + m r n² D + C_output + C_storage,
    C_repeatLU = m F(n)+m r S(n)+m r n² D + C_output.

F/S/E/D는 서로 다른 측정 단위다. D=일반 multiply 한 번으로 놓지 않는다.
동일 tolerance와 target certificate를 만족하는 Schur reuse, multishift block
Krylov, existing common-γ reuse가 경쟁 baseline이다. 위 식은 비용 모델이며
이번 작업에서 wall-clock cost를 관측하거나 calibrated selector를 만들지 않는다.

Horner와 각 target residual은 jet이 준비된 뒤 서로 독립이다. RHS columns도
독립적으로 처리할 수 있다. W(k+1)는 Wk를 요구하므로 d 깊이는 남는다.
미래 Rosenbrock stage의 nonlinear RHS가 아직 계산되지 않았다면 이 batch
primitive만으로 stage 인과 의존성을 없앨 수 없다. Common γ=동일 center의
기존 stage에는 새로운 factorization 절감이 없다.

## degree/cluster의 prospective admission

ρ=max|z|<1에 대해 정확 산술의 상대 꼬리계수는 ρ^d/(1−ρ)다. 이것을
정해진 candidate budget 이하로 하는 최소 d를 먼저 선택하되 degree/storage/
work cap을 넘으면 거절하거나 cluster를 나눈다. target을 관찰한 뒤 tolerance를
풀어 수용하지 않는다. 이 priori 기준은 candidate 생성량을 제한하는 계획이고
finite-arithmetic certificate 수용을 보장하지 않는다. 최종 수용은 항상 현재 r다.

r가 커지거나 ρ→1이면 dr solves와 출력 비용이 커져 이득이 사라진다.
만약 B≈QC의 저rank 압축을 추가한다면 실제 target residual에는 원래 B를
사용한다. 또는 ||B−QC|| bound를 inverse gain과 함께 더한다. 서로 독립인
modal amplitudes를 같은 RHS라고 가정하는 축약은 허용되지 않는다.

## 다음 물리 client의 정확한 선택

Loop10/11의 기존 Fourier primitive는 scalar oscillatory moments를 이미
analytic하게 적분한다. 여기에 불필요한 matrix resolvent를 삽입해 새 이득을
만들면 안 된다. 실제 semilinear coupled block L(k)와 independent amplitudes,
nonlinear convolution을 가진 client를 선택하고, 그 client가 원래 요구하는
shifted action을 이 API에 연결해야 한다. Phase carrier·bandwidth·primitive
initial constants·closure tube를 원 target certificate에 계속 결속한다.

Complex shifts는 real-positive의 gain=1을 그대로 상속하지 않는다.
이미 Loop02는 Re γ>0에서 H-dissipative J에 대한 gain≤|γ|/Re γ를 증명했다.
따라서 그 정리를 다시 연구할 필요는 없고, native complex interval residual과
그 gain의 directed 계산 및 physical norm transport를 이식해야 한다.
Re γ≤0 또는 near-pole 범위는 이 정리 밖이며 별도 실패다.
General H metric도 physical norm으로의 transport condition number를 계상해야
한다. 이 두 확장은 이번 native 이식의 완료 범위 밖이다.
