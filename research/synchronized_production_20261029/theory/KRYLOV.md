# Krylov 비용의 분해와 RHS별 quotient 보정

Source: PR101 `bc5ddbe42b24b1a6364bf653c3e2d6d938ca4087`. 조사일: 2026-10-29.

## 1. 이번 결과와 적용 대상

현재 기본 facade의 matrix-free route는 `RoutingOptions.krylov_ls_workspace=true`를 통해 GMRES-into와 재사용 small-QR workspace를 선택한다. `Rodas5pMfFastWorkspace::new`의 `gmres_into=false`만 읽으면 실제 경로를 잘못 분류한다. GMRES-into의 `begin_cycle`은 공통 `ArnoldiWorkspace::prepare`를 부르지만, 이 경로의 모든 Arnoldi 곱과 최종 보정은 `basis`에서 직접 읽는다. `directions`의 길이 n 벡터 m개는 이 경로에서 읽히지 않는다.

따라서 첫 구현 후보는 **공통 scalar/matrix scratch와 basis를 준비하되 directions는 augmented 경로가 실제 필요할 때 준비하는 것**이다. 정리 K1은 이 변경의 정확한 계산 의미를 준다. 두 번째 결과 K3는 기존 full-space quotient 아이디어를 RHS별 관계로 좁혀, 수치해석적으로 인증된 저차 보정 후보를 구성한다. K4는 operator epoch가 바뀔 때 recycle image 재사용의 추가 오차를 식으로 분리한다.

## 2. 병목 분류와 기존 결과 재사용

| 관측/코드 | 분류 | 이번 연결 |
|---|---|---|
| PR101 time-JVP MGS 24.22%, Arnoldi 7.36%, QR 5.60% guest Ir | 알고리즘 연산 비용. 이 비율은 wall-time 비율이나 하한이 아니다. | MGS를 바꾸기 전에 K2로 원래 residual 의미를 유지한다. |
| `ensure_pool` 396 calls / 12 solves = 33/solve | 작업공간 구현 비용 | m=16일 때 basis 17 + directions 16과 일치하는 정황. 새 계측으로 직접 분리한다. |
| `gmres_into.rs::least_squares_correction`은 cycle당 한 QR, monitored는 prefix마다 QR | 관측 계약에 따른 서로 다른 알고리즘 | SP(D)04/SP(D)05/RNEXT02의 완료 결과를 재사용. 모니터가 보는 prefix를 없애지 않는다. |
| `polynomial_resolvent`, `conditioned_polynomial` | full-space relation + 원공간 residual + epoch/norm binding | K3는 RHS별 계산 후보에만 적용. 기존 full-space token의 의미를 바꾸지 않는다. |
| PR93 fused Clenshaw/PR94 error contract | 목표별 다항식 계산, 총 오차 소유권 | 지연된 연산과 rounding/certificate 비용을 같이 센다. |
| GCRO-DR `system_identity`, refresh, `BU=C` | operator/preconditioner epoch 검증 | K4는 identity가 달라질 때의 변화량을 계산한다. |

PR101의 profile은 원본 측정 source와 workload에 속한다. 이번 구현의 성능은 별도 source-bound 비교로 판정한다. 과거 PR68의 warm-Chebyshev 3.17배와 통계 authority HOLD, PR69의 매우 느슨한 Laguerre majorant 및 0/22 scale 개선 결과는 새 성과로 다시 세지 않는다.

## 3. K1: 사용하지 않는 direction pool 제거의 계산 동등성

### 명제

메모리 확보가 성공하고, 같은 컴파일/산술 환경 및 같은 입력 bits를 사용하자. operator와 preconditioner callback은 인자·호출 이력·명시된 epoch에 따라 같은 값을 쓰고 같은 오류를 반환한다고 하자. allocator/pointer identity를 외부 계산 입력으로 삼는 callback은 이 계약의 대상이 아니다.

`gmres_into.rs`의 cycle 준비에서 `directions`의 allocation/resize만 생략하고, 나머지 필드의 준비 순서와 모든 수치 연산을 그대로 유지하면:

