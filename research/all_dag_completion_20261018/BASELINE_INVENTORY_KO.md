# Nonlinear homotopy 이론 연구의 저장소 기준

참조일: 2026-10-18 (Asia/Seoul). 이번 조사는 읽기 전용이다. 소스 수정, 브랜치 전환, push, 과학 테스트 실행은 하지 않았다. 아래 실행 판정은 저장된 원시 기록의 재사용이며 이번 세션의 재실행 결과가 아니다.

## 고정한 ref와 변경 범위

| 구분 | Commit | Tree |
|---|---|---|
| `research/cloud-completion-20261018` 최신 | `ac89a9ac5600e631120da480eee7f67bb16f9ea0` | `ad5b14a1159065a14dac8eaec1248717536cb3f0` |
| 실제 구현 publication source | `b2379658a4eb0d8b4ddddcd58f754ef28e615c12` | 최신 source manifest에 계속 귀속 |
| `audit/production-readiness-20261018` 최신 | `7e2873887ed6038b47b667c976435810851c5dba` | `9275e35d3d5afe1ae4e40e1c0d9aca1034994ca6` |

`b237965..ac89a9ac`에서 `crates/`, `Cargo.toml`, `Cargo.lock` 변경은 없다. 변경은 로컬 실행 증거, 보고서, ledger이다. 따라서 기존 source의 의미론을 재사용할 수 있다. 기존 무관한 untracked 임시 파일도 보존했다.

## 최신 로컬 실행 근거

`research/cloud_completion_20261018/evidence/local_codex_rerun_20261018/LOCAL_RERUN_20261018.json`은 2026-10-18 10:45 UTC 작성, 실제 실행 HEAD `412856893c3451917ea19c38ec2965f8790d3a1c`, source manifest 31개 동일, Cargo.lock 동일, check 7개·smoke 2개·bench 2개 명령 전부 exit 0을 기록한다. Rust/Cargo 1.94.1, Python 3.12.3, locked/offline 환경이다.

Allocator positive control도 통과했다. 3개 Vec clone allocation을 검출했고 warmed native step allocation 검사도 PASS다. 따라서 현재 로컬까지 allocator invalid라고 서술하면 틀린다. `DECISION.json`의 `allocation_measurement_valid_here:false`는 과거 cloud 환경 범위 기록이며 최신 local 기록을 대체하지 않는다. 전체 trajectory/일반 solver allocation으로 scope를 확대할 수는 없다.

최신 local 32-mode 비교 median structured/dense(ns)는 tolerance 순서로 `329854/527790`, `573487/902671`, `1329028/2115698`이며 모든 30행 empirical accuracy gate가 통과했다. 두 해상도 RK4 기준은 empirical이다. 이 결과는 여전히 `DIAGNOSTIC_ONLY`이며 production와 speed promotion은 HOLD다. 과거 ledger hash mismatch는 재실행되지 않은 inherited provenance issue로 남는다.

## 실제 모델과 API

Immutable `CollectiveModel`은

\[
\dot a_j=(-\kappa+i\,\mathrm{width}\,\xi_j)a_j+i\chi|m|^2m,\qquad m=\sum_jw_ja_j
\]

을 생성한다. 실수 배치는 `y[2j]=Re(a_j)`, `y[2j+1]=Im(a_j)`이다. `carrier`는 저장되지만 RHS에는 넣지 않는 회전좌표 메타데이터이다. Constructor는 weight sum이 정확히 1임을 인증하지 않고 `1e-12` 허용오차로 admission하므로, 새 증명에서는 실제 `S=sum w_j`를 유지하거나 별도의 exact-sum admission이 필요하다.

