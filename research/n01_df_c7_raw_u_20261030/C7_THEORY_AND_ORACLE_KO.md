# VigilODE N01 C7: derivative-free 보조 RK와 원래 RODAS5P raw-U의 과학적 판별

작성 날짜 2026-10-30 KST. **실제 native C7의 제한적 물리 검증**이며, production 경로/원래 8-stage solver 구현 변경이 아니다.

## 1. 출처와 계산 범위

- Parent: VigilODE PR120, HEAD `f022c4c5593b8d3f1f4ee7ba3efa56fd800dd413`.
- 현재 작업은 PR117의 Robertson 단일 BE/diagonal implicit-RK 정리, PR119의 RHS-only homotopy bridge, PR120의 C6 native 35/35 test를 선행 증거로 한다.
- 원래 목표는 `rodas5p_matrix_free_fast.rs`/`raw_stage_target.rs`가 규정하는 8-stage raw-U 목표. Snapshot SHA-256 `211255f6125b648c6955833c3da537d9c031ec78168b2918563c29fa88c8bce6`.
- 3성분 direct/explicit Jacobian은 **원래 native RODAS5P의 비교 oracle에서만** 사용한다. RHS-only Newton-free 경로를 JVP-first로 바꾸지 않는다.

## 2. C7의 핵심 과학적 반례

정확한 실수 모델에서

\[
 f(a,b,c)=(-k_1a+k_2bc,\quad k_1a-k_2bc-k_3b^2,\quad k_3b^2),\quad k_i\ge0.
\]

이전 단계에서 성공한 보조 BE root는 \(d=z-y=\tau f(y+d)\)이다. RODAS5P 원래 첫 stage는 \(\bigl(I-\tau J(y)\bigr)U_0=\tau f(y)\), \(\tau=h\gamma\)이므로 \(U_0=d\)를 그대로 투입했을 때

\[
 r_{U_0}=(I-\tau J(y))d-\tau f(y)
       =\tau\,[f(y+d)-f(y)-J(y)d]
       =\tau(k_2d_bd_c,-k_2d_bd_c-k_3d_b^2,k_3d_b^2).
\]

따라서 \(\|r_{BE}\|=0\)이더라도 \(\|r_{U_0}\|>0\)가 가능하다. 이는 **BE certificate → original raw-U admission**이 잘못된 추론임을 특정한 반례로 보인 것이다. 원래 목표에 대해 보편적인 불가능성 정리는 주장하지 않는다.

정확한 dyadic 예: \(k=(1/2,1,1/2)\), \(y=(35/64,27/128,31/128)\), \(\tau=1/4\), \(z=(1/2,1/4,1/4)\). BE 원잔차는 정확히 영이지만 raw-U0 잔차는 \((5/65536,-35/131072,25/131072)\), \(\|r_{U_0}\|_\infty=35/131072\approx2.6702880859375\times10^{-4}\).

256개 결정론적 dyadic 원본 z, rate, tau 사례에서 역구성한 y=z-tau*f(z) 및 BE 잔차 0이 성립하며 256/256 모두 raw-U0 잔차는 0이 아니었다. 1,799개의 기록된 Python Fraction exact assertion 전부 PASS. Wolfram CAS에서도 전체 이차 remainder 다항식, \(\mathbf1^Tr=0\)와 제조 사례를 exact 유리수로 확인했다.

### 정확한 작은-step 세제곱 오차

동일한 종료점 z와 원래 k를 고정한 가족 \(y(\tau)=z-\tau f(z)\)에서는 \(d=\tau f(z)\). Source의 remainder는 정확히 2차이므로

\[
 r_{U_0}(\tau)=\tau^3 Q(f(z),f(z)),
\]

이며 \(\tau\)를 정확히 두 배로 키울 때 \(\|r\|\)도 정확히 8배가 된다. 10개 exact dyadic \(\tau\) 전부 PASS. 이 결과는 **동일한 y의 fixed-step 변화**에 대한 주장과는 다르다. 초기 상태 y가 \(\tau\)와 함께 변화하는 제조 family에 한정한다.

