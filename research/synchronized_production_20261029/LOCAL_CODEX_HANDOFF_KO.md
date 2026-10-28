# Local Codex handoff — VigilODE proof-first synchronized production lane

아래 작업을 기존 연구의 연속으로 수행해줘. 이론 연구 R1–R10을 먼저 수행하고, 채택된 식·가정·오차 소유권을 넘겨받아 코딩 C1–C10을 진행해줘. 서로 독립적인 병목은 subagent로 나누되 실제 native benchmark는 소스·바이너리를 동결하고 순차 실행해줘. 완료된 이론은 재시작하지 말고 변경된 의존성부터 이어가줘.

## 1. 목표와 source pins

Repository: `cosmosapjw-quantum/vigilode`.
전달 branch: `research/synchronized-production-20261029`.
연구 node: `research/synchronized_production_20261029`.

| Identity | Commit |
|---|---|
| 조사 baseline, PR101 | `bc5ddbe42b24b1a6364bf653c3e2d6d938ca4087` |
| baseline 기존 allocation probe 실행 소스 | `c41caffa68d9f4eb9d1976422f7f74ec7e764cca` |
| baseline ownership probe 실행 소스 | `d37e3b2d2c418ca425a0ceed5e930caffd5bb771` |
| candidate 최초 native 소스 | `7fce5a4cbf5e1de3dd25de3b247f638bc551be24` |
| candidate 최종 native 실행 소스 | `ef84aa3d6c028b0922bb5c1e66202b95fd88a2fc` |
| 해당 native tree | `2989f07fb04134aa7bab38e38d7272e45d6a0917` |
| 조사 당시 main | `8d0c79184e09efb5bdadc24a6315c60a71a44264` |
| PR100 affine physical-symbolic 입력 | `8615b365c2812a126574f579328b2056ce5b03c1` |
| PR99 chemical remainder 입력 | `e46704cbc5a1356bd8fd199de09ad26b046acfd4` |
| 뒤늦게 확인한 PR102 native affine provider delivery | `8357faa5b42724a8be24609d4d2f16aec8a6eb3a` |
| PR102 실제 code 실행 소스 | `0d722b4ed71413926c76bc0037f02f3a6bbdd362` |

실행을 시작할 때 전달 branch의 실제 원격 head를 fetch하여 `DELIVERY_SHA`로 고정해줘. 이 값은 위 `NATIVE_SHA=ef84…`와 다를 수 있다. 전달 보고서·metadata 추가와 native code 변경을 구별하고, `git diff NATIVE_SHA DELIVERY_SHA -- crates Cargo.lock Cargo.toml`을 확인해줘. 변경이 있으면 해당 의존성의 qualification부터 다시 수행해줘. source 확인 없이 전달 commit을 native 실행 commit으로 바꾸어 쓰지 마.

첫 목표는 뒤늦게 확인한 PR102의 기존 native scalar-affine accepted-segment provider를 새로운 local integration tree에서 재사용·검증하는 것이다. 이 provider를 새로 중복 구현하지 마. 그 다음 K3의 RHS별 quotient 보정을 native에서 안전하게 채택할 정확한 계약을 완성하고, 실제 physical trajectory certificate 및 adapter 비용을 더 상세히 검증해줘. **Production 판정은 HOLD**다. 현재 구성은 기존 selective-JVP `ProductionConfig`와 단일 dispatcher를 실제 호출한 하나의 lane이며, 실제 BASS/REI client나 전체 허용 영역의 production qualification을 완료한 상태는 아니다.

## 2. 먼저 읽을 자료와 보존 규칙

이 node의 `REPORT_KO.md`, `RESULTS.json`, `CURRENT_STATE.json`, `CORRECTIONS.json`, `EXECUTION_PLAN.json`, `PREREGISTRATION.md`, `THEORY_ADMISSION.md`, `theory/{KRYLOV,ADAPTER,DEFECT,SYNTHESIS}.md`, 각 workflow, `reviews/`와 evidence manifest를 읽어줘. 이 node의 `evidence/RAW_EVIDENCE.tar.gz`를 `evidence/RAW_MANIFEST.json`의 identity에 따라 복원하고 member hashes를 검증해줘. 조사 근거는 `audit/CURRENT_AUDIT.md`, `audit/CURRENT_STATE.json`, `audit/evidence/CI_FAILURE_EXCERPTS.json`에 있다. PR102의 late compatibility review도 최종 보고서가 가리키는 문서에서 읽어줘. PR102 원본 report/admission/final-source identity와 이번 read-only review를 구별해줘.