1. 각 operator/preconditioner 호출의 인자 bits·순서, MGS·QR 입력 bits와 수치 work counter가 같다.
2. 성공 시 해·residual·반복수·오류 분류와 failure 시 output 불변성이 같다.
3. allocation 수, retained bytes, capacity 및 그에서 유도되는 growth telemetry는 변경된 실제 자원 사용을 보고한다.

### 증명

기계 상태를 `L=(common,basis,H,Hprefix,hcol,rhs_small,correction,raw_output,LS)`와 사적인 `D=directions`로 나누자. 두 실행의 관계는 L의 유효 길이·bits·산술 설정이 같다는 것이다. D는 이 관계에서 자유다.

준비 함수의 L 관련 실행이 같으므로 준비 후 관계가 성립한다. `basis[0]`은 start를 완전히 덮어쓰고 동일한 `normalize`를 호출한다. column j의 귀납 가정 아래 `basis[..=j]`가 같다. `split_at_mut(j+1)`가 분리한 다음 벡터에 left 경로는 `P^-1 A basis[j]`, right 경로는 `A P^-1 basis[j]`를 계산한다. 두 경로 모두 D를 읽지 않는다. 같은 callback 호출·출력 뒤 같은 두 번의 MGS, 같은 순서의 dot/axpy, 같은 `safe_l2`, 같은 breakdown 비교, 같은 normalization이 수행되므로 `H[:,j]`와 다음 basis가 같다. 이것으로 각 prefix의 귀납이 닫힌다.

최종/monitored `least_squares_correction`은 같은 H prefix와 `beta e1`을 사용하고, `basis[..actual]`로 선형 결합한다. 이어지는 preconditioner 적용·해 갱신·원공간 residual·monitor 순서도 같다. 오류가 발생하는 수치 연산까지 동일한 경로를 따라가며 외부 output은 성공 지점에서만 복사된다. D에 접근하는 연산은 없으므로 삭제한 준비가 수치 관측을 바꾸지 않는다. □

### 코드 계약과 혼합 경로

`ArnoldiWorkspace::prepare_basis_only(n,m)`가 L을 준비한다. 기존 `prepare(n,m)`는 이 함수를 사용하면서 directions도 기존처럼 준비한다. `gmres_into::begin_cycle`만 basis-only를 호출한다. 공개 `GmresWorkspace::reserve`는 full prepare를 유지한다. 이러면 reserve 뒤 legacy/augmented 경로로 전환해도 선언한 용량 계약이 보존된다. 기존 directions를 삭제·shrink하지 않으므로 한 번 확보한 legacy workspace를 into가 불필요하게 해제하지 않는다. 이후 legacy/augmented prepare가 바뀐 n에 맞게 D의 길이를 갱신한다.

검증에는 left/right/monitored, happy-breakdown, 실패 후 재사용, n 증가·감소, into→legacy→into, reserve→solve가 필요하다. poisoned stale D가 into의 해에 영향을 주지 않고 augmented 경로에는 유효한 D가 준비됨을 직접 확인한다. cold path의 n-vector allocation 감소는 m개이며, 기존 pool을 이미 보유한 warm path의 감소는 0개다. 예약 호출이 full pool을 준비하는 비용도 숨기지 않는다.

## 4. K2: prefix residual의 수학적 의미

정확한 실수 행렬 A, 초기해 x0, r0=b−Ax0에 대해 저장된 벡터 V와 H가

\[
AV_m=V_{m+1}\bar H_m+F_m,\qquad r_0=\beta v_0+g_0
\]

를 만족한다고 정의한다. 이 식은 계산된 각 양을 정확한 실수로 해석한 **결함의 정의**이며 직교성을 가정하지 않는다. 임의의 계수 y와 `x=x0+V_m y`에 대해

\[
r=V_{m+1}(\beta e_1-\bar H_my)+g_0-F_my. \tag{K2.1}
\]

이는 대입으로 얻는다. 따라서 같은 선언 norm에서

\[
\|r\|\le \|V_{m+1}\|\,\|\beta e_1-\bar H_my\|
 +\|g_0\|+\|F_my\|. \tag{K2.2}
\]

