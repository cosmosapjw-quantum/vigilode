# VigilODE 커널·스펙트럼 기초 run 보고서 (2026-11-24/25)

브랜치 `research/stage-lattice-referee-20261124`. 직전 run은 `research/stage_lattice_referee_20261124`.
수치는 모두 이 run의 결과 파일에서 가져왔다. 성능 수치는 2-vCPU 호스트의 wall time 진단값이다.
owner host의 사전 등록 셀(PLAN.json KS-N1, KS-N3)은 아직 측정하지 않았다.

## 1. owner 결정 세 건의 실행 결과

**SL-N12 "기본 경로로 올려줘" (commit 3f9762a1).**
V21 tight 커널의 고정 오프셋 산술을 보호 banded 드라이버의 `BandedKernel::FixedWidth2`로 옮겼다.
facade의 (2,2) band는 endpoint와 dense output 두 경로 모두 이 커널을 쓴다.
- 연산 순서, pivot tie, 계수 연산 수가 Indexed와 같다.
- 단위 테스트는 기존 parity 테스트(n = 3–100)와 새 테스트로 비교했다. 새 테스트는 대각이 약해 모든 열에서 pivot이 바뀌는 행렬과, 시작·내부·꼬리에 singular 열이 있는 행렬을 Slices 커널과 비교한다. pivot, factor, 해, 오류 문자열, 계수 연산이 비트 단위로 같았다.
- golden 파일 세 개에서 76개 값이 바뀌었는데, 모두 `entry_point`와 `driver_id` 라벨이다. 상태 비트와 카운터는 하나도 바뀌지 않았다(`results/SL_N12_GOLDEN_LABEL_DIFF.json`).
- 호스트 wall time 비율(FixedWidth2/Indexed, Brusselator (2,2) 전체 풀이)은 n=1e4에서 0.769, n=1e5에서 0.776이다.

(2,2)가 아닌 half width < 6 band는 Indexed에 남겼다. 그 폭에서 같은 비트를 내는 대안은 Slices뿐이고, V22 폭 스캔에서 Slices가 더 느렸다. dense-output 쌍둥이는 이제 FixedWidth2를 받는다(`rodas5p-fast-banded-dense-v1-fixed2`). ExactCache는 계속 거부한다.

**SL-N6 "cargo feature" (commit 67b71bd2).**
`research-gates` feature를 rodas5p-core, rodas5p-integrators, vigilode-coupled에 두었다.
- 기본 빌드에서 24개 모듈, 19,750줄이 빠진다.
- 각 crate는 자기 자신을 dev-dependency로 두어 feature를 켜므로 자체 테스트·예제가 그대로 빌드된다. fair-ab와 cli는 feature를 명시적으로 켠다.
- inventory가 파일 이름만 보고 research-gate로 분류한 다섯 모듈은 기본 경로가 import하므로 기본 빌드에 남겼다(kill criterion의 fallback 그대로): comparator_fidelity, controller_telemetry, g4_s5b0_inner_tolerance, problems, homotopy_experiments.
- 통합기 라이브러리 release 빌드 시간은 43.2 s로, 기존 기본 빌드 60.5 s 대비 −29%이다.
- Cargo.lock 변경은 cargo가 쓴 self dev-dependency 세 줄뿐이다.

**SL-N5 "새 의존성으로 추가".** rustfft 6.4.1(MIT OR Apache-2.0)을 추가했다. 기존 lock에 이미 있던 num-complex, num-traits 외에 num-integer, primal-check, strength_reduce, transpose가 새로 들어왔다. 이 의존성 위에 아래 2절의 이론과 3절의 코드를 올렸다.

## 2. 이론적 토대 (THEORY.md)

흩어져 있던 사이드 스레드를 하나의 대상인 RODAS5P 단계 연산자 `W(z) = zI − J`(`z = 1/(hγ)`)에 대한 명제로 다시 썼다.