다음 원자료를 각각 구별해줘:

- `lane-checks01`: 소스7fce…의 최초 실패 포함.
- `lane-checks02`: 소스ef84…의 fresh-target 검사.
- `lane-charge03`: 같은 ef84…에서 process-global allocator 검사를 직렬화한 독립 재검사.
- `lane-bench02`, `lane-deployment02`: ef84…의 비교·설정 호출.
- `lane-analysis-reviewed03`: raw 해시와 모든 paired block을 다시 읽은 최종 집계.
- `runtime/build_candidate*`, baseline/candidate bindings, shared-target incident: compile provenance.

이 이름들은 cloud receipt의 원래 basename이다. `/workspace/scratch/...`나 `/home/cosmosapjw/...`를 현재 로컬에 존재하는 경로로 가정하지 말고 복원 manifest에서 새 위치를 결정해줘. PR101의 큰 외부 raw가 없으면 compact packet의 재집계 범위와 실행 재현을 구분해줘.

기존 23-node/34-edge DAG, 73 paths, 8 suites, 10 interaction gates, 모든 negative evidence와 source identity를 보존해줘. ancestry·scoped pass·현재 누적 검증·원본-source replay는 각각 다른 상태다. 10개 interaction 전체가 이번에 닫혔다고 표시하지 마.

이번 cloud campaign의 global correction budget은 **3/3 소진**이다. episode3는 같은 소스의 allocator 직렬 실행 및 실패·누적비용을 보존하는 집계 교정이다. 최초 실패를 지우거나 task 이름을 바꾸어 횟수를 초기화하지 마. 이전 R-STRUCT3/3, R-ADAPT3/3, PR-N053/3, Q-LEDGER3/3도 소진 상태다. PR-N01·PR-N04·Q-LINT는 이번 변경 후 각각2/3, Q-BENCH는1/3이며, 나머지는 최종 ledger를 그대로 계승해줘. 이 prompt로 승인된 로컬 검증·후속 연구를 별도 execution id로 사전등록하되, 기존 과제의 소진된 수정 예산을 새 이름으로 우회하지 마. 새 수치 실험·수정에는 실제 소유 node와 남은 예산을 먼저 연결해줘.

## 3. 이미 확보한 결과

수학적 결과는 명시된 가정 아래 완결되어 있다. native provider 미완성을 이론 자체의 미증명으로 바꾸지 마.

- **K1:** GMRES-into가 읽지 않는 `directions` pool의 준비를 생략해도 live numerical fields, callback word, MGS/QR 산술과 성공 출력은 같다. public reserve 및 legacy/augmented prepare의 전체 용량 계약은 유지한다.
- **K2:** `AV=VH+F`, `r0=βv0+g0`에서 `r=V(βe1−Hy)+g0−Fy`. one-pass MGS로 바꾸려면 Gram/relation/rounding error를 새로 관리해야 한다.
- **K3:** RHS별 `p(J)v=e`의 quotient 후보와 원래 선형계 residual/solution error를 직접 연결했다. full-space annihilator를 요구하지 않는 조건부 후보이며 native default에는 연결하지 않았다.
- **K4:** 고정 J의 shifted image `C1=ρC0+(1−ρ)U`와 결함 전파를 유도했다. J/preconditioner/metric/domain epoch와 QR·rank·inverse conditioning 의무는 유지한다.
- **A1:** 소비되는 facade output을 clone 대신 move해도 공공 값·bits·Stats·event 및 선언된 callback 효과가 같다. allocator address를 계산 입력으로 쓰는 callback과 allocation failure는 이 성공 실행 정리의 범위 밖이다.
- **A3:** strict comparator는 동일 window의 후보 비용과 comparator 비용을 비교한다. discarded speculation까지 더한 전체 정책 비용이 별도의 RHS-only trajectory보다 싸다는 정리는 아니다.
- **A4:** 실제 reentrant model의 clock–fiber 분해와 clock defect를 포함하는 비교식을 유도했다.
- **D:** `E'≤−αE+ΓE²+δ`에 대해 주어진 반경R의 불변조건, endpoint/whole-slab enclosure, norm/reset/source/clock transfer 및 event 조건을 구성했다.
- **S:** lower triangular stage perturbation → reconstruction original defect → tube transport를 연결했다. 실제 original residual을 직접 인증한 경우 stage error를 다시 더하지 않는다.