W가 고정된 양의 정부호 metric이고 `||V*WV−I||₂≤η`를 검증했다면 `||V||_{2→W}≤sqrt(1+η)`. 검증된 Gram bound가 없으면 `||V||_{2→W}≤sqrt(sum_j ||v_j||_W²)`를 사용할 수 있다. 저장된 해 조립의 반올림 `Δx`가 있으면 우변에 `||AΔx||`를 더한다. 별도로 실제 original-space residual를 안전하게 계산했다면 그 검증이 이 분해를 대체할 수 있다.

두-pass MGS를 one-pass로 바꾸면 F와 Gram bound, stopping 이력이 바뀐다. MGS 비용이 크다는 profile만으로 이 변경의 정확도가 나오지 않는다. Paige–Rozložník–Strakoš(2006)의 backward stability 결과는 특정 MGS-GMRES와 유효한 numerical nonsingularity 조건을 분석한다. 현재 monitored/right/preconditioned 경로의 동일 contract를 자동으로 보장하는 정리로 사용하지 않는다.

실제 m columns의 현재 두-pass MGS는 각 column j에서 `2(j+1)` dot과 axpy를 호출하므로 각각 정확히 `m(m+1)`회다. 이는 현 알고리즘의 연산수이며 대체 알고리즘의 하한이 아니다. K1은 이 연산수와 산술을 그대로 둔다.

## 5. K3: RHS별 annihilator에서 얻는 quotient 보정

### 정의와 차원

고정 J∈Rⁿˣⁿ, 현재 RHS v, 비음수 σ=hγ를 택한다. J는 시간⁻¹, σ는 시간, Aσ=I−σJ는 무차원이다. 차수 d≥1인 monic polynomial

\[
p(z)=\sum_{j=0}^d p_jz^j,\quad p_d=1,
\qquad e=p(J)v
\]

를 사용한다. `p_j`의 차원은 시간^{j−d}이며 e는 v·시간⁻ᵈ이다. full-space `p(J)=0`을 가정하지 않는다. 현재 v에 대한 e의 검증된 bound 또는 최종 원공간 residual이 필요하다.

\[
D(\sigma)=\sum_{j=0}^{d}p_j\sigma^{d-j},\quad
N_k(\sigma)=\sum_{j=k+1}^{d}p_j\sigma^{d-j+k},
\quad Q_\sigma(z)=\sum_{k=0}^{d-1}N_k(\sigma)z^k.
\]

`D≠0`이면 `x=Qσ(J)v/D`로 정의한다. 모든 항의 차원을 대조하면 D와 Qσ(J)는 무차원이다.

### 항등식과 증명

\[
(1-\sigma z)Q_\sigma(z)=D(\sigma)-\sigma^d p(z). \tag{K3.1}
\]

상수항은 `N0=D−σᵈp0`, 1≤k<d의 zᵏ 계수는 `Nk−σN{k−1}=−σᵈpk`, 최고항은 `−σN{d−1}=−σᵈpd`이다. 모든 계수가 일치하므로 증명된다. z=J에 대입하고 v에 작용하면

\[
r=v-A_\sigma x=\frac{\sigma^d}{D(\sigma)}e. \tag{K3.2}
\]

σ=0에서는 D=1, N0=1, 나머지 Nk=0이므로 x=v, r=0이다. `p(1/σ)`나 `1/σ`를 계산할 필요가 없다. σ>0이면 D=σᵈp(1/σ); D=0은 이 표현의 pole이므로 quotient 경로를 거절하고 기존 residual-controlled solver로 넘긴다. 작은 |D|에서는 엄밀한 평가오차와 오차 예산이 허용할 때에만 채택한다.

### 원래 선형계의 오차 bound

고정 SPD W에 대해

\[
\mu_W(J)=\lambda_{\max}\left[W^{-1/2}
\frac{WJ+J^TW}{2}W^{-1/2}\right]\le\bar\mu,
\qquad \alpha=1-\sigma\bar\mu>0
\]

를 검증했다고 하자. 임의의 z에 대해
`<z,Aσz>W ≥ α||z||W²`. Cauchy–Schwarz로 `||Aσz||W ≥ α||z||W`이며 Aσ가 가역이고 `||Aσ⁻¹||W≤1/α`다. 따라서 정확한 해 x*에 대해