| 스레드 | 대상 | 코드에서의 쓰임 | 상태 |
| --- | --- | --- | --- |
| 푸리에/convolution | 경도 회전군 Z_np의 isotypic 분해 | 모드별 삼중대각 블록으로 단계 행렬 분해 | D, T(F3) |
| involution | 반사(W_m = W_{np−m}), 켤레(Hermitian 스펙트럼) | factorization 수 절반, 고리 두 개를 FFT 한 번에 | D, T(F3, F8) |
| 대수위상 | 이산 Hodge 형식, H^0 = 상수 | 단계 질량 항등식 `(z−c)M(x) = M(b)`, 질량 보존 | D, T(F1, F2, F7) |
| 특이기하 | 단계 pencil 특이 궤적 Σ = spec J, 극점 chart 특이성 | inertia로 Σ를 넘은 단계 준비를 거부, 극점 강성이 대각에만 들어감 | D, T(F4, F6) |
| 대수기하(관성) | Sylvester 관성 법칙, Sturm | Thomas pivot 부호 = spec J ≥ z인 고유값 수 | D, T(F4, Rust 테스트) |
| 동역학계 | A-내적 로그노름, Gershgorin | 축소성 명제, `spectral_bounds`(ρ̄, DOPRI5 단계 한계) | D, T(F5) |
| 단계 격자 | 임계 경로 | 단계 안 병렬 폭 = 대칭군의 기약 블록 수, 8단계 순차성은 여전히 N | D/P |

`scripts/verify_foundation.py`는 `spatial.rs`와 같은 계수로 연산자를 만들고 F1–F8을 검사한다. 모두 통과했다(`results/FOUNDATION_CHECKS.json`).
- F4: 무작위 40회 시행에서 pivot 수와 고유값 수가 모두 일치했다(최대 64개 교차).
- F6: 극/적도 방위각 rate 비가 nt=256에서 예측값 4nt²/π²의 0.99997배였다.
- F5: Gershgorin 경계 대비 실제 스펙트럼 반경은 0.982였다.

## 3. 코드: 구조-정확 단계 풀이기

`vigilode::spatial::SeparableStages`(`DiffusionGrid::separable_stages(c)`)와 `VigilODE::solve_separable`을 추가했다. 보호 client 드라이버 안에서 돈다.
- 구(sphere): 고리 두 개씩 FFT 한 번(켤레 involution 분리), ⌊np/2⌋+1개 모드별 Thomas(실수 RHS 둘), 역변환.
- 주기 상자: 다차원 FFT, 대각 나눗셈, 역변환.
- 준비할 때마다 inertia 인증을 계산한다. eigenvalue ≥ z가 있으면 typed linear-solve 오류로 거부하고, 드라이버가 h를 줄인다.
- 풀이할 때마다 단계 질량 항등식을 기록한다.
- `Verified` 모드(기본): 문제 자신의 JVP로 참 잔차를 만들고, 결손 교정을 최대 4회 돈다. 실패하면 거부한다.
- `Trusted` 모드: 질량 검사만 한다.
- 범위 밖 요청은 callback 전에 거부한다: dense output은 facade validation이 먼저 거부하고, 차원이 다르거나 Verified인데 JVP가 없으면 `SeparableStagesRequireMatrixFreeRoute`로 거부한다.

테스트(`crates/vigilode/tests/spatial_separable.rs`, 5개 통과):
- **T1/T2.** 7개 격자(홀수 nt, 홀수 np, 3D 상자 포함)와 z ∈ [1e-3, 1e4]에서 backward error 최대 3.4e-16, 질량 결손 최대 5.1e-17.
- **T3.** 독립 기준값은 RODAS5P도 FFT도 쓰지 않는 dense 행렬 지수다. 구 8×16, 구 7×10, 상자 12×10(c = −0.5)의 전체 IVP에서 세 경로(Trusted, Verified, shipped)의 오차는 모두 6.6e-11~1.2e-10이다. 질량 drift는 4e-16이다. 같은 tolerance에서 shipped 경로의 drift도 1e-16~5e-15라서, 질량 보존 이점은 이 설정에서 관찰되지 않았다(**N**).
- **T4.** pivot 부호로 센 수가 셀 순서 dense 소거의 비양 pivot 수와 세 격자 다섯 (z, c)에서 일치했다. 거부 여부도 일치했다.
- **T5.** 고리별 감쇠 1 + 0.2 sin r를 평균 c 하나로 선언한 경우:
  - Verified는 풀이당 교정 2.22회로 기준값 오차 5.2e-11, 63회 시도.
  - 같은 불일치를 Trusted로 풀면(틀린 Jacobian의 W-method) 819회 시도에 오차 2.3e-9.
  - 먼 불일치(고리 하나 건너 감쇠 40, 교정 1회 허용)는 50,003회 풀이를 거부하고 단계 예산에서 멈췄다. 설계대로이다.

