# VIGILODE-PREFIX01: signed lognorm과 original-residual-gated GMRES prefix

2026-10-29 KST. 이론 R1–R10 연구 단위. **문헌의 GMRES 조기 종료를 새로 발명했다는 주장은 하지 않는다.** 새 산출물은 PR104의 구조적 stage-solver와 PR103의 조건부 chemical6 signed-lognorm을 동일 original-equation norm에서 연결하는 구체적인 수학·구현 계약이다.

## Source lock 및 범위

- `cosmosapjw-quantum/vigilode` PR104, delivery `10854bc0bff81b6d3cf5cb0a44520ed9165c205f`, native execution `0a2a8eb9b9b0b775561f42f42cf36b1ef6e96f4b`.
- PR103, `1845e1d8bdfda222bf77637e8d38d5076743f238`: exact chemical6 signed lognorm의 **conditional** theorem, R6/C9 독립 저자 검토 미완.
- PR102, `8357faa5b42724a8be24609d4d2f16aec8a6eb3a`: scalar-affine event certificate, separate branch, 이번 native 실행에 병합하지 않음.
- BASS physical source `cc602e209e7deeb511ab2a1f3b2268d55206c43c`는 참조 identity이며 이 실험에 실제 입력하지 않았다.
- 원래 23-node/34-edge DAG는 inherited scoped `7/23`; 중앙 REI `28/88`; production HOLD.

## 1. 정의, 부호 및 차원

한 accepted stage에서 동일 frozen source/clock/epoch의 exact-real point matrix `J ∈ R^{n×n}`, `σ=hγ ≥ 0`를 선언한다. `A=I−σJ`, `b`와 `x0`는 binary64로 저장된 수를 정확한 실수로 해석한다. `w_i>0`도 고정된 exact represented weight다.

`||x||_{w,∞}=max_i |x_i|/w_i`, `μ_w(J)=max_i[J_ii+Σ_{j≠i}|J_ij|w_j/w_i]`.
`[J]=[clock]^{-1}`, `[σ]=[clock]`, `[A]=1`, `[r]=[b]`, `[||A^{-1}||]=1`.

**정리 1 (resolvent transport).** `μ_w(J)≤m`이고 `1−σm>0`이면

`||A^{-1}||_{w,∞} ≤ K:=1/(1−σm)`.

증명: logarithmic-norm semigroup estimate `||exp(tJ)||_{w,∞}≤exp(mt)`와
`(I−σJ)^{-1}=∫_0^∞ exp(−t) exp(σtJ) dt`를 사용한다. 조건 `1−σm>0`가 적분 수렴과 inverse 존재를 보장한다. `σ=0`에서는 `A=I`, `K=1`. 음수 σ에서는 같은 m을 재사용할 수 없으므로 현재 구현은 거부한다. J가 nonnormal이어도 성립하지만, J가 실제 물리 Jacobian임은 별도 source 증명이다.

**정리 2 (original residual to stage error).** 임의의 저장된 후보 `xhat`에 대해 `r=b−Axhat`, `x*=A^{-1}b`이면 `||xhat−x*||_{w,∞}≤K||r||_{w,∞}`. 따라서 `||r||≤ρ`의 outward-certified bound가 있을 때 `Kρ≤ε_stage`가 stage error sufficient condition이다. 이는 GMRES projected residual만으로 성립하지 않는다.

## 2. Prefix selection, authority 및 실패

GMRES Arnoldi의 projected least-squares 잔차 `|g_{k+1}|`는 **비용이 싼 검사 트리거**로만 쓴다. 이 값은 부동소수점 Arnoldi relation, 직교성 손실, warm-start reconstruction 때문에 원래 `||b−Axhat||`와 다를 수 있다. 기존 GMRES의 1997 CERFACS 원전과 2026 modular backward-error 분석도 원래 잔차를 별도로 확인할 필요성을 논의한다.