- `CollectiveModel::{rhs,jvp,to_ode_problem}`: nonlinear cubic RHS/JVP 생성, O(N).
- `CollectiveRodas5pWorkspace::{new,step_into}` 및 `integrate_collective_rodas5p_observed`: 실제 all-eight-stage O(N) native 구현. Exact stored weight sum 1 및 carrier=0 admission. Embedded estimate 기반이며 global verified window flow는 아니다.
- `CollectiveModel::to_bound_continuous_problem` → private `BoundCollectiveProblem::verify_continuous_residual_tube`: generated RHS와 continuous provider를 묶는 실제 API. 별도 callback substitution을 방지한다.
- `bound_collective.rs`가 구현한 Euclidean log norm은 `-kappa + 3|chi||mean|^2 sqrt(N)*sqrt(sum w^2)`이다. PT2 문서의 더 날카로운 weighted bound `-kappa+S^2|chi|E`는 이 provider에 구현된 값이 아니다.
- `AffinePrefixWorkspace::evaluate`, `parallel_error_prefix`: 고정된 caller-certified affine majorant를 outward tree composition하는 구현. 알려지지 않은 incoming tube에 의존하는 gain의 인증이나 nonlinear flow 생성 기능은 없다.
- `homotopy.rs`의 `run_fixed_homotopy_path`, `run_scheduled_homotopy_path`, `homotopy_step`: 하나의 RODAS step 내부 stage homotopy이다. PT2의 여러 시간 window shooting과 다른 알고리즘이다.
- `certify_nonlinear_target`/`NonlinearOutputCertificate`: 소스가 명시적으로 `is_error_bound() == false`로 분류한다. Exact-Jacobian Newton correction diagnostic이며 nonlinear root enclosure가 아니다.
- `Q2CertificateSource`는 `QuadraticStageProblem`에 결합한다. Cubic collective model을 받는 일반 certificate trait가 아니다.

## Missing client 구성요소

1. 고정 convex product tube와 실제 weighted-energy admission.
2. Fine/coarse `F_j,G_j` map의 immutable model/time/tube binding.
3. Fine/coarse numerical map error enclosure와 invariant-ball 보장.
4. Weighted contraction constant 및 representable-time outward handling.
5. Jacobi boundary iteration, continuation predictor, `lambda=1` endpoint residual stopping.
6. Curve/output certificate와 window endpoint radius의 verified transaction.
7. Reject/fallback/setup/certificate·dispatch 비용과 memory를 모두 기록하는 ledger.
8. Model-specific endpoint/branch validation. 실제 PT2 nonlinear client 실행은 아직 없다.

## 기존 HT01/HT02의 정확한 적용 범위

HT01은 `ABSTAIN`: 8개 screened candidate 모두 S1 실패, measured callback share 0.011–0.108로 해당 threshold 0.348 미만이다. HT02는 `ABSTAIN_NOT_RUN`, `runs:0`이다. 이는 homotopy의 일반적 불가능성 증거가 아니다.

`HOMOTOPY_EXPENSIVE_CLIENT_CONTRACT.md`의 `f_min=0.348`은 stage homotopy의 고정 가정 `r=5,P=8`에서 유도했다. 이를 새 window shooting의 필수 수학적 threshold로 자동 이식하면 안 된다. 실제 `HomotopyWorkLedger`에는 contract가 요구한 `max_task_per_round`, `barriers`, `rejected_candidate_work`, `fallback_work`, `peak_bytes`가 아직 없다.

## 별도 audit branch 최신 내용

Audit branch의 최신 5개 commit은 packet local certificate만 개선했다. `PacketFrozenOutput`, immutable `PacketCertifiedView`, unequal-frequency Fraction oracle, subnormal case, query/refusal cost instrumentation을 추가했다. 모두 undamped local packet scope이며 damped homotopy client를 제공하지 않는다.

최신 `weighted_radius()`는 정확 frozen polynomial 주위의 기본 반지름이며 returned floating center displacement를 제외한다고 명시한다. 이를 바로 returned-center weighted boundary ball로 전달해서는 안 된다.

## 근거 상태

- Current ref, file bytes, API 존재·한계: `implementation-verified` (read-only source inspection).
- 저장된 로컬 test/bench: `numerically checked`/`implementation-verified` 재사용, 이번 세션 재실행 아님.
- 기존 PT2: 조건부 `derived`이며 가정의 native client admission은 `unresolved`.
- 신규 nonlinear homotopy client: `NOT_IMPLEMENTED`.

정확한 파일 경로·SHA-256·git blob은 같은 폴더의 `SOURCE_IDENTITY.json`에 수록했다.
