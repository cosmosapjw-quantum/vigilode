# RODAS5P homotopy coding research loop 2 보고서

## 1. 목적과 범위

이 cycle은 실제 nonlinear homotopy solver를 구현하지 않았다. 앞선 탐색 연구에서 살아남은 partial-coupling 구조를 다음 수치실험이 신뢰할 수 있도록, 동일 Rust backend 위의 dense affine oracle·유한 nilpotent inverse·original-output certificate·결정론적 structural screen으로 고정했다.

보호 대상인 sequential RODAS5P, SABR5P, official coefficients, Krylov kernels, Fair A/B 계약은 변경하지 않았다.

## 2. 구현

새 module `rodas5p-integrators::homotopy`는

\[
D=I_s\otimes(M-h\gamma J),
\qquad
C=h(\beta-\gamma I_s)\otimes J
\]

및

\[
A_{\theta,\lambda}
=D-[\theta+\lambda(1-\theta)]C
\]

를 구성한다. `theta,lambda`는 private-field validated type으로만 생성된다.

제공 API:

- affine path/target operator and solve;
- homotopy/target residual;
- `Q=eta D^{-1}C` action;
- `sum_{m=0}^q Q^m D^{-1}` action;
- original target correction and weighted output WRMS certificate;
- deterministic design-check report.

## 3. 핵심 수치 결과

Official eight-stage screen:

| quantity | result |
|---|---:|
| endpoint max difference over theta | `0` |
| affine FLRH lambda spread | `0` |
| q=0 relative inverse error | `1.33937e-1` |
| q=1 relative inverse error | `1.28245e-3` |
| q=2 relative inverse error | `8.72020e-6` |
| q=7 relative inverse error | `2.49277e-16` |
| `||L^8||_F` | `6.14667e-20` |
| nonnormal `cond_1`, mu=0.5 | `2.98828` |
| nonnormal `cond_1`, mu=10 | `1.22222e8` |

`q=2`의 작은 affine error는 다음 실험의 seed일 뿐, nonlinear sufficiency나 성능을 의미하지 않는다.

## 4. 검증

- baseline: 26 tests pass;
- exact code-head validation before the documentation-only closeout commit: 34 workspace/doc tests pass;
- strict release Clippy/fmt/build: pass;
- deterministic CLI SHA-256: `cc6cb1c963e75415f5b35ba879d83b081a08fd7103e0f2cc27c234e63ec7743a`;
- failed timeout and schema-check validation attempts are preserved rather than rewritten as passes;
- deterministic CLI: two byte-identical outputs;
- independent NumPy reconstruction: 42/42 checks pass;
- maximum Rust–NumPy discrepancy: `1.15e-14`;
- noncommuting mass/Jacobian commutator norm: `1.67556`;
- independent diff review: two Important findings fixed by Red–Green tests, no remaining blocker.

## 5. 중요한 negative result

`det(A)=1`과 finite nilpotent inverse는 numerical benignity를 보장하지 않는다. Nonnormal shift screen에서 condition estimate가 eight orders 가까이 증가했다. 따라서 다음 solver는 homotopy residual이나 determinant가 아니라 original RODAS output defect로 accept해야 한다.

또한 이번 oracle은 full block LU를 사용한다. 이는 correctness reference이지 batched common-W implementation의 performance proxy가 아니다.

## 6. 현재 판정

PROMOTE:

- affine/reference partial-coupling API;
- exact/truncated inverse oracle;
- original-output certificate;
- deterministic design-check CLI;
- next-branch experiment contract.

HOLD:

- nonlinear continuation;
- q<=2 fast path;
- order/stability/DAE/performance claims;
- Anderson/Chebyshev/Padé;
- BDF/Radau and GCRO-DR joint tuning.

## 7. 다음 단계

다음 branch `homotopy-numerical-experiments-v0.5`에서는 first-principles nonlinear remainder adapter, fixed path schedules, original nonlinear certificate, sequential fallback, and equal-work SABR controls만 먼저 구현한다. 자세한 kill/promotion gate는 handoff 문서를 따른다.