Native 구현은 `rodas5p-krylov/src/{workspace,gmres_into}.rs`의 private basis-only preparation과 `vigilode/src/solve.rs`의 consumed-output 이동이다. `time_fd_workspace_contracts.rs`의 Copy lint도 수정했다. 원래 허용오차·method/router·callback permission·수치 산술은 유지했다. `directional_quotient.py`, `defect_composition.py`, `stage_transfer.py`는 exact research consumer이며 native certificate provider를 발급하지 않는다. 별도 PR102에는 이미 native scalar-affine provider가 있다. PR102의 source와 이 lane의 source는 아직 통합 실행되지 않았다.

실제 ef84… 결과:

- exact Python 25 methods PASS.
- library 99 PASS, ignored1 유지.
- facade integration 280 PASS, time-FD2 PASS.
- Krylov integration 최초115 PASS/1 FAIL. `sp02_workspace_charge`의 process-global allocator가 병렬 test function과 간섭했다. 같은 소스 `--test-threads=1`에서 해당3 methods PASS. 최초 failure는 보존하며 통과 횟수에 중복 가산하지 않는다.
- facade/Krylov focused all-target Clippy와 time-FD focused Clippy PASS. 전체 workspace/모든 feature qualification을 뜻하지 않는다.
- ownership11 cases×3=33 pairs에서 전체 반환 observation parity PASS. 4개 FD/JVP case의 probe/allocation arms에서도 반환 bits·Stats·callback parity PASS.
- endpoint 최대 diagnostic error4.4617096683907675e-9, 기존 cap1e-5 이하. binary64 analytic reference이며 reference uncertainty나 모든 dense interior를 엄밀하게 감싼 결과는 아니다.
- 최초 실패와 serial control을 포함한 campaign wall491.539190153초/900초. 사전 compile 비용과 후처리는 분리 기록되어 있다.

| Case | 첫 solve allocations baseline→candidate | 후속 두 solve 각각 | paired time baseline/candidate median [min,max] |
|---|---:|---:|---:|
| dense-fd | 324→315 | 68→59 | 0.96549 [0.93893,1.03682] |
| banded-fd | 318→314 | 62→58 | 0.96415 [0.89009,0.98013] |
| time-jvp | 386→358 | 129→101 | 1.01486 [0.98224,1.11859] |
| autonomous-jvp | 389→361 | 133→105 | 1.01049 [0.97792,1.03521] |

7개 ABBA/BAAB block이며 child마다16회 solve+drop의 중앙값을 만든 뒤 block의 두 arm 값 합의 비를 계산했다. 위 min/max는 신뢰구간이 아니다. Dense와 banded의 시간 회귀를 그대로 유지해줘. Allocation 감소가 wall-time 개선을 보장하지 않는다. 이번 호스트에는 perf/valgrind/Callgrind/heaptrack이 없어서 새 function profile은 미실행이다. 새 profiling 근거는 allocation·WorkCounters이며, PR101의 function Ir는 이전 source/workload에서 얻은 근거로만 재사용했다.

## 4. 로컬 source/build 격리와 최소 검증

기존 checkout의 dirty WIP와 기존 실행을 보존하고 새 worktree를 사용해줘. 최신 main 및 PR head/status를 다시 읽고, 새 main이 선행 branch를 포함하면 graph를 확인한 뒤 필요한 integration만 수행해줘. 원격 source publication과 fetch/readback은 새 native scientific run보다 앞서야 한다.

예시 구조는 다음과 같다. 아래 task-specific 변수의 값은 현재 로컬에서 정하고 cloud 경로를 복사하지 마.