## 4. 성능 진단 (호스트 wall time, `results/SL_N5_TIMING.jsonl`)

전체 풀이, rtol 1e-6, κ = 1e-2. 비교 대상인 shipped matrix-free 경로는 재시작 GMRES이며 문제별 전처리기가 없다.

| 격자 | n | shipped | separable Trusted | separable Verified | 시도 수 (shipped/separable) |
| --- | --- | --- | --- | --- | --- |
| 32×64 | 2,048 | 0.86–0.88 s | 0.0066–0.0099 s (88–130×) | 0.0099–0.0145 s (60–86×) | 29 / 29 |
| 64×128 | 8,192 | 14.5–15.8 s | 0.043–0.045 s (340–355×) | 0.064–0.090 s (177–227×) | 56 / 42 |
| 128×256 | 32,768 | 3회 반복이 약 26분 안에 끝나지 않음 | 0.283 s | 0.415 s | – / 53 |

달성 오차(엄격한 separable 기준 대비 상대 오차)는 shipped 1.2e-9~7.3e-9, separable 5.2e-9~2.1e-8로 모두 rtol 1e-6보다 세 자리 이상 작다.

단계 하나(`(I − σL)x = r`, GMRES rtol 1e-10)를 SL-N5가 지정한 비교 대상인 GMRES + 경도선 전처리기와 비교했다. separable 쪽은 FFT plan과 factor 비용을 포함한다. 결과는 세 격자, h ∈ {1e-3, 1e-2, 1e-1} 모두에서 separable이 109–131배 빨랐다. 전처리기 없는 GMRES는 128×256, h = 0.1에서 3000 Arnoldi 벡터 안에 수렴하지 않았다.

해석: 이득의 원천은 Krylov 반복을 없앤 것과, 극점 강성(pole problem)이 모드별 대각으로 흡수된 것이다. 이 측정은 호스트 진단이다. 판정은 KS-N3의 owner-host 셀로 한다.

## 5. 남은 것과 부정적 결과

- 8단계 순차 루프의 병렬화는 여전히 **N**(V05 깊이 8). 대칭이 있는 문제에서는 단계 안 모드 병렬(⌊np/2⌋+1 폭)이 올바른 형태이다(KS-N4, **P**).
- 질량 보존 이점: 이 tolerance에서는 관찰되지 않았다(**N**). 느슨한 Krylov tolerance에서 다시 볼 가치가 있다.
- 비교 대상 정리:
  - 경도 의존 계수나 비선형 반응은 정확성이 깨지므로 Verified 결손 교정으로 처리한다.
  - 고리별 c_r(정확성 유지)과 separable 전처리 GMRES는 KS-N6이다.
  - 이류(advection)는 비정규라 inertia와 축소성 명제가 성립하지 않는다(**O**, KS-N8).
- `spectral_bounds`를 자동 라우터의 강성 인증으로 쓰는 일은 owner 결정 사항이다(KS-N5).
- 툴체인: 고정된 1.94.1은 이 호스트에서 설치할 수 없어 stable 1.97.0을 썼다.

## 6. 검증 명령

```
python3 -I research/kernel_spectral_foundation_20261124/scripts/verify_run.py
python3 -I research/kernel_spectral_foundation_20261124/scripts/verify_foundation.py
cargo test --locked -p vigilode --release --test spatial_separable
cargo test --locked -p rodas5p-integrators --release --lib banded_exact
cargo test --locked -p rodas5p-integrators --release --test banded_kernel_entry_identity
cargo test --locked -p vigilode --release --no-fail-fast
cargo clippy --locked -p vigilode --all-targets --release --no-deps -- -D warnings
```

## 7. 검증 결과 (2026-11-25)

- facade 전체(기본 feature): 578 통과 / 0 실패 / 3 무시, 결과 줄 70개.
- facade 전체(`--features structured`, 직전 run과 같은 조건): 583 통과 / 0 실패 / 3 무시. 직전 run의 577/0/1에 새 테스트 6개(통과)와 timing 진단 2개(무시)가 더해진 값이다.
- 그 밖의 검증: clippy(-D warnings), fmt, verify_foundation.py, verify_run.py, 직전 run의 verify.py가 모두 통과했다. 상세는 `results/VALIDATION.json`에 있다.
- 커밋: 3f9762a1(SL-N12), 67b71bd2(SL-N6), a0ec98ba(SL-N5, 이론, handoff). 원격 브랜치 SHA를 확인했다.
