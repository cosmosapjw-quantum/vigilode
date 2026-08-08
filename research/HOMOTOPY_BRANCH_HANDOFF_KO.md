# Homotopy 기반 RODAS5P stage-coupling 연구 브랜치 handoff

Date: 2026-08-07 Asia/Seoul
Frozen predecessor tag: `rust-pre-homotopy-v0.2.0-alpha1`
Planned branch: `homotopy-stage-coupling-v0.3`

## 1. 출발점

이 handoff는 homotopy 구현 이전 Rust 정본을 고정한다. 출발 tag에는 다음이 포함된다.

- 공식 RODAS5P tableau와 protected sequential oracle;
- 동일 Rust kernel 위의 GMRES, LGMRES, fixed-left GCRO-DR;
- structured all-at-once block operator와 SABR5P certificate/fallback;
- immutable trace, operator budget, true-residual certificate;
- allocation-free shifted operator와 reusable Krylov workspaces;
- Rust-only Fair A/B harness.

`crates/*/src`에는 homotopy 또는 continuation 구현이 없다. 새 브랜치는 이 tag에서 분기하며,
기존 oracle과 failure semantics를 변경하지 않는다.

## 2. 새 브랜치의 핵심 연구 질문

RODAS5P 전체 stage vector를

\[
K=(k_1,\ldots,k_8)
\]

로 묶으면 원래 순차 계산은 lower-triangular nonlinear residual

\[
R_i(K)=Wk_i-\Phi_i(K_{<i})=0,
\qquad W=M-h\gamma J_n
\]

의 정확한 forward propagation이다. 새 브랜치의 질문은 이 causal dependency를 없앤다고
주장하는 것이 아니라, homotopy를 통해 정확한 8-depth propagation을 소수의 batched
approximate propagation round로 압축할 수 있는지 검증하는 것이다.

우선 후보는 predictor-anchored coupling homotopy

\[
H(K,\lambda)
=(1-\lambda)(K-\widehat K)
+\lambda (I_8\otimes W^{-1})R(K)=0,
\qquad 0\le\lambda\le1
\]

이다. Stage coupling derivative는 strictly block-lower-triangular이므로 nilpotent 구조를
갖는다. 이 구조를 이용해 exact tangent, truncated Neumann tangent, secant/dynamic-inverse
continuation을 비교한다.

## 3. 첫 구현 범위

1. homotopy residual과 path diagnostics만 추가한다.
2. protected sequential RODAS5P를 oracle/fallback으로 유지한다.
3. stage RHS와 common-`W` multiple-RHS solve의 batched interface를 사용한다.
4. `p=0,1,2` truncated coupling propagation을 비교한다.
5. final acceptance는 original RODAS5P residual과 embedded LTE-relative defect로 판정한다.
6. 실패한 speculative path는 timestep state와 recycle state를 commit하지 않는다.
7. Chebyshev/Lagrange는 RODAS stage abscissa가 아니라 homotopy path `K(lambda)`에만 시험한다.

## 4. 대조군

같은 Rust executable과 trace에서 최소한 다음을 비교한다.

- protected sequential RODAS5P;
- SABR5P fixed-point/block correction;
- homotopy-only;
- homotopy + small-depth Anderson;
- homotopy warm-start + protected sequential fallback.

R10-B `(m,k)` tuning과 adaptive GCRO-DR admission은 폐기하지 않고, homotopy branch의
공통 비교 단계로 이월한다. 먼저 homotopy iteration의 비용 장부를 만든 뒤에야 Krylov tuning과
동시에 비교한다.

## 5. 성능 장부

다음 비용을 서로 섞지 않는다.

\[
N_f,\quad N_{Jv},\quad N_{W^{-1}v},\quad
N_{\rm homotopy\ rounds},\quad N_{\rm fallback},
\]

\[
T_{\rm batch\ RHS},\quad T_{W\text{-solve}},\quad
T_{\rm path},\quad T_{\rm certificate},\quad T_{\rm total}.
\]

Fast path 승격에는 false acceptance 0, 5차 global convergence 보존, stiff-decay regression 없음,
and holdout median wall speedup `>=1.15x`를 요구한다.

## 6. 초기 prior art anchor

- D. A. Brown and D. W. Zingg, “Matrix-free monolithic homotopy continuation with
  application to computational aerodynamics,” Numerical Algorithms 78 (2018),
  DOI `10.1007/s11075-017-0424-8`.
- P. J. van der Houwen and B. P. Sommeijer, “Iterated Runge–Kutta Methods on Parallel
  Computers,” SIAM J. Sci. Stat. Comput., DOI `10.1137/0912054`.
- P. J. van der Houwen and B. P. Sommeijer, “Parallel-iterated Runge-Kutta methods for
  stiff ordinary differential equations,” J. Comput. Appl. Math. 45 (1993),
  DOI `10.1016/0377-0427(93)90271-C`.
- P. Munch, I. Dravins, M. Kronbichler, and M. Neytcheva, “Stage-Parallel Fully Implicit
  Runge–Kutta Implementations with Optimal Multilevel Preconditioners at the Scaling Limit,”
  SIAM J. Sci. Comput. 46 (2024), DOI `10.1137/22M1503270`.

이 문헌들은 각각 Newton-free/matrix-free homotopy, 병렬 stage iteration, coupled-stage
uncoupling, stage-parallel block linear algebra를 제공하지만, protected RODAS5P의 공통 `W`,
lower-triangular nilpotent coupling, LTE-relative certificate와 transactional fallback의 결합은
별도로 검증해야 한다.

## 7. 금지 주장

- homotopy가 causal dependency를 수학적으로 제거한다;
- finite homotopy round가 자동으로 원래 RODAS5P의 5차/L-stability를 보존한다;
- synthetic batch speedup이 실제 GPU/CPU speedup을 뜻한다;
- 현재 baseline이 BDF/Radau보다 일반적으로 빠르다;
- 새 방법의 신규성이 이미 확정됐다.
