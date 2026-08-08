# RODAS5P Rust parity 재구축 및 strict Fair A/B 보고서

- 날짜: 2026-08-07 Asia/Seoul
- benchmark 구현 HEAD: `d34dee5f55106bfda9f7a4c6667dbdce531b8622`
- 상태: alpha 연구 checkpoint
- 범위 잠금: homotopy/continuation 경로는 의도적으로 제외

## 1. 요약 판정

Homotopy 이전에 개발했던 RODAS5P 연구 스택을 하나의 Rust workspace로 재구축했다.
GMRES, LGMRES, fixed-left GCRO-DR는 이제 같은 release executable, 같은 Rust vector kernel,
같은 operator abstraction, 같은 immutable trace를 사용하고, 동일한 외부 true-residual 기준과
operator-action budget으로 판정된다.

이에 따라 기존의 SciPy compiled path 대 Python prototype이라는 구현 불균형은 이번 비교에서
제거됐다. 실제 release binary의 dynamic dependency는 `libgcc_s`, `libm`, `libc`뿐이며 BLAS,
LAPACK, OpenBLAS, MKL shared library는 연결되지 않았다. `faer 0.24.4`도
`default-features = false`, feature `std`, `linalg`, `sparse-linalg`로 고정했다.

첫 strict benchmark의 결론은 recycling default 승격이 아니다. Preconditioner가 없는 core
suite 12개 셀 중 LGMRES/OFF가 9개에서 가장 빨랐고, 나머지 3개는 GMRES/OFF가 가장 빨랐다.
GCRO-DR persistence는 차원 64에서는 자기 OFF 경로를 개선했지만, 차원 128과 256에서는 현재의
고정 `(m,k)=(20,6)` 정책 아래 operator work와 wall time을 모두 증가시켰다.

## 2. Rust로 이식된 범위

- 공식 RODAS5P coefficient snapshot과 ROW 변환
- norm, failure type, work ledger
- explicit/matrix-free operator, mass matrix, shifted system
- `faer` direct LU oracle, Jacobi/direct preconditioner
- restarted GMRES, LGMRES, fixed-left GCRO-DR
- protected sequential RODAS5P와 adaptive controller
- structured block operator와 certified sequential fallback을 가진 SABR5P
- immutable Fair A/B trace, SHA-256 identity, external certificate, hard budget
- OFF/STAGE/PERSISTENT recycle lifetime
- release CLI `validate`, `trace`, `benchmark`

Rust source tree에는 homotopy 또는 continuation 구현이 없다.

## 3. Strict benchmark 계약

- 하나의 release binary: `rustc 1.94.1`, `faer 0.24.4`
- `opt-level=3`, fat LTO, `codegen-units=1`, `panic=abort`
- logical CPU 0에 affinity 고정
- 관련 thread 환경변수 전부 1로 고정
- 차원 `N=64,128,256`
- operator history: fixed, slow drift, abrupt change, rotating subspace
- trace당 4 timestep × 8 stage = 32 shifted systems
- seed 20260806, stiffness scale 1000, nonnormality 0.2
- restart 20, recycle dimension 6
- `rtol=10^{-9}`, `atol=10^{-12}`
- solve당 operator action budget 4000
- warm-up 1회, timed fresh-process repetition 5회
- 모든 solver에 같은 previous-oracle initial guess
- refresh와 최종 diagnostic action도 operator work에 포함

Previous-oracle guess는 deployable predictor가 아니라 recycling 효과를 분리하기 위한 의도적인
낙관적 control이다. Zero-guess ablation은 별도로 수행했다.

## 4. Core unpreconditioned 결과

| N | trace | wall 최속 경로 | GMRES 대비 speedup | GCRO-DR persistent op/GMRES | GCRO-DR persistent wall/GMRES | persistent op/GCRO-DR OFF | persistent wall/GCRO-DR OFF |
|---:|---|---|---:|---:|---:|---:|---:|
| 64 | fixed | `lgmres/off` | 1.059 | 0.912 | 1.606 | 0.858 | 0.884 |
| 64 | slow-drift | `lgmres/off` | 1.062 | 0.900 | 1.525 | 0.873 | 0.854 |
| 64 | abrupt | `lgmres/off` | 1.076 | 0.929 | 1.608 | 0.877 | 0.898 |
| 64 | rotating | `lgmres/off` | 1.126 | 0.886 | 1.520 | 0.879 | 0.889 |
| 128 | fixed | `lgmres/off` | 1.041 | 1.099 | 1.696 | 1.089 | 1.117 |
| 128 | slow-drift | `lgmres/off` | 1.021 | 1.105 | 1.709 | 1.095 | 1.116 |
| 128 | abrupt | `lgmres/off` | 1.086 | 1.061 | 1.612 | 1.056 | 1.107 |
| 128 | rotating | `lgmres/off` | 1.004 | 1.091 | 1.683 | 1.084 | 1.145 |
| 256 | fixed | `gmres/off` | 1.000 | 1.156 | 1.439 | 1.126 | 1.158 |
| 256 | slow-drift | `gmres/off` | 1.000 | 1.201 | 1.506 | 1.181 | 1.185 |
| 256 | abrupt | `lgmres/off` | 1.011 | 1.147 | 1.435 | 1.125 | 1.147 |
| 256 | rotating | `gmres/off` | 1.000 | 1.162 | 1.451 | 1.129 | 1.148 |

12개 셀 전체에서:

