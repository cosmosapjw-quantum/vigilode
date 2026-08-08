# R10-A — Rust allocation/kernel audit 보고서

Date: 2026-08-07 Asia/Seoul  
Baseline tag: `rust-parity-v0.1.0-alpha1`  
Baseline HEAD: `cbb29dfb32ef4a358f5e77f4320190534543fa36`  
Benchmark code HEAD: `2d3fcaffa688e2cb99d7e79d9f59536fae4dae06`  
Branch: `rust-adaptive-v0.2`

## 1. 요약 판정

이번 cycle은 Rust-only 공정 비교의 수치 경로를 바꾸지 않은 채 반복 heap allocation을 줄이고,
그 효과를 이전 tagged binary와 paired A/B로 검증하는 데 한정했다.

핵심 결과는 다음과 같다.

1. `ShiftedOperator::apply`는 생성 이후 allocation-free가 됐다.
2. GMRES, LGMRES, fixed-left GCRO-DR에 reusable workspace API를 추가했다.
3. Fair A/B `SolverSession`은 recycle state를 reset하더라도 scratch workspace는 보존한다.
4. 252개 paired cell에서 baseline과 current의 총 operator work는 전부 정확히 같았다.
5. 최대 solution-error 차이는 `5.294e-23`, 양쪽 failure는 모두 0이었다.
6. GMRES와 LGMRES는 중앙값 기준 약 1% 내외의 wall-time 개선을 보였다.
7. GCRO-DR는 거의 중립이었다. 남은 projected eigensolve, QR, real-subspace 구성과 recycle update
   allocation이 workspace 이득을 상쇄했다.

따라서 **allocation-free shifted operator와 reusable workspace infrastructure는 PROMOTE**하지만,
**default solver/recycling policy 변경과 GCRO-DR 성능 승격은 HOLD**한다.

## 2. Runtime interruption recovery

R10-A 구현 도중 working runtime이 초기화되어 uncommitted source와 worktree가 사라졌다.
이를 정상 연속 실행으로 취급하지 않았다. 검증된 Git bundle에서
`cbb29dfb32ef4a358f5e77f4320190534543fa36`을 복원하고 새 isolated worktree를 만든 뒤,
baseline release test를 다시 통과시킨 후 TDD cycle을 처음부터 재구성했다.

Durable recovery evidence:

- `research/recovery/RUNTIME_INTERRUPTION_RECOVERY_2026-08-07.md`
- `research/recovery/RECOVERY_INVENTORY_2026-08-07.txt`

중단 이전의 uncommitted 구현은 완료 증거로 사용하지 않았다.

## 3. 구현 변경

### 3.1 Allocation-free shifted operator

기존 `ShiftedOperator::apply`는 매 적용마다 임시 `Jx: Vec<f64>`를 만들었다. 이제 출력 buffer에
`Jx`를 직접 쓰고, mass-matrix row product 또는 identity mass를 같은 buffer에 결합한다.


생성 이후 contract는 다음과 같다.

```text
128 consecutive ShiftedOperator::apply calls
heap allocations = 0
```

이 변경은 operator action의 부동소수점 식과 counting category를 바꾸지 않는다.

### 3.2 Reusable Krylov workspaces

공통 buffer와 Arnoldi storage를 다음 workspace로 분리했다.

```text
GmresWorkspace
LgmresWorkspace
GcrodrWorkspace
```

공유되는 장기 buffer는 다음을 포함한다.

- solution, residual, operator output;
- left-preconditioned vector;
- scratch vectors와 coefficient buffer;
- Arnoldi basis와 search directions;
- Hessenberg storage;
- least-squares RHS와 correction vector.

기존 public wrapper API는 유지하고, Fair A/B runtime에서는 명시적 workspace API를 사용한다.

### 3.3 Transactional state와 workspace lifetime 분리

`OFF` 또는 `STAGE` policy가 recycle state를 지울 때 다음은 reset된다.

- LGMRES outer augmentation vectors;
- GCRO-DR `U,C` recycle pair;
- state generation과 operator-image validity.