```bash
REPO_ROOT=$(git rev-parse --show-toplevel)
VIGILODE_RUN_ID=$(date -u +%Y%m%dT%H%M%SZ)
VIGILODE_LOCAL_ROOT="$(dirname "$REPO_ROOT")/vigilode-local-sync-$VIGILODE_RUN_ID"
LOCAL_VALIDATION_BRANCH="validation/local-sync-$VIGILODE_RUN_ID"
mkdir "$VIGILODE_LOCAL_ROOT"
DELIVERY_REF=origin/research/synchronized-production-20261029
NATIVE_SHA=ef84aa3d6c028b0922bb5c1e66202b95fd88a2fc
BASELINE_RUST_SHA=c41caffa68d9f4eb9d1976422f7f74ec7e764cca
BASELINE_OWNERSHIP_SHA=d37e3b2d2c418ca425a0ceed5e930caffd5bb771

git fetch origin
DELIVERY_SHA=$(git rev-parse "$DELIVERY_REF")
# 이 실제 값과 전달 보고서를 대조하고 실행 기록에 고정한다.
git diff "$NATIVE_SHA" "$DELIVERY_SHA" -- crates Cargo.lock Cargo.toml
git worktree add --detach "$VIGILODE_LOCAL_ROOT/baseline-rust" "$BASELINE_RUST_SHA"
git worktree add --detach "$VIGILODE_LOCAL_ROOT/baseline-ownership" "$BASELINE_OWNERSHIP_SHA"
git worktree add -b "$LOCAL_VALIDATION_BRANCH" "$VIGILODE_LOCAL_ROOT/candidate" "$DELIVERY_SHA"
```

`DELIVERY_SHA`의 실제 값과 전달 보고서를 확인해줘. local worktree/branch가 이미 있으면 덮어쓰거나 reset하지 말고 새 이름을 사용해줘. 수정한 source/protocol은 non-force push하고 origin의 정확한 새 SHA를 fetch한 뒤 실행해줘. main merge는 하지 마.

Rust1.94.1/compiler·CPU·affinity·libc·thread 설정을 실제 probe로 기록해줘. `Cargo.lock` SHA256은 `efb379071fa42c2b6bdbe969163ac2f41cf318c262c5c67d0910450298333351`이었다. graph를 편의상 갱신하지 말고 공급된 offline vendor의 package checksum 및 file hash를 검증해줘. 도구가 현재 환경에 있으면 사용하고, 과거 tool availability를 현재 실패의 근거로 쓰지 마.

**각 worktree마다 서로 다른 CARGO_TARGET_DIR를 사용해줘.** 이번에 shared target의 timestamp freshness가 baseline library artifact를 candidate에 재사용하여 서로 다른 native source의 probe binary hash가 같아졌다. 이를 실행 전에 발견했고, fresh target 재컴파일로 교정했다. `cargo build` exit0만으로 source provenance를 판단하지 마.

```bash
export CARGO_BUILD_JOBS=2
export RAYON_NUM_THREADS=1
export OMP_NUM_THREADS=1
export OPENBLAS_NUM_THREADS=1
export PYTHONDONTWRITEBYTECODE=1
# 선택한 현재 Rust1.94.1의 cargo/rustc가 PATH에 있는 상태에서 실행한다.
# baseline-rust worktree
CARGO_TARGET_DIR="$VIGILODE_LOCAL_ROOT/target-baseline-rust" cargo build --profile measurement --locked --offline -p vigilode --example rust_allocation_probe
# baseline-ownership worktree
CARGO_TARGET_DIR="$VIGILODE_LOCAL_ROOT/target-baseline-ownership" cargo build --profile measurement --locked --offline -p vigilode --example ownership_projection_probe
# candidate worktree
CARGO_TARGET_DIR="$VIGILODE_LOCAL_ROOT/target-candidate" cargo build --profile measurement --locked --offline -p vigilode --example rust_allocation_probe --example ownership_projection_probe --example production_from_json
```

각 명령은 표시된 worktree에서 실행하고 stdout/stderr/exit/timeout/wall/CPU와 실행 전후 native file hashes를 저장해줘. 복사한 executable과 build receipt를 source/tree/compiler/lock/probe-source digest에 연결해줘. `BINDINGS.json`의 schema는 **binary name→binding의 direct mapping**이며, outer `binaries` wrapper를 넣지 마. 최소 필드 `source`, `binary_sha256` 외에도 실제 build receipt, source snapshot 및 compiler identity를 남겨줘. self-declared SHA만으로 build provenance를 대신하지 마.

변경 없는 기존 검증은 identity로 재사용하고, 아래 검사는 현재 로컬 portability와 새 변경의 영향에 맞춰 실행해줘. process-global allocator 검사는 직렬로 실행한다.

