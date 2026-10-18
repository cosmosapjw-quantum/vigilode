# Codex 인계 프롬프트

아래 작업을 실제로 실행하라. 대상은 **VigilODE**이며 BASS/REI 프로젝트의 실행 제한을 가져오지 않는다.

저장소: https://github.com/cosmosapjw-quantum/vigilode
전달 브랜치: `research/cloud-completion-20261018`
검토·실행 소스 commit: `b2379658a4eb0d8b4ddddcd58f754ef28e615c12`
통합 기준: `c513f478f952068b91e3da368fb2cb156d942b28`
산출물: `research/cloud_completion_20261018/`

## 목적과 현재 상태

클라우드에서 증명·native Rust 구현·표적 실행·독립 검토를 끝낸 변경을 로컬
환경에서 컴파일하고 실제로 실행하라. 이번 로컬 작업은 이미 닫힌 증명을 다시
연구하는 작업이 아니다. 현재 `physmath-research-loop` 내장 규약 1.0.0을 적용하되,
기존 근거의 source hash와 가정이 유지되면 재사용한다. 현 환경에서 이용 가능한
실행자만 기록하고 모델 라벨이나 미실행 테스트를 실행 근거로 쓰지 않는다.

먼저 `git status --short --branch`, remote refs와 최신 commit을 확인하라. 사용자의
수정·실행 중 작업은 보존하고, 필요하면 별도 worktree로 전달 브랜치를 연다.
현재 로컬 브랜치를 무조건 reset하거나 변경을 덮어쓰지 않는다. 전달 브랜치 이후
커밋이 있으면 아래 source manifest와 비교하여 변경된 의존성만 확인한다.

읽을 순서:

1. 이 파일, `REPORT_KO.md`, `DECISION.json`, `LOCAL_EXECUTION.json`.
2. `SOURCE_IDENTITY.json`과 `REVIEW.json`: 실제 source binding과 검토 범위.
3. 필요한 구현 문서만: `PACKET.md`, `LEJA.md`, `DENSE.md`, `STRUCTURED.md`,
   `PARALLEL_THEORY.md`. 과거 감사 전체를 다시 수행하지 않는다.

완료된 코드:

* `packet_window::integrate_verified_packet`: zero-carrier·무감쇠 collective 모델의
  요청 mesh별 검증된 다중 window 해. 정확히 표현되는 시간차와 원래 absolute
  budgets를 사용하며, 실제 반환 중심의 평가 반올림을 포함한 weighted ball을
  전달한다. 초기 disks→ball은 한 번만 수행한다.
* `leja_action::leja_phi_action_enclosed_with_workspace`: 저장된 dense symmetric
  nonpositive 행렬에 대한 fused 출력의 Euclidean 전체 오차. 기존 legacy API는
  EstimateOnly다. 새 API의 `Certified`는 상계의 유효성이고 `converged`는 별도의
  요청 예산 충족이다. 개별 columns의 인증으로 확대하지 않는다.
* `Pr101Model`: 실제 RODAS5P candidate에 대해 연속 defect, signed log norm,
  동일 curve 출력 반경을 계산하고 검증 후 commit하는 driver. 원래
  lambda=-1e6, eta=10, [0,1], 101 outputs, atol=1e-11, rtol=1e-8 witness를
  통과했다. pure-relative와 arbitrary OdeProblem을 지원한다고 가정하지 않는다.
* `collective_rodas5p::CollectiveRodas5pWorkspace` 및 observed driver: native
  K-coordinate의 8개 stage 모두 O(N) Woodbury를 사용하고 같은 factorization과
  preallocated buffers를 재사용한다. source-level O(N) 정리와 실제 wall time은
  별개의 근거다.
* `affine_transport::AffinePrefixWorkspace`: 이미 검증된 비음수 local affine error
  maps의 outward prefix. 준비 후 O(N) work/O(log N) fork-join span이다.
  caller가 local ODE bound를 제공해야 하며 allocating wrapper의 O(N) setup은
  비용에서 제외하지 않는다.

## 실행

Rust/Cargo **1.94.1**, Python3 stdlib, 잠긴 dependencies를 사용한다. 프로젝트에
제공된 offline vendor와 로컬 설치 경로를 사용하고 Cargo.lock/Cargo.toml을 변경하지
않는다. 이 저장소는 우리 클라우드의 절대 runtime 경로에 의존하지 않는다.
해당 환경에서 proc-macro의 `E0463`가 나오면 `ldd`로 실제 원인을 확인하라.
클라우드 번들에서는 libstd 동적 라이브러리 경로가 필요했다. 그때는 로컬
`rustc --print sysroot`의 `lib`와 `lib/rustlib/<host>/lib`를 LD_LIBRARY_PATH에
추가하면 된다. 소스 수학이나 tolerance를 바꾸는 해결책을 사용하지 않는다.

