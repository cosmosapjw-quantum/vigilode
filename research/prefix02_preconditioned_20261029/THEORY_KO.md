# VIGILODE-PREFIX02: 좌·우 전처리 GMRES와 원래 방정식 잔차 인증

2026-10-29. **새로운 일반 GMRES 수렴정리나 새로운 알고리즘의 우선권을 주장하지 않는다.** 원래 연구 목표는 VigilODE PR104의 구조적 stage acceleration 및 PR105의 original-residual prefix를 실제 전처리 의미론에 연결하는 것이다. 이번 정리의 실험 범위는 binary64로 저장된 *point dense* `J`, 고정된 대각 전처리기, synthetic model이다.

## R1–R3. 기준, 컨벤션, 근거

- GitHub PR105 `2a815e98a277c3b9ee807f6098f3d1950bf81985` 위 새 독립 연구 노드. PR104 `10854bc0bff81b6d3cf5cb0a44520ed9165c205f`, PR103 `1845e1d8bdfda222bf77637e8d38d5076743f238`, PR102 `8357faa5b42724a8be24609d4d2f16aec8a6eb3a`는 서로 다른 admission 범위다.
- 원래 ODE stage의 선형화는 `A=I−σJ`, `σ=hγ≥0`, `[J]=[clock]^{-1}`, `[σ]=[clock]`, `A`는 무차원. `b`, `x`, `r`은 같은 stage 단위. `w_i>0`를 고정해 `||x||_{w,∞}=max_i|x_i|/w_i`.
- `M`은 가역 전처리기. 코드에서는 `M^{-1}=diag(d_i)`, `d_i`는 finite nonzero represented binary64이며, 오차 인증 자체는 M의 상태공간 표현에 의존하지 않는다. 일반 non-diagonal M은 이 실험에 구현하지 않았다.
- 원전: Saad & Schultz (1986), GMRES DOI 10.1137/0907058; Drkošová, Greenbaum, Rozložník & Strakoš (1995), *Numerical stability of GMRES*, DOI 10.1007/BF01732607; Wathen, *Preconditioning and convergence in the right norm*, https://ora.ox.ac.uk/objects/uuid:355787b7-ef50-4765-8f9a-970989034ffa/files/m91a00ccaffb99536866adfaacca9d925; Duintjer Tebbens & Meurant (2014), DOI 10.1007/s11075-013-9695-x. 문헌의 일반적인 GMRES 잔차 문제와 이번 source/epoch 계약은 구별한다.

## R4–R5. 좌·우 전처리의 서로 다른 잔차 항등식

`r0=b−Ax0`를 원래 방정식 잔차라고 하자. 

**우전처리**: `B_R=A M^{-1}`, `r0=βv1+g0` 및 계산된 Arnoldi 관계

`B_R V_k = V_{k+1} Hbar_k + F_k^R`.

저장된 후보가 `xhat=x0+M^{-1}V_k y+e_x`이면, 정확한 실수 항등식은

`b−A xhat = V_{k+1}(β e1−Hbar_k y)+g0−F_k^R y−A e_x`. (R)

**좌전처리**: `B_L=M^{-1}A`, `M^{-1}r0=βv1+g0^L`, `B_L V_k=V_{k+1}Hbar_k+F_k^L`. 후보 `xhat=x0+V_k y+e_x`에 대해

`M^{-1}(b−A xhat)=V_{k+1}(βe1−Hbar_k y)+g0^L−F_k^L y−M^{-1}A e_x`. (L)

좌전처리 projected residual은 **원래 r가 아니라 M^{-1}r의 좌표**다. 따라서 `||M^{-1}r||≤τ`를 `||r||≤τ`로 바꾸면 안 된다. 예: `A=I_2`, `M^{-1}=10^{-12}I_2`, `b=(1,0)`, `x0=0`이면 전처리 잔차의 Euclidean norm은 `10^{-12}`, 원래 잔차는 1이다. 원래 stage 허용오차 `10^{-8}`이라면 초기해를 채택하면 거짓 양성이다.

두 방식 모두 **저장된 후보에 대해 원래 `r=b−A xhat`를 직접 outward 계산**하면 (R)/(L)의 Arnoldi decomposition 및 전처리기 평가오차를 다시 합산할 필요가 없다. 그러나 실제 `JVP`가 원래 `J`를 나타내는지, 계산된 source와 clock이 동일한지는 별도 검증 입력이다.