```bash
python3 -m unittest discover -s research/synchronized_production_20261029/code -p 'test_*.py' -v
cargo test --profile measurement --locked --offline -p vigilode -p rodas5p-krylov -p vigilode-coupled --lib
cargo test --profile measurement --locked --offline -p vigilode --test '*' --no-fail-fast
cargo test --profile measurement --locked --offline -p rodas5p-krylov --test '*' --no-fail-fast -- --test-threads=1
cargo test --profile measurement --locked --offline -p rodas5p-integrators --test time_fd_workspace_contracts
cargo clippy --profile measurement --locked --offline -p vigilode -p rodas5p-krylov --all-targets --no-deps -- -D warnings
```

`run_lane.py --phase all`은 failed checks 뒤 benchmark를 실행하지 않도록 설계되어 있다. 별도 phase를 실행할 때는 앞선 실패를 해석한 수용 결정을 남기고 모든 이전 `RUN.json`을 `--prior-run`으로 전달해 같은 누적 wall cap에 넣어줘. 현재 runner의 default-parallel allocator test 문제를 수정 없이 재현할 때는 serial control을 별도 기록하고 최초 failure를 유지해줘. 새 로컬 lane에서 직렬 실행으로 조정하면 그 orchestration diff와 이유를 사전등록해줘.

```bash
python3 research/synchronized_production_20261029/code/run_lane.py \
  --source-sha "$LOCAL_SOURCE_SHA" --remote-ref "$LOCAL_REMOTE_REF" \
  --baseline-dir "$LOCAL_BASELINE_BIN_DIR" --candidate-dir "$LOCAL_CANDIDATE_BIN_DIR" \
  --output "$LOCAL_NEW_LANE_DIR" --phase all --wall-cap 900
```

HEAD와 fetched remote ref는 실제 `LOCAL_SOURCE_SHA`와 같아야 한다. candidate binding의 source도 그 값과 같게 실제 build를 수행해야 한다. 문자열만 고쳐 맞추지 마. 새로운 local campaign의 runtime budget과 compile budget은 별도 사전등록하고, cloud900초 잔여분을 새 승인·예산으로 가정하지 마.

## 5. 다음 연구를 병렬로 나누는 기준

### 0. PR102 native affine provider 재사용·통합 — 우선순위0

PR102 delivery `8357faa5b42724a8be24609d4d2f16aec8a6eb3a`, 실제 code source `0d722b4ed71413926c76bc0037f02f3a6bbdd362`를 fetch해줘. 이번 synchronized source ef84…와 common PR101 baseline을 공유하고 변경 crate 경로는 서로 겹치지 않는다는 read-only 검토를 마쳤다. 이는 통합 tree의 실행 PASS를 의미하지 않는다. 별도 isolated local branch에 merge 또는 필요한 commit을 cherry-pick하고, 실제 최종 통합 tree를 source-bound 방식으로 publish/build/test해줘. 두 원본 실행 identity와 수정 budget을 유지하고 두 번째 affine provider를 만들지 마.

재사용 대상은 `crates/rodas5p-integrators/src/affine_event_certificate.rs`와 `rodas5p_fast_dense.rs`의 actual accepted segment adapter다. 실제 stored stage/H/endpoint/time bits에서 quartic을 구성하고 `p_theta'/h + lambda p - b`의 whole-slab Bernstein enclosure를 만든다. cubic으로 단순화하면 마지막 H row의 실제 항을 잃는다. `h`는 represented endpoint 두 개의 exact-real 차를 outward하게 감싸며, 선언된 lambda/forcing box uncertainty는 이 original residual에 이미 포함된다.

이 정리는 scalar absolute norm에서 전역 affine contraction을 쓰므로 D 이론의 `Gamma=0`, carrier mismatch0 특수 경우다. affine field에 불필요한 유한반경 bootstrap을 추가하지 말고 기존 endpoint `max(U1,U2)`와 whole-slab `max(E0,endpoint)` 계약을 우선 재사용해줘. 새 exact consumer의 더 타이트한 exponential enclosure로 바꾸는 일은 별도 조건·비용 검증이 필요한 확장이다.

**Query rounding과 carry를 분리해줘.** 실제 returned binary64 center q와 같은 represented time의 exact polynomial enclosure I에서 `rho=max(abs(q-I.lo),abs(q-I.hi))`를 outward 계산하면 관측 오차는 `rho+E`다. 이 rho는 해당 dense query의 pointwise representation displacement다. query마다 physical jump로 누적하지 마. 다음 slab에 넘기는 carry는 polynomial endpoint error이며, rho는 그 반환 관측에만 더한다. 표본42 query로 모든 가능한 query의 uniform rounding bound를 주장하지 마.