반면 scratch workspace capacity는 보존된다. 실패한 solve는 이전과 마찬가지로 algorithmic state를
commit하지 않는다. 따라서 performance state와 scientific state의 lifetime이 분리됐다.

### 3.4 Allocation audit binary

일반 benchmark binary에는 allocator instrumentation을 넣지 않았다. 별도의
`rodas5p-allocation-audit` binary만 `std::alloc::System` 위에 atomic event counter를 둔다.
측정 범위는 다음처럼 잠갔다.

```text
full_trace_run_including_result_materialization
```

따라서 allocation-audit binary의 wall time은 production benchmark 지표가 아니다. Atomic counting
자체의 overhead가 들어가므로 allocation event/bytes 진단에만 사용한다.

## 4. Paired A/B 설계

### 4.1 고정 조건

```text
Rust:              1.94.1
release profile:   opt-level=3, fat LTO, codegen-units=1, panic=abort
CPU affinity:      taskset -c 0
thread variables:  OPENBLAS/OMP/MKL/RAYON/VECLIB/NUMEXPR = 1
preconditioner:    none
restart m:         20
recycle k:         6
rtol:              1e-9
atol:              1e-12
operator budget:   2000 per solve
warmup:            1
paired repetitions: 3
```

비교 binary:

```text
baseline SHA-256: d9677f613d065f82fee5b4ef65d340c554e4b7428206ae636eb2cc791196ce3e
current  SHA-256: 430a75c6be1eaa5727f2ae7773e71bf5f0fbb8abd6b56ba30baac87a5b0204f4
```

각 repetition에서는 baseline/current 실행 순서를 교대했다. 동일 trace, seed, external initial guess,
preconditioner, tolerance와 budget을 사용했다.

### 4.2 Trace grid


사용한 immutable trace set:

```text
dimension: 64, 128, 256
kind:      fixed, slow-drift, abrupt, rotating
cells:     GMRES/OFF
           LGMRES/OFF, STAGE, PERSISTENT
           GCRO-DR/OFF, STAGE, PERSISTENT
```

총 paired row 수는

\[
3\times3\times4\times7=252
\]

이다.

## 5. 수치 경로 불변성

252개 paired row 전체에서

\[
\Delta N_{Wv}^{\rm total}=0
\]

이었다. Baseline과 current의 failure는 모두 0이고,

\[
\max|E_{\rm current}-E_{\rm baseline}|
=5.294\times10^{-23}
\]

이다. 따라서 이번 wall-time 차이는 Krylov path, tolerance 또는 recycle decision 변화가 아니라
allocation/memory reuse 변경의 효과로 해석할 수 있다.

## 6. Paired wall-time 결과

아래 비는

\[
\rho_T=\frac{T_{\rm current}}{T_{\rm baseline}}
\]

이며, 1보다 작을수록 current가 빠르다. 첫 표는 36개의 개별 paired ratio에 대한 통계다.

| solver/lifetime | median | Q25 | Q75 |
|---|---:|---:|---:|
| GMRES/OFF | 0.9918 | 0.9730 | 0.9985 |
| LGMRES/OFF | 0.9907 | 0.9708 | 1.0079 |
| LGMRES/STAGE | 0.9929 | 0.9669 | 1.0152 |
| LGMRES/PERSISTENT | 0.9836 | 0.9670 | 1.0057 |
| GCRO-DR/OFF | 0.9981 | 0.9833 | 1.0109 |
| GCRO-DR/STAGE | 1.0151 | 0.9933 | 1.0383 |
| GCRO-DR/PERSISTENT | 1.0056 | 0.9937 | 1.0190 |

Trace마다 3회 wall time의 중앙값을 먼저 취한 뒤 12개 scenario ratio를 집계해도 결론은 같다.

| solver/lifetime | scenario-median ratio의 median |
|---|---:|
| GMRES/OFF | 0.9918 |
| LGMRES/OFF | 0.9889 |
| LGMRES/STAGE | 0.9935 |
| LGMRES/PERSISTENT | 0.9761 |
| GCRO-DR/OFF | 0.9973 |
| GCRO-DR/STAGE | 1.0175 |
| GCRO-DR/PERSISTENT | 1.0065 |

