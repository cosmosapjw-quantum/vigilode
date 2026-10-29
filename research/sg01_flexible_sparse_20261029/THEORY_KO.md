# SG01: 시간고정 비정규 stage의 원잔차 인증, Flexible GMRES, symbolic route 검증

2026-10-29. 범위: VigilODE native PR104/105/106/107 후속의 *독립 standalone 연구*. PR108 CMODE01의 weighted lognorm/cell-theory 노드와는 별개이다. 새 GMRES 일반 정리의 독창성이나 native/physical33 solver-ready를 주장하지 않는다.

## 정의와 충분조건 (derived)

하나의 시각, 동일 source/norm/epoch에서 저장된 정확한 이진64 점데이터 \(J,\sigma,b,w\)를 정확한 실수로 해석한다. \(\sigma=h\gamma\ge0\), \(w_i>0\), \(A=I-\sigma J\), \([\sigma][J]=1\). 요구 stage 해는 \(Ax_*=b\). Weighted infinity norm 및 signed logarithmic norm:

\[
\|v\|_{w,\infty}=\max_i\frac{|v_i|}{w_i},\quad
\mu_w(J)=\max_i\left(J_{ii}+\sum_{j\ne i}|J_{ij}|\frac{w_j}{w_i}\right).
\]

\(\sigma\mu_w(J)<1\)이면 semigroup/resolvent representation
\[
A^{-1}=\int_0^\infty e^{-t}e^{\sigma tJ}\,dt,
\qquad \|A^{-1}\|_{w,\infty}\le K=\frac1{1-\sigma\mu_w(J)}.
\]

출력 후보 \(\hat x\)에 대해 **원래 식** \(r=b-\hat x+\sigma J\hat x\)의 outward bound \(\rho\ge\|r\|_{w,\infty}\) 및 \(K_\uparrow\ge K\)를 생성하면
\[
\|\hat x-x_*\|_{w,\infty}\le K_\uparrow\rho.
\]

승인은 \(\rho\le\tau_{\mathrm{orig}}\) 그리고 \(K_\uparrow\rho\le\varepsilon_{\mathrm{stage}}\)일 때만 한다. 상한을 검사하기 전에 \(J,\sigma,b,w\)의 동일한 frozen identity와 epoch가 있어야 한다. Dense 및 exact-zero sparse 캐시에서 같은 수학적 값을 평가하며, 불확실한 작고 비영인 원소는 절대 버리지 않는다. 이 등식은 **저장된 이진64로 정의된 원래 연산자**에 대한 것이며 callback, physical model, source clock 및 actual owner rounding까지 입증하는 것은 아니다. 별도 binary64 배열 \(\widetilde A\)를 미리 구성하고 그 잔차만 계산하는 경우 \(A=I-\sigma J\)의 exact-real 의미를 잃을 수 있으므로 직접 원식을 interval 계산한다.

## 비정규성, flexible PC 및 original residual (derived)

Flexible right preconditioning에서는 Arnoldi가 각 열에서 \(z_j=P_j(v_j)\)를 저장한다. 사용되는 관계는
\[
A Z_m=V_{m+1}\bar H_m+F_m,\quad \hat x=x_0+Z_m y+e_x,
\]
\[
r=V_{m+1}(\beta e_1-\bar H_m y)+g_0-F_m y-Ae_x.
\]

\(P_j\)가 선형·고정일 필요는 없으며, 각 \(z_j\)는 그 열의 실제 반환 벡터로 저장해야 한다. `P(Σ v_j y_j)`를 후보 조립 시 다시 계산하면 시간 가변·상태 의존·비선형 PC에서 위 동일성을 잃는다. 예: \(A=I,\ b=(1,2)\), 첫 호출 \(P_1(v)=v\), 후보 재호출 \(P_2(v)=4v\). 저장 \(Z_1\) 경로는 \(b\)를 재구성하지만 재호출 경로는 \(4b\)가 된다. 즉 원래 잔차의 직접 검사는 필요하지만, 저장 Z는 계산량과 알고리즘적 의미까지 보호한다.

Projected Givens 잔차 \(|g_{k+1}|\)는 *후보 backsolve 시작 여부만* 결정한다. Arnoldi-orthogonality 손실 및 preconditioner 왜곡, callback error 때문에 이것만으로 상태오차를 승인하지 않는다. `every_prefix` 비교군은 모든 열에서 backsolve와 후보 생성을 수행하지만, **승인검사는 동일한 trigger 위치에서만 실행**하여 동일한 출력·열수·허용오차를 만든다.

## Symbolic dynamics의 범위 (derived, implementation-verified scoped)

알고리즘 경로의 유한 알파벳 \(\{K,Q,R,C,X\}\)는 Arnoldi, original residual check, reject, commit, abort다. 허용 전이는 `Init→K/X`, `K→K/Q/X`, `Q→R/C/X`, `R→K/X`이고 C/X는 terminal. 이를 `valid_itinerary`에서 결정론적 유한상태 언어로 검사한다. 이것은 **solver-control symbolic itinerary**이며 실제 ODE flow의 Markov partition, 궤도 결합이나 topological conjugacy를 의미하지 않는다. 물리 event partition을 인증하려면 각 slab의 interval guard·grazing/refusal과 chronological transfer가 따로 필요하다.

## 수치 조건과 실패 클래스

Interval 연산은 입력 binary64 point를 exact-real로 간주하고 곱·합·나눗셈의 각 결과를 인접 binary64로 바깥쪽 확장한다. Inf/NaN, denominator \(\le0\), nonpositive weight, 바뀐 epoch, zero PC, 잘못된 budget은 fail-closed. Rust `SparseFrozen`은 동일 immutable model lifetime과 동일 \(J,w,\sigma\)의 exact-zero 구조만 사용하며 n×n을 한번 스캔한다. 이후 `apply A`는 \(O(\mathrm{nnz})\), residual certificate도 \(O(\mathrm{nnz}+n)\). `mu_upper`는 한번 계산 후 동일 frozen model에 대해 재사용한다. Dense reference는 per-call \(O(n^2)\)이다. Sparse 사전구축 비용과 메모리는 별도 과금하며 캐시 identity가 보장되지 않는 변경 가능한 원본에는 재사용 금지.

**열린 엄밀성 의무:** 실제 JVP/RHS의 함수평가오차 포함, full30/physical33 Gram 규약과 norm transport, 가변 물리 clock, 비선형 tube에서 J variation, stage-to-IVP transfer, event root enclosure, 중간 callback failure/partial work의 native WorkCounters, 재진입·thread safety, full authority 및 모든 물리 validation gate. SG01의 binary64 point certificate만으로 이 항목을 닫지 않는다.