\[
\|x-x_*\|_W\le
\frac{|\sigma|^d\|e\|_W}{|D(\sigma)|\alpha}. \tag{K3.3}
\]

이 증명은 J의 normality나 eigenvalue-only spectral bound를 쓰지 않는다. W와 J가 바뀌면 해당 인증의 epoch가 바뀐다. WRMS norm도 고정된 양의 가중치에서는 W의 특수 사례다.

### 계산된 후보에 대한 두 가지 유효한 계약

1. 정확한 Q/D와 계산된 후보 사이의 `||δx||W≤ηx`를 enclosure로 구하면 K3.3에 ηx를 더한다. ηx에는 coefficient·operator action·Horner·division·range 손실을 포함한다.
2. 계산된 후보에 대해 원래 Aσ의 residual r̂를 직접 계산하고 `||r−r̂||W≤ηr`를 검증하면 `||x̂−x*||W≤(||r̂||W+ηr)/α`다. v나 J의 source uncertainty는 ηr 또는 별도 명시한 오차 소유권에 포함한다.

위 둘을 동시에 더해 중복 과금하지 않는다. 통상 f64 residual norm만 계산한 결과는 진단값이며 directed/source-error enclosure를 가진 인증과 구분한다. fixed floating operator를 실제 J로 선언해 분석하는 경우에도 physical J와의 차이는 외부 owner가 처리해야 한다.

### 현재 구조에 대한 구체적 d=2 구성

`p(z)=z²−az−b`이면

\[
D=1-a\sigma-b\sigma^2,\quad
x=\frac{(1-a\sigma)v+\sigma Jv}{D},\quad
r=\frac{\sigma^2}{D}(J^2v-aJv-bv). \tag{K3.4}
\]

후보에는 Jv 한 번, 관계 결함을 확인하면 J²v 한 번이 필요하다. 실제 original residual 검증을 별도로 하면 그 추가 곱도 센다. 이미 검증된 구조에서 Jv가 준비된 경우와 cold 경우를 분리한다. 이 후보는 MGS/QR을 호출하지 않는다. 실패한 후보의 setup·곱·잔차 후 fallback 비용을 모두 센다.

예: `J=[[-1,2,0],[0,-2,0],[0,0,-9]]`, `p(z)=z²+3z+2`. v=(1,1,0)에서는 e=0이나 p(J) 자체는 diag(0,0,56)으로 0이 아니다. 따라서 RHS별 증명이 full-space 증명보다 실제로 약한 가정이라는 비공허한 예다. v=(1,1,1/1000)에서는 e=(0,0,7/125); K3.2는 근사 구조의 잔차를 그대로 준다. Python Fraction으로 σ∈{0,1/16,1/2,2}의 두 RHS, 8개 항등식을 정확히 확인했다. Wolfram은 d=1,…,6의 계수 항등식을 모두 0으로 확인했다. 일반 d의 증명은 위 계수 비교이며 이 유한 검산에 의존하지 않는다.

### 구현의 첫 범위

별도 opt-in 연구 leaf로 d=2 후보와 exact-rational oracle부터 구현한다. input identity는 J provider/epoch, v bits, W/norm epoch, σ bits, a/b 및 그 오차, residual owner를 포함한다. 원래 residual 수용 기준을 유지한다. 선언되지 않은 full-space 관계를 sampling으로 만들지 않는다. 다음 Rust production 이식은 directed residual와 provider identity가 있는 existing polynomial-resolvent contract를 재사용한다.

## 6. K4: shifted epoch에서 recycle image를 갱신하는 식

같은 J와 같은 U에 대해 identity preconditioner의 `C0=(I−σ0 J)U`, σ0≠0라 하자. ρ=σ1/σ0이면

\[
C_1=\rho C_0+(1-\rho)U=(I-\sigma_1J)U. \tag{K4.1}
\]

이는 양변을 전개하면 증명된다. 저장된 Ĉ0의 결함 `F0=A0U−Ĉ0`와 새 조립 오차 ΔC를 두면

\[
A_1U-\widehat C_1=\rho F_0-\Delta C,
\quad \|A_1U-\widehat C_1\|\le |\rho|\|F_0\|+\|\Delta C\|. \tag{K4.2}
\]