저장소 root에서:

```bash
python3 research/cloud_completion_20261018/run_local.py --mode check --output artifacts/generated/cloud_local_check
python3 research/cloud_completion_20261018/run_local.py --mode smoke --output artifacts/generated/cloud_local_smoke
python3 research/cloud_completion_20261018/run_local.py --mode bench --profile measurement --modes 32 --repeats 5 --output artifacts/generated/cloud_local_bench
```

출력 폴더는 새 이름이어야 한다. 재실행 시 이전 로그를 덮어쓰지 않는다. Cargo
캐시가 없고 다운로드가 필요한 환경에서는 명시적으로 `--online`을 추가한다.
먼저 check/smoke를 끝내고 bench를 실행한다. Measurement profile은 모든 arm에
동일하게 적용한다. 필요하면 같은 명령으로 modes=128,256을 추가하되 cold/warm,
출력 개수와 achieved accuracy가 다른 행을 한 speedup으로 합치지 않는다.

Allocation은 **양성 대조군을 먼저** 확인한다. 클라우드의 제공 Rust 번들은
실제로 세 Vec를 복제하는 library 호출을 custom GlobalAlloc이 0으로 세었다.
별도 `collective_rodas5p_allocation_contract`가 이 오류를 검출한다. 이 환경에서는
zero-allocation을 인정하지 않고 runner의 allocation 값은 null이다. 로컬에서도
같은 calibration 실패면 numerical 검사 결과와 분리해 기록하고, 정상적인 static
toolchain 또는 실제 allocator profiler에서만 계측한다. 그 실패를 없애려고
검사를 삭제하거나 “0 allocation”으로 바꾸지 않는다. runner의 전체 exit code와
`nonallocation_commands_success`를 함께 확인한다.

## 수용 기준과 수정 범위

원래 모델, 입력, atol/rtol, exact-duration 계약을 유지한다. 빌드·경로·직렬화·호환성의
작은 수정은 직접 고치고 해당 변경 범위만 다시 검증하라. 수치 실패가 생기면 첫
입력/로그/exit code를 보존하고 원인을 구현·환경·수치/증명 중에서 분류한다.
더 큰 수학 변경이 필요하면 기존 통과 범위를 유지하고 그 반례와 정확한 식을
별도 보고하라. 이미 닫힌 전체 연구를 처음부터 반복하지 않는다.

공통 측정 runner는 동일 모델·동일 output grid·동일 tolerance를 사용하고,
setup을 포함한 시간, 두 해상도 RK4 참조, normalized error, 실패 arm과 allocator
calibration을 모두 남긴다. 두 해상도 참조는 empirical이며 certified reference로
승격하지 않는다. 순수 wall-time 차이를 전역 production 판정으로 바꾸지 않는다.

이번 로컬 인계의 완료 기준은 다음과 같다.

1. 실제 checkout identity 및 변경한 파일을 기록하고 check/smoke 결과를 확보.
2. 가능한 범위의 optimized 비교를 실행하여 원시 JSONL·stdout/stderr를 보존.
3. accuracy gate와 allocation calibration을 시간비교와 함께 보고.
4. 작은 소스 수정이 있으면 source hashes와 해당 검증을 갱신하고, 별도 요청이나
   기존 push 승인 범위에 따라 전달 브랜치에 fast-forward commit/push.
5. `environment.json`, `executions.json`, `RESULT.json`, 모든 stdout/stderr와 JSON
   결과를 반환. 실행하지 않은 항목은 not_run으로 남긴다.

일반 production 전체는 아직 HOLD다. damped packet, 일반 사용자 OdeProblem의
continuous provider, sparse/callback Leja, 실제 nonlinear homotopy client의 완성은
이 로컬 validation 인계에 숨겨진 구현 과제가 아니다. 현재 opt-in 경로의 증명과
실제 동작을 확인한 뒤, 이 확장은 별도 범위로 진행한다. PT2에는 common-damping
contraction 조건하의 homotopy parallel shooting 수렴·span 정리와 실행 계약까지
이미 작성되어 있다.
Common-damping 부등식은 fine flow의 contraction 조건이며, coarse map의
공통 invariant-ball 보존과 contraction은 별도 전제다.

추가로 전체 repository의 `check-research-node.py`는 기존 L-0124가 가리키는
보고서 두 개의 hash 불일치로 실패한다. 두 파일은 전달 기준 commit에서도 이미
같은 상태였고 이번 작업에서 변경하지 않았다. `evidence/publication_checks.json`을
참조하고, 이를 신규 solver 수치 실패로 분류하거나 과거 ledger를 고쳐 숨기지 않는다.
이 이력 정리는 현재 제공 실행 경로의 추가 수학 과제가 아니다.