즉 workspace 이식은 GMRES/LGMRES에서 작지만 일관된 중앙값 개선을 보였지만, GCRO-DR에서는 거의 중립이다.
이 차이는 promotion gate `>=1.15x`에 한참 못 미치므로 default 선택을 바꿀 근거가 아니다.

### 6.1 Exact-head rerun과 중단 기록

최종 benchmark code HEAD에서 첫 전체 재실행을 시도했을 때 container execution wrapper가
`n=128/abrupt` baseline invocation 중 장시간 정지한 작업을 종료했다. 해당 partial output은 결과에
사용하지 않았다. 같은 invocation을 독립적으로 다시 실행하면 baseline `1.03 s`, current `1.00 s`로
정상 종료했고 operator work와 해오차도 일치했다.

이후 각 invocation에 `120 s` guard를 둔 전체 72-invocation 재실행을 수행했고 모두 성공했다.
현재 CSV·summary·raw archive는 이 exact-head guarded rerun만 사용한다. 최초 중단 log도
`R10A_EXACT_RERUN_INTERRUPTED.log`로 보존했다.

## 7. Allocation audit

다음 값은 current binary의 6개 scenario
`n={64,128,256} × {fixed,rotating}`에서 얻은 중앙값이다.

| solver/lifetime | allocation events | allocated bytes | total operator actions |
|---|---:|---:|---:|
| GMRES/OFF | 35,033 | 23,623,808 | 3,392 |
| LGMRES/OFF | 39,684 | 21,817,648 | 3,439 |
| LGMRES/STAGE | 49,181 | 31,310,322 | 3,639 |
| LGMRES/PERSISTENT | 51,355 | 32,982,760 | 3,752 |
| GCRO-DR/OFF | 160,771 | 46,563,264 | 3,419 |
| GCRO-DR/STAGE | 176,103 | 51,014,242 | 3,585 |
| GCRO-DR/PERSISTENT | 183,686.5 | 52,930,456 | 3,715 |

GCRO-DR/OFF는 GMRES/OFF보다 allocation event가 약 `4.59x`, bytes가 약 `1.97x`다.
PERSISTENT는 각각 약 `5.24x`, `2.24x`다.

이미 공통 Arnoldi vectors와 scratch를 재사용하므로, 남은 주요 후보는 다음이다.

- augmented relation과 projected matrix의 반복 생성;
- QR/rank-revealing orthonormalization;
- generalized harmonic-Ritz eigensolve;
- complex-pair를 real invariant plane으로 변환하는 temporary vectors;
- recycle-state update에서 `Vec<Vec<f64>>` 재구성;
- result document materialization.

이 표만으로 어느 함수가 몇 allocation을 발생시켰는지는 알 수 없다. 다음 단계에서는 sampling
profile과 함수 경계별 allocation/timing counters로 원인을 분해해야 한다.

## 8. 과학적 해석

### 8.1 확인된 것

- shifted operator hot apply에서 불필요한 allocation을 제거할 수 있다.
- Krylov scratch lifetime을 solve보다 길게 두어도 수치 경로와 work ledger를 보존할 수 있다.
- GMRES/LGMRES에서는 이 변경이 작은 wall-time 개선으로 연결된다.
- GCRO-DR의 남은 비용은 공통 Arnoldi scratch보다 method-specific projected algebra에 더 가깝다.

### 8.2 확인되지 않은 것

- GCRO-DR가 production에서 느리다는 일반 명제;
- sparse/large-scale 문제에서도 allocation 비율이 같다는 주장;
- `faer` 내부 scratch가 최적이라는 주장;
- adaptive recycling의 효과;
- BDF/Radau 대비 RODAS5P 전체 적분기 우위;
- homotopy stage solver의 성능.

## 9. 판정

### PROMOTE

- allocation-free `ShiftedOperator::apply`;
- reusable GMRES/LGMRES/GCRO-DR workspace API;
- Fair A/B session-level workspace reuse;
- recycle reset과 workspace lifetime의 분리;
- dedicated allocation-audit binary와 schema.