- LGMRES/OFF wall/GMRES 중앙값: `0.970`, 범위 `0.888–1.037`
- LGMRES/OFF operator/GMRES 중앙값: `1.021`
- GCRO-DR/OFF wall/GMRES 중앙값: `1.495`
- GCRO-DR/PERSISTENT wall/GMRES 중앙값: `1.565`
- GCRO-DR/PERSISTENT operator/GMRES 중앙값: `1.095`
- failure: `0`
- 최대 relative solution error: `3.574e-08`

GCRO-DR/PERSISTENT를 GCRO-DR/OFF와 비교한 `(wall, operator)` 중앙값은:

- `N=64`: `(0.886, 0.875)`
- `N=128`: `(1.116, 1.087)`
- `N=256`: `(1.153, 1.127)`

따라서 현재 고정 recycle 정책은 `N=64`에서는 유용한 방향을 포착하지만 차원 증가에 대해
강건하지 않다. 이는 Krylov recycling 일반의 반증이 아니라 admission과 `(m,k)` 조정이
필요하다는 증거다.

## 5. Jacobi preconditioner ablation

`N=64,128`에서 LGMRES/OFF의 GMRES 대비 wall ratio 중앙값은 `0.666`, operator ratio
중앙값은 `0.791`이었다. GCRO-DR/PERSISTENT도 operator work는 GMRES의 `0.814`까지 줄였지만,
wall ratio 중앙값은 여전히 `1.405`였다.

GCRO-DR/PERSISTENT를 자기 OFF 경로와 비교하면 wall ratio 중앙값 `0.868`, operator ratio
중앙값 `0.814`다. 즉 recycling은 GCRO-DR 자체의 수학적 work를 줄이는 데 성공하지만, 현재
harmonic-Ritz/orthogonalization 구현 비용 때문에 더 가벼운 LGMRES 경로의 wall time을 이기지
못한다.

## 6. Initial-guess ablation

`N=128`의 fixed/rotating trace에서 previous-oracle guess를 zero guess로 바꿨을 때 14개
solver/policy 셀의 wall ratio 중앙값은 `1.000`, operator ratio 중앙값은 `1.016`이었다.
Wall ratio 범위는 `0.940–1.084`였다.

현재 순위가 하나의 initial-guess 선택에 의해 만들어진 것은 아니다. 다만 predictor와
recycling은 같은 초기 residual 성분을 부분적으로 제거하므로, 실제 RODAS trace에서는 두 효과를
공동으로 평가해야 한다.

## 7. Rust 재구축으로 확정된 것

1. 기존 SciPy 대 Python wall-time 비교만으로 방법론을 순위화할 수 없다는 문제를 제거했다.
2. 같은 Rust 환경에서도 알고리즘 고유 비용은 남는다. GCRO-DR의 QR, harmonic-Ritz extraction,
   projection, refresh, 추가 state는 실제 비용이다.
3. 현재 dense manufactured trace에서는 LGMRES/OFF 같은 가벼운 augmentation 경로가 가장
   실용적이다.
4. unconditional persistence는 허용할 수 없다. Residual projection gain, subspace drift,
   update amortization을 보고 켜야 한다.
5. 이 checkpoint로 BDF, Radau, sparse PDE, MPI, GPU, singular mass matrix, 또는 제안한
   homotopy-vectorized stage coupling에 관한 결론을 내릴 수 없다.

## 8. 현재 구현의 한계

- vector kernel에 scalar Rust loop와 일부 operator-application당 임시 allocation이 남아 있음
- `(m,k)=(20,6)` 고정이며 calibration/holdout tuning 미수행
- preconditioner는 none과 Jacobi만 비교
- trace는 dense manufactured shifted system이며 실제 nonlinear RODAS trace가 아님
- 공유 VM에서 repetition 5회이므로 timing interval이 거침
- allocator, hardware counter, cache miss, memory bandwidth audit 미수행
- GCRO-DR는 fixed-left뿐이며 flexible variant가 아님
- sparse/multilevel physics preconditioner 없음

## 9. 판정

### PROMOTE — Rust 연구 정본

- 공통 Rust 수치환경
- immutable trace와 SHA 계약
- 외부 true-residual과 hard-budget enforcement
- protected RODAS5P, SABR5P fallback, GMRES, LGMRES, fixed-left GCRO-DR
- release CLI와 Rust-only Fair A/B harness

### HOLD — default solver 또는 성능 주장

- GCRO-DR default backend
- STAGE/PERSISTENT recycling의 무조건 활성화
- SciPy, PETSc, BDF, Radau, direct factorization에 대한 일반 우월성
- production readiness
- homotopy-based stage solver

## 10. 다음 gated 작업

1. **Allocation/kernel audit**: reusable solver workspace를 도입하고 hot-loop allocation을 제거한 뒤,
   같은 frozen trace를 재실행한다.
2. **Matched `(m,k)` sweep**: restart `20/40`, recycle dimension `3/6/12`를 calibration과
   holdout trace로 분리해 비교한다.
3. **Adaptive admission/reset**: projection gain, principal-angle drift, refresh rank/condition,
   실제 saved-cost를 기록해 OFF/STAGE/PERSISTENT/ADAPTIVE를 비교한다.
4. **Sparse·physics-preconditioned trace**: fixed block-Jacobi와 parameter-locked ILU를 추가하고,
   diffusion–reaction, advection–diffusion–reaction, kinetics, semilinear parabolic 문제에서 실제
   RODAS trace를 고정한다.
5. **Complete-integrator comparison**: linear accounting이 닫힌 뒤에만 동일 IVP tolerance와
   output/event policy에서 protected RODAS5P, BDF, Radau를 비교한다.
6. **Homotopy branch는 계속 닫아둔다**: Rust baseline, adaptive recycling, frozen nonlinear trace
   gate를 통과한 이후 별도 branch로 시작한다.