NativeAccepted provenance는 실제 native segment에서 왔음을 증명한다. caller가 적은 source/model/clock/unit/event ID만으로 임의의 RHS callback이 선언된 affine equation과 같다는 사실이나 incoming global error의 정당성까지 인증하지 않는다. 실제 physical equation/source/rates/units/clock/IC를 검증한 owner capsule을 추가로 공급해줘. source·norm reset이 있으면 별도 transfer proof가 필요하다.

PR102의 원본 실행 근거는 code0d722…에서13 new affine +8 fast-dense=21 tests PASS, verifier381 checks(367 exact-rational comparisons+14 Decimal90 diagnostics)다. 6 records 중2개와42 queries 중14개만 actual native이며 native IVP는 `y'=-y`, span[0,0.125]다. 이번 cloud lane에서 이 실행을 재수행한 것이 아니므로 합산하거나 동일 source evidence로 바꾸지 마.

최소 통합 실행은 다음과 같다. 실제 최종 combined SHA와 새로운 isolated target에서 수행해줘.

```bash
cargo test --profile measurement --locked --offline -p rodas5p-integrators --test affine_event_certificate_contracts --test fast_dense_contracts
cargo test --profile measurement --locked --offline -p rodas5p-krylov --test basis_only_workspace_contracts -- --test-threads=1
cargo test --profile measurement --locked --offline -p vigilode --test '*' --no-fail-fast
python3 -m unittest discover -s research/synchronized_production_20261029/code -p 'test_*.py' -v
```

원본 PR102 bit verifier도 새 통합 source의 `sym02_certificate_probe` output bits로 실행해줘. 정확한 CLI는 원본 verifier/README를 읽고 고정해줘. facade ownership 변경 뒤 반환 값과 dense query displacement, chronological carry가 같은지 직접 확인하고, compiler arithmetic/nearest-ties-even/gradual-underflow 전제를 검증해줘. 원본은 x86_64 및 proof call 중 floating environment 불변 가정에 묶여 있다.

이 affine provider는 nonlinear Gamma/deltaJ/local-domain tube와 multi-component physical norm 또는 numerical clock–fiber 문제를 자동으로 닫지 않는다. A4의 `delta_j+G delta0+L Phi(B)(C+s delta0)` forcing와 original-coordinate recovery는 별도로 유지해줘. 실제 equation/source-offset embedding을 증명한 뒤 Bernstein/query/carry 기법을 재사용해줘. DOPRI나 다른 dense reconstruction은 실제 polynomial/partition부터 별도로 연결해줘. direct original residual이 이미 stage error를 포함하면 S의 stage perturbation을 다시 더하지 마.

### A. Native K3 admissibility — 우선순위1

`theory/KRYLOV.md` K3와 `code/directional_quotient.py`를 재사용해 다음 d=2 후보를 native opt-in 연구 경로로 이식할 조건을 완성해줘.

\[
A_\sigma=I-\sigma J,\quad p(z)=z^2-a z-b,\quad
D=1-a\sigma-b\sigma^2,
\]
\[
x=\frac{(1-a\sigma)v+\sigma Jv}{D},\qquad
v-A_\sigma x=\frac{\sigma^2}{D}(J^2v-aJv-bv).
\]

`σ=hγ≥0`, 고정 SPD metric W, `μ_W(J)≤μbar`, `α=1−σ μbar>0`에서 `||x−x*||_W≤||r||_W/α`다. 먼저 이 proof-to-code contract를 독립 검토하고 다음 의무를 해결해줘.