### HOLD

- default solver 변경;
- unconditional STAGE/PERSISTENT recycling;
- GCRO-DR performance promotion;
- allocation-audit wall time의 benchmark 사용;
- homotopy branch 개시.

## 10. 후속 작업의 보류와 branch 전환

최종 user decision에 따라 다음 항목은 homotopy-based stage-coupling prototype 이후의 joint comparison까지 보류한다.

- GCRO-DR hotspot localization;
- R10-B `(m,k)` calibration/holdout, with \(m\in\{20,40\}\), \(k\in\{3,6,12\}\);
- adaptive `G_n,D_n` admission/reset;
- nonlinear frozen traces and complete-integrator ranking.

현재 branch는 pre-homotopy baseline으로 닫고, 다음 active branch에서는 protected sequential/SABR/Krylov
기준선을 보존한 채 lower-triangular nonlinear stage coupling용 homotopy method를 별도 구현한다.

## 11. Durable evidence

- `research/results/R10A_PAIRED_BASELINE_CURRENT.csv`
- `research/results/R10A_ALLOCATION_AUDIT.csv`
- `research/results/R10A_KERNEL_AUDIT_SUMMARY.json`
- `research/receipts/R10A_PAIRED_RUN.sh`
- `research/receipts/R10A_ALLOCATION_RUN.sh`
- `research/receipts/R10A_PAIRED_RUN.log`
- `research/receipts/R10A_ENVIRONMENT.json`
- external raw archive: `RODAS5P_Rust_R10A_Raw_v0.2.0-alpha1.tar.zst`
- raw archive SHA-256: `2650b170a7c06b20c5e5b77f47c216128ef2a52b113d94bbc20cb981386b00fc`

## 12. Current branch closure

사용자 결정에 따라 R10-A 이후의 GCRO-DR hotspot tuning, `(m,k)` calibration 및 adaptive recycling은
현재 branch에서 계속하지 않는다. R10-A의 quantitative result와 source를 pre-homotopy baseline으로
동결하고, 다음 독립 branch에서 homotopy-vectorized stage-coupling method를 먼저 개발한다.

연기된 tuning은 폐기된 것이 아니다. Homotopy prototype이 residual/fallback gate를 통과한 뒤
sequential, recycled-sequential, SABR와 homotopy paths를 동일 Rust binary와 frozen traces에서 함께
튜닝·비교한다. 상세 handoff는 `research/HOMOTOPY_BRANCH_HANDOFF_KO.md`에 있다.

## 12. 최종 closeout 상태

R10-A 이후의 미세한 GCRO-DR tuning은 이번 branch에서 더 진행하지 않는다. Canonical 252-row
paired benchmark는 benchmark implementation commit `2d3fcaffa688e2cb99d7e79d9f59536fae4dae06`과
raw archive에 고정한다. 이후 source 변화는 strict Clippy를 만족하기 위한 lint annotation과
`Option<&mut T>` 처리의 동치 단순화이며, benchmark 표를 재해석할 수 있는 알고리즘 변화는 없다.

최종 closeout에서는 최신 tag에 대해 format, Clippy, release test/build, CLI smoke, dynamic dependency,
homotopy-source absence, raw-archive hash와 clean-archive build를 새로 검증한다. 최종 검증 log와
artifact SHA-256은 Git tag 밖의 delivery package에 저장한다. 이는 validation log를 commit하여
검증 대상 HEAD가 다시 바뀌는 자기참조를 피하기 위함이다.

Deferred work:

- GCRO-DR projected-algebra hotspot localization;
- `(m,k)` calibration/holdout;
- adaptive recycle admission/reset;
- actual nonlinear frozen RODAS traces.

이 항목들은 폐기되지 않으며 `homotopy-stage-coupling-v0.3` 브랜치에서 homotopy 후보와 같은 Rust
환경으로 함께 비교한다. 자세한 범위는 `research/HOMOTOPY_BRANCH_HANDOFF_KO.md`에 고정했다.
