# VigilODE 생산 준비 감사 continuation 보고서

## 실행 정체성과 범위

- 작업 브랜치: `audit/production-readiness-20261018`
- 시작 audit commit: `6c067105f0b9e16cbfd397b2d5316673e698e00e`
- continuation 기준 HEAD: `c3036c63da02ca9b04cd39ccf580cd67d00ccb0c`
- 코드 실행 commit (EXEC): `01c5b93357ec4318e06e36d938deedf1b2db4a3f`
- 실행 commit은 origin 브랜치에 push하고 fetch로 동일 SHA를 확인했다.
- Rust/Cargo: 1.94.1, x86_64-unknown-linux-gnu, `--locked --offline`; Cargo.toml/Cargo.lock 수정 없음.
- 전체 `cargo test --workspace` 실행 기록은 없다. 아래는 등록된 표적 테스트와 workspace check/fmt/clippy 결과다.

## DAG 실행 및 과학적 판정

| 노드 | continuation 범위 | 판정 |
|---|---|---|
| A01 | 반환 중심을 포함하는 local outward 경로, one-mode와 고정 mpmath multi-mode 참조, 중간/끝점, uncertainty/refusal/work 검증 | **SCOPED PASS WITH HOLD**. 전체 비선형 계수·force·residual·tail 및 descriptor-bound certificate는 미완료. `roundoff_enclosed=false`. |
| A02 | 선형 전용 trajectory 경로와 고정밀 fixture, 관측시각/끝점 계약 | **SCOPED PASS WITH HOLD**. 비선형 실제 재구성 곡선 continuous residual, true/candidate tube 및 전체 T1 gain 증명 미완료. |
| A03 | `Q_{r,d}(a,h)=I_{d,r}(a,h)` scalar damped forcing-moment 소비자 | **SCOPED PASS WITH HOLD**. `exp(-2κt)` 비선형 force, carrier/output convention, κ→0 multiwindow 재구성 미완료. |
| A04 | Leja outward interval replay와 실패 시 report transaction/work 보존 | **SCOPED PASS WITH HOLD**. 전체 additive-underflow 산술 상계가 없어 `EstimateOnly` 유지. |
| A05 | 표현된 quartic dense curve의 value/derivative 구간과 정확한 산술 작업 카운터 | **SCOPED PASS WITH HOLD**. 연속 ODE residual 및 verified full-tube one-sided gain 없음. |
| A06 | 기존 carrier-zero rotating-frame rank-two Woodbury 경로 | **SCOPED PASS WITH HOLD**. 전체 trajectory 비용·속도는 미측정. |
| A07 | 기존 보고서가 무결성 검토된 Robertson 비교 재사용 | campaign integrity만 scoped pass. Diffsol 정확도 gate 실패로 matched-accuracy/speed **HOLD**. |
| A08/A09 | A04 total arithmetic 및 A07 accuracy prerequisite 의존 | **BLOCKED**. |
| A10 | outward Bernstein 변환/subdivision helper의 입력·overflow refusal | helper만 scoped pass. A01 outward candidate 경로에 통합되지 않아 전체 노드 **HOLD**. |
| A11 | 제공된 Gray–Scott 상자에 대한 semidiscrete `mu_2` 상계 | 기존 scoped pass 유지; trajectory 상자/global defect는 미포함. |
| A12 | scope별 승격 판정 | 전체 생산 승격 **HOLD**. |

채택된 조건부 에너지 gain은 `m=Σw_j a_j`, `S=Σw_j`, `||a||_w²=Σw_j|a_j|²`, `μ_w(Df(a))=-κ+S|χ||m|²`로 정의하고 검증된 초기 `E`에 대해 두 흐름의 gain을 `exp[-κh+S²|χ|E θκ(h)]`로 제한한다. `θκ(h)=(1-exp(-2κh))/(2κ)`, `θ₀(h)=h`이다. 이 식은 실제 출력 곡선의 연속 residual 또는 재구성 mean 상계를 제공하지 않으므로 A02/A05의 빈 의무를 대신하지 않는다.

## A07 Robertson 결과

고정 입력은 Robertson `[0,40]`, `rtol=1e-8`, `atol=[1e-11,1e-11,1e-11]`, 101 output points다. 기존 유효 campaign을 반복하지 않았다. 20 cold rows/10 paired units, 160 warm rows, allocator 2 rows가 유지되고 실행 row는 모두 완료됐다.

- Native 최대 grid WRMS: `0.33753007149457903` (고정 gate 0.05 이내)
- Diffsol 최대 grid WRMS: `3.3026992500504733` (고정 gate 실패)
- Cold Diffsol/native median ratio: `0.9195676453507838`
- Warm Diffsol/native median ratio: `1.4194023547437313`
- 시간 authority: `diagnostic_only`; 정확도 gate 실패로 speedup 주장을 하지 않는다.

따라서 이 결과는 공정한 동일 정확도 속도 우위를 보이지 않는다. 기존 별도 generic RODAS5P의 문제별 검증 사용 범위는 그대로다.

## 최종 검증

- Core 표적: 28 passed, 0 failed.
- Integrator 표적: 76 passed, 0 failed.
- Fair-ab dense error adapter: 1 passed, 0 failed.
- `cargo check --locked --offline --workspace --all-targets`: exit 0.
- `cargo fmt --all -- --check`: exit 0.
- `cargo clippy --locked --offline --workspace --all-targets -- -D warnings`: exit 0.
- 각 원시 로그와 별도 exit-code 파일이 이 노드에 보존됐다.
- `git diff --check`는 source EXEC 전에 통과했다. manifest/lock 변경은 없다.

## 이전 finding 상태

- **FIXED / 제한된 수정 유지:** PK-01, PK-03, PK-04, PK-06, SA-03, SA-04, SA-05, PA-01, DP-02, DP-03.
- **부분 수정이나 전체 의무 OPEN:** PK-02, PK-05(좁은 outward domain만), PK-07, PK-08, PK-09, PK-10, SA-01, SA-02, SA-06, PA-02, PA-04, PA-05, PA-06, PA-07, DP-01, DP-04.
- **SUPERSEDED_SCOPE:** PA-03의 과거 LP10 광범위 승격 문구는 독립 증거로 인정되지 않아 후속 판정으로 대체됐다.
- A07 캠페인 보고/identity 불일치는 후속 수정으로 정리됐지만, solver의 accuracy gate 실패와 empirical reference 한계는 열린 과학 조건이다.

## 게시 및 남은 실행점

코드 실행 commit은 원격 ref에서 확인했다. 결과/evidence commit 게시 여부는 이 보고서 작성 시점에 남은 ledger와 pre-push gate에 달려 있으며 과학적 승격과 별도다. 다음 작업은 A01 비선형 outward 산술 완결, A02 실제 재구성 곡선 residual/tube, A03 전체 damped packet/frame 연결, A04 모든 Leja 반올림 항 상계, A05 interval RHS residual과 full-tube gain이다. A04와 A07 조건이 충족되기 전 A08/A09는 실행하지 않는다.