1. 현재 `crates/rodas5p-integrators/src/{polynomial_resolvent,conditioned_polynomial}.rs`, `physical_error_budget.rs`의 provider·epoch·원공간 residual 계약을 실제로 따라가줘. 새 RHS별 증거를 기존 full-space identity로 위장하지 마.
2. source/provider token, frozen J/preconditioner epoch, v bits, σ bits, metric/domain epoch, a/b enclosures 및 residual owner를 함께 묶어줘. 각 Rosenbrock stage RHS는 다른 v이므로 이전 stage의 relation을 자동 상속하지 마.
3. Jv·J²v·coefficient/Horner/division rounding·physical J uncertainty를 outward enclosure로 처리해줘. 후보를 정확식과 비교하는 ηx 방식과 original residual의 ηr 방식 중 하나를 정해 중복 과금하지 마.
4. `D` interval이0을 포함하거나 `α≤0`, provider mismatch, nonfinite/range loss, 허용 error cap 초과이면 기존 solver로 fallback해줘. 실패한 proposal의 모든 JVP·source·certificate·setup 비용을 보존해줘.
5. 첫 native test population은 exact rational/diagonal·nonnormal triangular operator, exact relation와 near relation, σ=0, pole/near-pole, v의 invariant subspace 밖 작은 성분, sign/normalization/epoch mutation을 포함해줘. 독립 Fraction solve/original residual oracle와 비교해줘.
6. admission이 확정된 다음에만 MGS/QR을 생략하는 opt-in prototype을 작성해줘. default dispatcher나 solver 선택은 검증된 조건 없이 바꾸지 마. matched-accuracy comparison은 cold setup, warm frozen J, proposal failure+fallback을 각각 포함해야 한다.

이것이 현재 가장 직접적인 수학 기반 성능 연구다. K1 allocation 감소와 달리 K3는 실제 MGS/QR 연산을 줄일 수 있지만, 작은 D·certificate 비용·높은 relation degree 때문에 손해가 날 수 있다. 측정으로 판별해줘.

### B. Actual physical whole-trajectory 및 clock–fiber provider

`ADAPTER.md` A4의 실제 reentrant model은 `x0'=1`, `xj'=−λ(x0)φ(xj−g(x0))+g'(x0)`, `φ(e)=e+e³/2`다. carrier variable `ej=xj−g(x0)`를 쓰면 `ej'=−λ(x0)φ(ej)`가 정확히 성립한다. 다만 clock direction은 neutral이며 clock defect가 fiber를 강제한다.

정리의 `κ,L,G,B,δ0,δj,C,Ej`를 실제 reconstructed slab에서 outward하게 공급해줘. 현재 글로벌 analytic 선택은 `κ=1`, `L=999900`, `G=5/2`이며 reconstructed fiber의 B는 별도 검증 대상이다. 시간 `s`에서 clock error `C+sδ0`와 fiber convolution을 함께 계산하고 original-coordinate error로 되돌려줘. endpoint를 whole-slab 최대오차로 대체하지 마.

우선순위0에서 PR102 affine provider와 실제 accepted reconstruction을 재사용한 통합 검증을 완료해줘. 그 뒤 polynomial/rational clock–fiber carrier/rate의 exact manufactured field로 원래 defect 및 subdivision certificate를 연결하고, 실제 tanh/sin/cos의 outward transcendental enclosures로 확장해줘. scalar requested-time affine provider와 autonomous numerical clock model의 차이를 식으로 매핑해줘. 실제 physical client를 사용할 때는 원래 state lifting, conservation/positivity, norm weights, clock, source/rate input, IC와 모든 rounding owner를 bind해줘. PR95/99의 rate-box remainder 및 δJρ+Γρ² 조건은 source 입력이 실제 존재하는 범위에서만 공급해줘. 누락된 rate/process cell을 fitted 숫자로 채우지 마.

`SYNTHESIS.md`의 stage bound를 연결할 경우 실제 RODAS coefficients·stage-domain에서 `α_i,L_ij,C0,C1,Lf`를 유도해줘. direct original-curve residual에 이미 stage effect가 들어있으면 그 stage term을 다시 더하지 마. slab adjacency·norm/reset transfer·event margin/transversality를 확인하고 method switches 전후 physical error가 같은 IVP에 대해 이어지는지 독립 검토해줘.

### C. Adapter/session 비용과 dense/banded 회귀

현재 A1은 반환 값의 ownership만 바꾼다. persistent explicit session은 이미 존재하고 implicit batch는 facade/workspace를 새로 구성한다. 다음 implicit storage reuse는 모든 dependency-sensitive cache를 fresh construction과 같게 reset하거나 epoch validity를 증명해야 한다. dimension만 맞는다는 이유로 Jacobian/factor/FSAL/controller/Krylov image를 재사용하지 마.