## 3. 원래 8-stage mass invariant와 C7 native 비교

고정 \(J_0=Df(y)\)에 대해 \(\mathbf1^Tf=0\), \(\mathbf1^TJ_0=0\). 원래 raw-U 식

\[
 (I-\tau J_0)U_i=\tau f\bigl(y+\sum_{j<i}A_{ij}U_j\bigr)
       +\gamma\sum_{j<i}C_{ij}U_j
\]

에 \(\mathbf1^T\)를 곱하면 \(\mathbf1^T U_i=\gamma\sum_{j<i}C_{ij}\mathbf1^TU_j\). 따라서 strictly causal A,C 아래에서는 모든 \(i=0,\dots,7\)에서 \(\mathbf1^T U_i=0\)이고 \(\mathbf1^Ty_{new}=\mathbf1^Ty\)이다. 컴퓨터에서는 반올림·coefficient 변환 차이를 명시적으로 검사해야 한다.

실제 native Rust1.94.1 소규모 비교는 \(y=(0.75,0.125,0.125)\), \(k=(0.5,1.0,0.5)\), \(h=0.005\), identity mass, autonomous source로, **기존 protected sequential RODAS5P를 direct 3×3 oracle**로 8-stage 실행했다. 각 K stage를 frozen snapshot의 Gamma로 U로 재구성하고, 원래 8-stage raw-U residual을 재평가했다. 해당 주장이 적용되는 것은 이 한 source 상태·step·tableau 조합이다. Native local-error estimate를 전역 물리 오차 상한으로 재해석하지 않는다.

### C7에서 아직 수치적으로 닫히지 않은 일반적 연결

위에서 얻은 보조 BE 해는 source-specific 첫 stage 후보 생성에 사용할 수 있지만, full RODAS5P 8-stage 후보의 각 원방정식 잔차 및 반환 물리 오차가 충분히 작아야 한다. 엄밀한 단계 오차 전파는 \(\kappa=\|(I-\tau J_0)^{-1}\|_w\) 및 admissible tube에서의 source RHS Lipschitz \(L_f\)가 **실제 verified owner**에서 주어질 때 다음 조건부 형태로 쓸 수 있다.

\[
 e_i\le D_i+\sum_{j<i}L_{ij}e_j,\quad
 D_i=\kappa\|r_i\|_w,\quad
 L_{ij}=\kappa(|\gamma C_{ij}|+\tau L_f|A_{ij}|),
\]

\(L\)이 strictly lower 8×8이면 \(e\le(I+L+\cdots+L^7)D\)이고 \(\|\Delta y\|_w\le\sum_i|b_{code,i}|e_i\). 이는 원래 **이산 8-stage algebraic target의 오차 전파**에 대한 식이지 시간 적분의 전역 오차 보장은 아니다. Source domain, clock/epoch, tableau allowance와 interval rounding을 합치지 않으면 production admission은 불가하다.

## 4. 독립 검산 및 음성 증거

- Wolfram: 이차 source remainder 정확식과 질량합 0, 제조 r_U0 유리수 세 성분 PASS.
- Python Fraction: 256/256 exact physical positive manufactured cases, 1,799 기록 검산 PASS. τ^3 10/10 및 doubling ratio8 PASS.
- 실제 native C7: 전용 테스트 Rust source는 GitHub blob `1156ea9e67562b8177cebbead92635e7e8a697a6`. 최초 실행에서 수학 테스트 3/3 PASS, 반복문 형태 Clippy 5개 FAIL; 수정 후 테스트/Clippy PASS. 뒤이어 Python oracle의 repository 내 경로 오류가 확인되어 원문 `ORACLE_PATH_RED.json`에 보존했다. 소스만의 경로 교정 후 전체 focused CI 결과는 최종 receipt로 판정한다.
- R5 및 C5/C6의 과학적 범위를 확대하지 않는다. 원래 global DAG 7/23, REI28/88, successor0/12, production HOLD.