1. 동일한 초기 `x0`와 Arnoldi basis에서 incremental Givens를 갱신한다.
2. `|g_{k+1}| ≤ κ τ_proj`이면 triangular backsolve로 prefix candidate를 만든다. 이 판정은 채택이 아니라 **검사 요청**이다.
3. binary64 J, σ, b, candidate를 exact represented 실수로 보고 interval 연산으로 `r_i=b_i−xhat_i+σΣ_jJ_ij xhat_j`를 outward 포함한다. `ρ=max_i |r_i|/w_i`를 위에서 감싼다.
4. `ρ≤τ_orig`와 `Kρ≤ε_stage`를 동시에 만족해야 output을 commit한다. 하나라도 실패하면 Arnoldi를 이어가거나 budget 소진 시 실패를 반환한다. 입력 오류·비정상 callback·분모/FP 오류를 성공으로 치환하지 않는다.

프로토타입은 **identity-preconditioner, one-cycle, dense point-matrix**다. PR104의 production GMRES는 다른 left/right preconditioning, workspace/restart, stage transactions, matrix-free callbacks가 있으므로 이 소스를 바로 drop-in patch로 취급할 수 없다. Native adaptation은 별도 C-stage다.

## 3. Exact binary64 interval arithmetic와 한계

`Interval`은 각 basic operation 후 `next_down`/`next_up`을 사용한다. 최근접/짝수 반올림과 gradual underflow를 동기적 호출마다 관측하는 x86_64 probe를 포함한다. overflow/nonfinite, invalid weights/clock, `σm≥1`은 fail closed. 연산 중 FP 환경이 변경되지 않고 IEEE 기본 연산이 준수된다는 전제다. JVP callback의 평가오차, source/rate uncertainty, clock rounding, physical norm transport는 이 점행렬 oracle에 포함되지 않는다.

**정리 3 (단일 상태 event 부호).** `g(x)=c^Tx−θ`에서 stage error `E`가 있으면 `|g(xhat)−g(x*)|≤(Σ_i |c_i|w_i)E`. outward center±dual radius가 0과 겹치면 `UNRESOLVED`다. 이는 한 stage 상태의 부호만 인증하며 연속 시간 crossing uniqueness를 인증하지 않는다.

## 4. 성능 조건 및 공정한 비교

Selective prefix가 m-column end-only보다 유리하려면 제거한 Arnoldi/JVP/MGS 비용이 추가 prefix backsolve와 original-residual certificate 비용보다 커야 한다. 반대로 모든 column에서 expensive certificate를 실행하면 실패 prefix의 비용을 누적해 더 느려질 수 있다. 이 주장은 실제 동일 binary AB/BA timing으로 검증해야 하며, 단순한 MGS 카운터 0을 wall-time speedup으로 해석하지 않는다.

본 실험의 16 distinct-rate synthetic diagonal J에서 `σ=1/4`, `μ=-2`, `K=2/3`이고 Wolfram exact inverse check도 `2/3`이다. `τ_orig=ε_stage=10^{-7}`에서 prefix는 7 columns, end-only는 16 columns이며 둘 다 같은 **허용 예산**을 만족한다. End-only의 실제 error는 더 작으므로 이 비교를 같은 달성 오차라고 주장하지 않는다. 별도의 **matched-output** 비교에서는 selective trigger와 every-column trigger가 같은 7-column output bits를 반환하되 검사 횟수만 2 vs 8이다.

## 5. 문헌 및 검증 계보

- Saad & Schultz, *GMRES*, SIAM J. Sci. Stat. Comput. 7 (1986) 856–869, DOI 10.1137/0907058.
- CERFACS, *A Set of GMRES Routines*, TR_PA_97_49 (1997): projected vs original residual, https://cerfacs.fr/algor/reports/1997/TR_PA_97_49.pdf.
- Neumaier, *Global, rigorous and realistic bounds for dissipative differential equations I*, Computing 52 (1994): lognorm/ODE enclosures, https://arnold-neumaier.at/ms/ode.pdf.
- *Modular framework for backward error analysis of GMRES*, IMA J. Numer. Anal. (2026), DOI 10.1093/imanum/draf049.
- PR104 structural source and PR103 chemical6 source topology, source SHAs above.

이번 계산은 새로운 범용 no-go를 주장하지 않는다. PR103의 physical owner rate/clock, full30 Jacobian, physical33 Gram, 실제 accepted vector dense polynomial 및 continuous-event 증명은 미해결이며 `null`로 보존한다.