## 원래 방정식 잔차에서 stage 상태오차로

`μ_w(J)=max_i(J_ii+Σ_{j≠i}|J_ij|w_j/w_i)`. `μ_w(J)≤m`, `σ≥0`, `1−σm>0`라면

`||(I−σJ)^{-1}||_{w,∞}≤K=1/(1−σm)`.

증명은 `||(exp(tJ))||≤exp(mt)` 및 `A^{-1}=∫_0^∞exp(−t)exp(σtJ)dt`의 수렴을 사용한다. `σ=0`이면 `K=1`. 비정규 J에도 적용된다. 

`r=b−A xhat`, `x*=A^{-1}b`이면 `xhat−x*=-A^{-1}r`이고,

`||xhat−x*||_{w,∞}≤K ||r||_{w,∞}`.

실제 binary64 `J,σ,b,xhat,w`를 *저장된 정확한 실수*로 간주하여 outward interval로 `ρ≥||r||_{w,∞}`, `K_up≥K`를 얻으면 `ρ≤τ_orig` 및 `K_upρ≤ε_stage`를 **둘 다** 만족할 때만 stage 후보를 승인한다. 원래 `JVP` callback 자체의 rounding/source 오차는 이 점행렬 증명에 포함되지 않는다.

## R6–R7. 반례, 기호 검산, 구현의 실패 모드

- Wolfram rational 2×2 검산: `A={{2,1},{0,3}}`, `M={{3,0},{0,5}}`, `r0={1,2}`에서 우/좌 항등식의 차가 각각 `{0,0}`. 이전 PR105의 `J=diag(-2,-3)`, `σ=1/4` 예에서는 `μ=-2`, `K=2/3`이며 실제 inverse weighted-infinity norm도 `2/3`.
- 전처리 잔차가 0에 가깝거나 underflow해도 원래 잔차가 크면 거절. stale source/clock/norm/epoch, invalid diagonal, overflow/nonfinite, nonpositive inverse margin, zero budget, false projected trigger, restart refusal에서 출력 불변을 테스트한다.
- Native subagent 기반 독립저자 R6는 현재 도구 미노출로 `NOT_RUN`. Wolfram, Rust 및 Python Fraction은 **독립 알고리즘 교차검산**이지 별도 연구자 검토가 아니다.

## R8–R9. 수치 형태와 복잡도

두 단계 모두 Arnoldi는 2-pass MGS와 incremental Givens를 사용한다. projected `|g_{k+1}|`는 **candidate 검사 trigger**일 뿐, original residual 또는 물리 오차 인증이 아니다. 좌전처리는 `M^{-1}r0`, `M^{-1}A v`로 Krylov를 생성하며 후보는 `x0+V y`; 우전처리는 `r0`, `A M^{-1}v`로 생성하며 후보는 `x0+M^{-1}V y`이다.

원래 잔차의 outward 계산은 이 prototype에서 `O(n²)` dense 비용이다. `k`개 Arnoldi 열을 줄여도 인증 비용이 지배하면 실제 wall-time 향상은 작을 수 있다. Native `gmres_into.rs`는 이미 `solve_gmres_into_right_monitored`를 갖지만 **매 열 least-squares/candidate/preconditioner를 구성**하므로 단순 wrapper는 prefix의 비용 장점을 재현하지 못한다. Native 진척에는 기존 Givens kernel을 활용한 trigger-before-backsolve 경로와 original-residual transactional gate가 필요하다.

Native `residual_scale`은 RHS/잔차 선택 노름에 영향을 주므로 `τ_orig`와 동일하다고 가정하지 않는다. 소스 식별자와 norm epoch가 변하면 검사 전후 모두 fail-closed. 원래 stage error budget과 callback 비용은 독립 소유권을 유지한다.

## R10. 판정과 다음 연결

- 수학: (R)/(L), 원래 잔차 충분조건은 `derived`, Wolfram 유리수 검산 `symbolically checked`.
- 구현: dense point-matrix diagonal-M prototype은 실제 테스트 결과에 한해 `implementation-verified`.
- 실제 BASS chemical6/physical33, full30, accepted quartic, nonlinear whole-slab, source/clock/rate, callback rounding, native PR104 integration, matched-accuracy full IVP, 독립 R6/C9는 **unresolved/NOT_RUN**.
- 원본 VigilODE DAG `7/23 inherited scoped`, 중앙 REI `28/88` unchanged. Production `HOLD`.
