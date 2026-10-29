# VIGILODE-PREFIX02: 전처리 GMRES 원래 잔차 인증, 실행 보고서

2026-10-29. 기준 PR105 `2a815e98a277c3b9ee807f6098f3d1950bf81985`. 기존 PR104/103은 source-lock 참조이며 병합하지 않았다.

## 핵심 물리·수학 결과

`A=I−σJ`에서 좌전처리는 `M^-1 A`, 우전처리는 `A M^-1`의 Arnoldi를 수행한다. 좌전처리의 작은 projected residual은 원래 `b−Ax`의 작은 잔차를 보증하지 않는다. `A=I`, `M^-1=10^-12 I`, `b=(1,0)`이면 preconditioned residual은 `10^-12`, 원래 잔차는 `1`인 명시적 반례를 구성했다.

원래 저장된 point-matrix `J`와 `σ≥0`, `μ_w(J)≤m`, `1−σm>0`에서는 `||(I−σJ)^-1||_{w,∞}≤(1−σm)^-1`. Outward original residual `ρ`와 `Kρ`가 각각 `τ_orig`, `ε_stage` 이하여야만 출력 후보를 채택한다. Wolfram exact rational 2×2 우/좌 residual identity 검산에서 두 차 모두 zero vector를 반환했다. 이 정리는 source/epoch/clock/weight가 고정된 **exact represented point matrix** 범위다.

## 구현·검증 (이번 실행)

- Rust 1.94.1, `cargo test --locked --offline --all-targets`: **25 PASS** (기존 PREFIX01 9 + 새 좌/우/거절/재시작 16), exit0.
- `cargo clippy --locked --offline --all-targets -- -D warnings`: PASS, exit0. `cargo fmt --all -- --check`: PASS, exit0.
- Rust binary64 `precond_bits` 40 records의 `μ,ρ,K,Kρ,||x−x*||` 및 160 component inequalities를 Python `Fraction`으로 독립 재구성: **360/360 PASS**, exit0.
- 부정 대조군: stale epoch at final commit, source/clock/norm mismatch, zero/nonfinite diagonal, underflowed left residual, zero budget, negative inverse margin, restart refusal, warm start, zero shift, nonnormal triangular. 실패는 출력 불변을 유지.
- 최초 누락된 bin target로 test가 실패한 기록, 첫 Clippy 1건 및 다음 Clippy 9건, 대응 수정 후 최종 PASS를 보존. numerical tolerances는 변경하지 않음.

## 실행 시간: synthetic dense point matrix, seven paired blocks

고정 `τ_orig=ε_stage=1e-9`, 동일 RHS와 모델, 20회/measurement, 7 blocks, 3 dimensions, 3 arms, 각 arm 블록당 2회, 총 2520 timed *linear solves*. 동일 accuracy **허용 예산**을 충족하지만 achieved errors는 다르므로 matched-achieved-error 비교는 아니다. 시간비는 identity/right 또는 identity/left이고 1 초과 시 prototype에서 전처리가 빠르다.

| n | Arnoldi identity→left/right | Identity/left median [min,max] | Identity/right median [min,max] | max achieved error identity/left/right |
|---:|---|---:|---:|---|
| 16 | 9 → 1/1 | 1.1508 [1.0460,1.3463] | 1.2029 [1.0625,1.5721] | 4.801e-11 / 1.110e-16 / 0.000e+00 |
| 64 | 15 → 1/1 | 1.2905 [0.9587,1.4252] | 1.2645 [1.1684,1.3635] | 1.607e-10 / 1.110e-16 / 0.000e+00 |
| 128 | 20 → 1/1 | 1.3961 [1.3370,1.5610] | 1.4109 [1.3380,1.5770] | 2.761e-10 / 1.110e-16 / 0.000e+00 |

특히 n=128에서 20→1 column에도 시간비는 약 1.4배다. 이 prototype은 residual certificate를 **dense O(n²)**로 다시 계산하기 때문에 절약된 Arnoldi 비용 일부를 인증 비용이 소모한다. 일부 n=64 block은 left가 더 느리다. **이것은 PR104 native full-IVP 성능 향상도, BASS30 물리 오차 개선도 아니다.** 원자료 `evidence/bench.csv`, `bench_summary.json`, `prefix02_ratios.png`에 모든 블록과 실제 달성 오차를 보존했다.

## GitHub 및 CI 조사

PR105 workflow run `40806732910`의 research-process는 41개 checker 테스트 PASS 후 **`research/prefix01_original_residual_20261029/PREREGISTRATION.md` 부재와 ledger 행 부재**로 FAIL했다. authority step은 SKIPPED였다. 이 과거 결함을 새 사전등록인 것처럼 재작성하지 않는다. 이번 PREFIX02는 실제 실행 전 PREREGISTRATION.md를 GitHub에 게시한 신규 노드다. 신규 결과는 유효한 append-only ledger 행으로 연결하되, 기존 Q-LEDGER authority 실패와 소진된 교정 예산은 그대로 보존한다.

실제 PR104 `crates/rodas5p-krylov/src/gmres_into.rs`의 `solve_gmres_into_right_monitored`는 매 column마다 least-squares/candidate/preconditioner를 구성한다. 이번 선택적 trigger-before-backsolve 코드를 그대로 wrapper로 쓰면 비용 동치가 성립하지 않는다. Native 수정은 별도 소스·빌드·CI가 필요하다.

## 판정

- **DERIVED / SYMBOLICALLY CHECKED**: 좌·우 전처리 원래 잔차 항등식, lognorm resolvent bound, 좌전처리 false-convergence 반례.
- **IMPLEMENTATION-VERIFIED, SCOPED**: standalone point dense diagonal-preconditioned Rust prototype, 25 tests, 360 Fraction inequalities, 7 paired synthetic blocks.
- **NOT_RUN**: native subagent R6/C9, native PR104 workspace build, physical source/rate/clock/capsule binding, actual full-IVP matched accuracy and production CI.
- **DAG**: original VigilODE `7/23 inherited scoped`, central REI `28/88`, auxiliary PREFIX01 `0/1`, new PREFIX02 `0/1 pending native+independent`; production **HOLD**.

백업은 GitHub stage별 커밋과 최종 source/evidence ZIP을 분리한다. Drive/Dropbox의 실제 ACK와 재다운로드 해시 검증은 별도 DELIVERY_RECEIPTS.json에서 판정한다.