σ0=0에서는 이 식을 쓰지 않는다. J U를 별도로 저장했다면 `C1=U−σ1 JU`를 직접 쓴다. nonidentity fixed left P에서는 U 대신 P⁻¹U를 별도로 갖춰야 한다. J나 preconditioner epoch가 바뀌면 이 항등식의 가정을 다시 확인하고 원래 refresh를 수행한다. metric/domain epoch 변경은 대수 항등식 자체와 구별해 norm·오차 인증을 무효화하므로 그 인증을 갱신한다.

중요하게도 C0의 직교성이 C1에 보존된다는 결론은 없다. QR에서 `Ĉ1 Π=Q R`의 채택된 rank block에 대해 U도 `U Π R⁻¹`로 함께 바꿔 `A Unew≈Q`를 유지해야 한다. 결함은 R⁻¹의 norm에 의해 증폭되며 rank/conditioning gate가 필요하다. 현재 GCRO-DR의 `orthonormalize_pair`는 paired QR transform과 rank floor 정책을 구현한다. outward `||R⁻¹||` 및 변환된 image defect의 엄밀 상한은 별도 인증 의무다. 이 구조를 우회하여 stale image를 즉시 채택하지 않는다.

같은 RHS b를 유지하는 일반 epoch 변경 A1=A0+ΔA에서는 재사용 candidate x=x0+U y의 residual 변화가 `r1=r0−ΔA x`이고, correction-only residual은 `r_before,1−A1 U y`다. `||ΔA U|| ||y||` 또는 직접 original residual이 누락되면 stale projected residual만으로는 수용할 수 없다. RHS도 바뀌면 `r1=r0+Δb−ΔA x`로 Δb를 포함한다. 이것은 구체적인 결함 identity이며 새로운 strong no-go로 주장하지 않는다.

## 7. 독립 검토와 채택 기준

현재 문서는 source audit, 계수별 증명, 정확 Fraction 검산과 Wolfram 대수 검산을 포함한다. root, state_audit, proof_audit가 원문·실제 source를 독립 검토해 이론 및 K1 이식을 scoped 채택했다. proof_audit의 75개 추가 Fraction 항등식 및 K4 부호 검산도 통과했다. 같은 RHS 조건, metric epoch와 대수 identity의 구별, QR rank policy와 엄밀 오차 인증의 구별을 반영했다. 이론 채택은 K1의 live-field 판정, K2의 부호와 조립 오차, K3의 σ=0/pole/metric 조건, K4의 preconditioner와 QR 재정규화 조건을 모두 확인한 뒤 한다.

이번에 새 strong no-go는 제시하지 않는다. NG1–NG7은 no-go 후보에 해당 없음이며, 정확한 counterexample은 그 반박 범위로만 사용한다. 문헌상 일반 GMRES/recycling 이론 자체의 신규성은 주장하지 않는다. 프로젝트 기여는 live route의 불필요한 pool을 특정한 것과, 기존 quotient/residual-first 계약을 RHS별 관계 및 shifted epoch 결함에 연결한 것이다.

## 8. 코딩 및 local 인계 수용 기준

K1의 최소 Rust 수정 뒤 같은 바이너리의 legacy reference/into를 left·right·dimension 변경·비정상 callback·monitored 경로에서 비교한다. 모든 수치 output bits와 callback/work parity를 요구하고 allocation/retained bytes는 의도된 차이로 별도 기록한다. m=16 cold solve에서 directions n-vector 16개 감소 가설을 검증하며 wall-time은 paired diagnostic으로만 판정한다.

K3는 정확 Python oracle, d=2 general dense rational 행렬, near relation, σ=0, D=0, 부호/normalization mutation을 먼저 구현한다. native route 연결에는 original residual enclosure와 α proof provider를 완성해야 한다. K4의 code 이식은 immutable J epoch와 U-image error bound를 가진 별도 contract가 필요하므로 현재 기본 경로를 바꾸지 않는다.

원래 DAG·correction budget을 유지한다. K1은 PR-N01의 workspace/연산 분해에 속하고, exhausted PR-N05나 Q-LEDGER를 이름만 바꿔 재개하는 작업이 아니다. 새 수정 episode는 parent의 전체 campaign ledger에서 관리한다.
