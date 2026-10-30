# N4C 이론 연구와 native 코딩 연구 결과

## Scope/원본

실제 실행은 사용자가 첨부한 `vigilode-offline-install(2).zip` (2026-10-29 workspace)에 대해 Rust **1.99.1**과 **그 저장소의 `rodas5p-integrators::sequential_step`**를 호출했다. 기존 N4B의 원래 rank-2 복소 소스 방정식 RHS/Jacobian 함수 부분을 source-owned `Arc` 안에 복사하였다. PR #131 HEAD의 `sequential.rs`, `problem.rs`는 구버전 offline snapshot과 **Git blob이 다르므로**, PR #131의 최신 실제 바이너리를 검증했다고 간주하지 않는다. 원본 tableau fixture는 SHA256 `211255...`로 **byte-identical**이다.

## 물리 수식과 정확한 raw-U 변환 (derived)

\[
\dot a_j=(-\kappa_j+i\theta_j)a_j+i\sum_{\alpha=1}^{2}\chi_\alpha\phi_{\alpha j}|m_\alpha|^2m_\alpha,\qquad
m_\alpha=\sum_j w_j\phi_{\alpha j}a_j.
\]
양의 weights, 실제 binary64 represented χ와 w, \(\kappa_j\ge0\), 실수 \(\phi,\chi\)에서
\(E=\sum_jw_j|a_j|^2,\ \dot E=-2\sum_j w_j\kappa_j|a_j|^2\le0\). n=12/24/65에서 독립 RHS·Jacobian 방향미분을 수치 검산했다.

원래 tableau에서 strict lower triangular \(C\), \(\gamma\ne0\)일 때
\[
\Gamma=(\gamma^{-1}I-C)^{-1},\quad U=\Gamma K,\quad \alpha=A\Gamma,
\quad \Gamma^{-1}=\gamma^{-1}I-C.
\]
현행 native K equation과 원래 U equation은 위 변환으로 동치다 (Wolfram 3×3 symbolic check 4/4):
\[
R_i=U_i-h\gamma J_0U_i-\gamma\sum_{j<i}C_{ij}U_j-h\gamma f\!\left(y_0+\sum_{j<i}A_{ij}U_j\right)=0.
\]
재현된 실제 native `StepResult.stages` 8개의 K를 \(\Gamma\)로 변환했고, native core `directed::Interval`로 전산된 성분별 residual을 enclosure했다. 이 residual은 정확히 **표현된 binary64 stage 값과 coefficient**에 대한 대수 잔차 구간이며, 일반적인 forward stage error bound나 물리 전체 오차 구간과 동일하지 않다. 특히 \(\|(I-h\gamma J)^{-1}\|\) 및 stage nonlinear coupling의 증폭이 필요하다.

## 실제 범위

- n12, h=0.25: native 원래 8-stage residual upper \(9.1980\times10^{-14}\), embedded WRMS=253229.2134 -> REJECTED.
- n12, h=1e-8: residual upper \(1.3764\times10^{-20}\), WRMS=\(1.0621\times10^{-13}\) -> test-only accepted.
- n24/n65, h=.001: residual upper \(1.0935\times10^{-15}\) / \(1.3147\times10^{-15}\); WRMS=548.7348 /543.1565, 모두 REFUSED.
- 단일 source 8 RHS callbacks, 한 Jacobian, 8 native direct solves가 계수화된다. n12의 거부+수용에서 시도 16회=거부8+수용8 linear solves.
- 소스 대입, epoch/clock/state 변경, 재사용된 pending receipt, forged step length·endpoint·embedded WRMS·raw-U 잔차를 거부. pending abort도 실패 작업량 과금. n<=8 protected baseline 유지.
- 원래 N4 independent raw-U direct reference와 n12 첫 6개 endpoint 성분 비교: h=.25 최대 차이 \(1.3672\times10^{-16}\); h=1e-8 최대 차이 \(3.47\times10^{-18}\) (first6 only).

## Blind-style hostile review

1. **높은 residual precision ≠ time-acceptance:** h=.25에서 작은 residual도 embedded WRMS 큰 step은 거부. CLOSED for tested source.
2. **원본 source 위조·동시 시도:** 서명된(비암호학적 sealed-private) pending metadata와 Arc identity를 통해 거부. 실제 독립 작성자 리뷰 아님.
3. **involution:** 두 번째 sign basis \(\phi_2\to-\phi_2\)의 RHS/Jac은 bitwise unchanged. PASS in source scope.
4. **실제 native provenance:** 코어 coefficient loader는 PR131과 byte identity가 같지만 `sequential.rs`와 `problem.rs`는 다름. **latest PR131 native execution NOT_RUN**, 다음 직접 구현 의무.
5. **기존 빌드 strict Clippy:** User supplied Rust1.99.1에서 이전 core `damped_moments.rs`의 `too_many_arguments`, krylov `lgmres.rs`의 `manual_slice_fill`로 FAILED. 새 원본이나 이 기존 lint를 숨기지 않았고 원본 변경하지 않음.
6. **성능:** source-specific 8-stage의 accepted/rejected work를 계수화했지만 BDF·N4B와 전체 matched achieved-error trajectories/wall-time는 미검증. No speedup or production claim.

결론: **실제 이전 native core와 source-owned rank2 8-stage callback, original U residual, adaptive reference transaction은 성공**. 최신 PR131 source와의 완전한 동일성, production 물리 IVP 및 speedup은 HOLD.