Dense/banded의 timing regression부터 source-bound local profile로 분해해줘. CPU affinity, frequency/background load, allocator, first/later solve, solve/export/drop 범위를 고정하고 최소한 paired block 전체를 보존해줘. `perf`, Callgrind, heaptrack이 있으면 실제 availability와 명령·version을 기록해줘. 없으면 해당 profile만 미실행으로 남겨줘. self와 inclusive 비용을 더하지 말고 startup/serialization/harness를 solver-only 비용으로 표시하지 마. allocator 계측 binary와 wall-time binary의 영향도 분리해줘.

Strict sentinel/RHS-only는 정책과 accepted trajectory가 다르다. 같은 정책의 baseline/candidate parity 및 cost를 우선 비교해줘. 이후 strict policy의 discarded work, replay windows, accepted segments, error와 dense interior를 모두 기록해 original same-accuracy objective로 비교해줘. A3의 local strict-win을 global speed theorem으로 바꾸지 마. 정책을 바꿀 후보는 `D≤βP+C0` 같은 명시적 overhead budget과 physical acceptance 계약을 먼저 증명해줘.

### D. Release/API/CI qualification

다음은 별도 gate로 관리하며 numerical PASS로 닫지 마.

- Q-API: 실제 downstream fixtures의 E0063/E0004 source break와 cargo-semver-checks의 baseline package discovery 실패는 다른 문제다. migration/version/compatibility 정책을 명시하고 양쪽 원본 fixture를 보존해줘.
- Q-GIVENS: CI RHS1e14에서 `Ok(3640)` 대 기존 expectedErr 차이가 남아 있다. input/output bits·compiler/profile을 내보내고 원래 residual을 exact/rigorous oracle로 먼저 판정해줘. 결과 pin이나 tolerance만 바꾸어 통과시키지 마.
- SUNDIALS/MPI: installed SUNDIALS7.1.1의 mpi.h include/compiler/link discovery 문제를 실제 provider/version contract로 해결해줘. 로컬 adapter pass가 CI portability pass를 대신하지 않는다.
- Q-NEGATIVE: WFiltered15/Off7 비용비2.143>1.3 및 Off interior311.467 대 WF0.350를 유지하고 matched-accuracy 문제로 다시 설계해줘.
- authority/node: 이전 full authority31 missing/16 repository-unavailable/11 invalid-repository/4 unreachable와40 inherited node diagnostics가 있었다. 최신 전달의 실제 checker receipt를 우선하고, CI skipped를 PASS로 바꾸지 마. 누락된 원본 preregistration이나 authority를 만들지 말고 실제 object/source를 취득해줘. 소진된 Q-LEDGER 예산은 유지한다.
- optional features, full release, platform, actual client 및10 interactions는 각 원래 수용 기준으로 검증해줘. 이번 focused lint PASS를 전체 CI green으로 표현하지 마.

## 6. 최종 synchronize·감사·반환

각 agent는 source pin, 가정, 식–코드 mapping, 실제 R1–R10/C1–C10 상태, 독립 review, 원자료, 실패 및 남은 의무를 같은 형식으로 반환해줘. PR102 통합 결과는 새 combined-tree 실행으로 따로 기록하고, 기존21 tests/381 checks를 새 실행에 합산하지 마. 마지막에는 K3 stage residual→S stage/reconstruction transfer→D/A4 whole-slab physical certificate→A1 output observation을 한 논증과 하나의 실행 lane에 연결해줘. 의미가 다른 원본 정책·정확도·callback cost를 섞어 합성하지 마.

새 native 채택마다 이론 audit 다음 code audit을 수행하고, 정확도/비용/회귀 plot을 직접 읽어줘. raw hash 검증 및 mutation test를 포함하되, 같은 완료 검증을 새로운 원인 없이 반복하지 마. strong no-go를 새 성과로 제시할 경우 NG1–NG7을 모두 갖춰야 하며, 특정 fixture 실패는 그 범위로 기록해줘.

최종 반환에는 실제 최종 tree, source-bound build/execute pins, 전체 실행 비용, 모든 실패·회귀, 증명한 조건과 native 구현된 범위, 생산 qualification의 열린 gate, 정확한 다음 명령을 포함해줘. 승인된 수정은 isolated branch에 non-force push하고 base=main draft PR로 전달해줘. 기존 source/history/WIP와 negative evidence를 보존하고, main merge나 production deployment는 이 인계 작업에 포함하지 마.
