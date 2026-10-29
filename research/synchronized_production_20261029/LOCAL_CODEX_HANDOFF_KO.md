# Local Codex handoff — VigilODE 구조적 solver 연구의 후속 검증

아래 작업을 기존 연구의 연속으로 수행해줘. `$physmath-research-loop`를 사용해 병목별 이론 R1–R10을 먼저 수행하고, 검토된 식·가정·오차 소유권을 넘겨받아 코딩 C1–C10으로 진행해줘. 독립 문제는 subagent로 병렬 연구하고, 실제 native 실험은 소스·바이너리·프로토콜을 고정한 뒤 순차 실행해줘. 완료된 증명과 기존 구현은 재사용하고 변경된 의존성부터 검증해줘.

이번 후속 작업의 중심은 **이미 이식한 방향별 quadratic stage solver의 적용 범위와 선택 비용을 넓히고, 실제 물리 궤적 인증에 연결하는 것**이다. 단순 할당 감소를 성능 개선의 대리 지표로 삼지 마. 실제 연산량, 같은 정확도의 전체 IVP 시간, 거절·fallback까지 포함한 총비용으로 판정해줘. 전체 production 판정은 여전히 **HOLD**다.

## 출발 source와 증거

Repository: `cosmosapjw-quantum/vigilode`.
전달 branch: `research/synchronized-production-20261029`.
연구 node: `research/synchronized_production_20261029`.

| 구분 | 고정 identity |
|---|---|
| 기존 조사 baseline PR101 | `bc5ddbe42b24b1a6364bf653c3e2d6d938ca4087` |
| phase1 allocation/ownership 최종 native 실행 | `ef84aa3d6c028b0922bb5c1e66202b95fd88a2fc` |
| phase1 문서·raw checkpoint | `dd600057d16a7717c47409f7806c33dd515e9da5` |
| 구조적 quadratic native 실행 | `0a2a8eb9b9b0b775561f42f42cf36b1ef6e96f4b` |
| 해당 native tree | `5795e928f78fe3e99bfa91a4e7d10eaa7540c4db` |
| 조사 당시 main | `8d0c79184e09efb5bdadc24a6315c60a71a44264` |
| PR102 native affine delivery | `8357faa5b42724a8be24609d4d2f16aec8a6eb3a` |
| PR102 실제 native code 실행 | `0d722b4ed71413926c76bc0037f02f3a6bbdd362` |
| 뒤늦게 확인한 PR103 head, base=PR102 | `1845e1d8bdfda222bf77637e8d38d5076743f238` |

먼저 branch의 실제 원격 head를 fetch해 `DELIVERY_SHA`로 기록해줘. 문서 전달 commit과 native 실행 commit을 구분하고 `git diff NATIVE_SHA DELIVERY_SHA -- crates Cargo.lock Cargo.toml`을 확인해줘. metadata만 추가된 전달 commit을 native 실행 identity로 바꾸어 쓰지 마. 기존 dirty checkout은 보존하고 별도 worktree/branch에서 작업해줘. 최신 main 및 모든 관련 PR 상태를 재조회하고 필요한 통합만 수행해줘. 새 combined tree의 qualification은 원본 PR 증거와 별도로 기록해줘.

읽을 순서는 `REPORT_KO.md`, `STRUCTURAL_RESULTS.json`, `CURRENT_STATE.json`, `STRUCTURAL_PROTOCOL.json`, `theory/STRUCTURAL_NATIVE.md`, `theory/{KRYLOV,ADAPTER,DEFECT,SYNTHESIS}.md`, 구조적 독립 review, `RESULTS.json`과 두 raw manifest다. `reviews/PR102_REUSE_REVIEW.md`와 최종 PR103 검토 addendum도 읽어줘. raw packet은 manifest의 member digest부터 확인하고 현재 환경의 새 경로로 복원해줘. 과거 `/workspace/scratch/...` 절대 경로가 지금도 존재한다고 가정하지 마.

이번 구조적 실행의 확인된 결과는 다음과 같다: **동일 binary의2460회 full-IVP 실행,19 native tests PASS/2 ignored, 실제6 accepted/2 refused kernel records의88 exact Fraction checks PASS. 회전3.425배/큰 회전1.973배/감쇠 회전3.174배/time-JVP1.186배, 잘못된 계수0.804배. 모든35 paired blocks 및 원자료를 독립 재집계했다**. 정확한 테스트·실행 수와 source binding은 `STRUCTURAL_RESULTS.json`이 기준이다. phase1 결과나 PR102 원본 실행을 새 source의 재실행 수에 합산하지 마.

## 변경하지 않을 연구 회계

기존 **23-node/34-edge DAG, 73 paths, 8 suites, 10 interaction**, negative evidence, 원본 source identity와 correction budget을 보존해줘. ancestry, 원본-source replay, 현재 누적 native 검증, scoped pass는 각각 다른 상태다. 이번 focused kernel/IVP 검사를 기존 8 suites나 10 interactions 전체 통과로 올리지 마.

기존 cloud global correction episode는 **3/3**이다. 이후 사용자의 명시적 구조적 개선 요청에 따라 미구현 K3의 첫 native 이식을 수행했으며, **PR-N01은 2/3에서3/3으로 소진**했다. 이는 과거 repair 횟수의 초기화가 아니다. 이 구조적 최초 이식 뒤 추가 numerical repair iteration은 **0(수치 변경·사후 성능 조정 없음; 최초 이식의 비수치 lint 교정은 별도 기록)**이다. R-STRUCT, R-ADAPT, PR-N05, Q-LEDGER도 각3/3을 유지한다. PR-N04는2/3, Q-LINT는 최초 이식의 비수치 lint 교정 후3/3, Q-BENCH는1/3이며 다른 node는 최종 `CURRENT_STATE.json`을 계승해줘.

이 prompt로 승인된 로컬 검증·연구는 별도 execution id와 runtime/compile 예산으로 사전등록해줘. 기존 소진 과제를 이름만 바꾸어 재시작하지 마. 이미 닫힌 회차를 더 실행해야 하는 의무와 새 수학적 문제의 최초 연구를 구분하고 실제 controlling node를 기록해줘. cloud scientific 누적 시간 **602.900460228/900초** 및 모든 실패 실행은 보존하며, cloud 잔여 시간을 로컬 자동 승인 예산으로 해석하지 마.

## 이미 확보한 구조적 알고리즘

`crates/rodas5p-integrators/src/directional_quadratic.rs`와 `Rodas5pMfFastWorkspace::set_directional_quadratic_proposal`이 이미 구현되어 있다. 기존 ordinary MF-fast 경로에서 opt-in으로 실행하며 기본값은 off다. 두 번째 동일 prototype을 만들지 마.

원래 stage 선형계에서 `A=I−σJ`, configured warm start `x0`, `v=f−Ax0`를 두고

\[
D=1-a\sigma-b\sigma^2,\qquad
q=\frac{(1-a\sigma)v+\sigma Jv}{D},\qquad
x_{candidate}=x_0+q
\]

를 구성한다. 직접 전개로

\[
v-Aq=\frac{\sigma^2}{D}(J^2v-aJv-bv)
\]

가 성립한다. 현재 방향의 관계면 충분하며 전체 공간의 다항식 항등식이나 다른 stage의 관계로 자동 승격하지 않는다. supplied a,b는 후보 생성 가설이다.

실제 구현은 live shifted action을 써서 numerator를 `(2−aσ)v−Av`로 계산하고, full candidate의 **원래 live residual**이 기존 `max(linear_atol, linear_rtol*||f||₂)`를 통과할 때만 채택한다. 분모·범위·잔차 거절은 출력값과 warm start를 보존한 채 기존 GMRES로 fallback하고 모든 시도 비용을 남긴다. 실제 callback 오류와 불완전/nonfinite action은 오류로 전달한다. initial residual이 이미 허용오차 이내인 shortcut은 별도로 기록한다. 이 floating residual 기준 자체는 rigorous physical certificate가 아니다.

비영 shift의 비자명한 성공은 zero start에서2 action, nonzero start에서3 action과 O(n) vector 작업으로 Arnoldi/MGS/QR를 생략한다. 비용이득 조건은

\[
(m_{base}-m_{accept})C_J+C_K>C_V,
\quad
p(C_{base}-C_{accept})>(1-p)C_{refusal}
\]

다. 모든 성공에 무조건 빠르다고 결론내리지 마. 연산 계측의 `dot/axpy`는 기존 계측 kernel을 세며 새 일반 vector loop의 전체 FLOP 수를 뜻하지 않는다. 실제 wall time은 그 작업도 포함한다.

특정 회전 모델은 `J²=−9I`, `D=1+9σ²`, `||(I−σJ)⁻¹||₂=1/sqrt(1+9σ²)`다. 반면 16개 서로 다른 rate의 time-JVP 모델은 exact degree2 관계가 일반적으로 불가능하다. endpoint-root 후보는 `p(z)=(z+2)(z+31/8)`이고 이 **정규 대각 J**에서 `||p(J)||₂≤225/256`이다. 따라서 작은 shift의 근사 후보를 정당화하지만, 임의의 nonnormal J에 같은 spectral 논증을 적용하면 안 된다.

원래 native bit oracle은 실제 emitted binary64 회전 kernel records를 Fraction으로 복원해 exact represented original residual과 resolvent error를 확인한다. 그 유한 records는 전체 ODE stage나 모든 floating operator의 enclosure가 아니다. 17-point 전체 IVP 비교는 binary64 analytic diagnostic이며 whole-slab 인증과 구분한다.

## 병렬 후속 연구 단위

### A. 원래 residual로 인증하는 GMRES prefix stopping과 구조적 선택 — 우선순위1

**먼저 현재 GMRES의 full-restart 비용을 연구해줘.** `gmres_into.rs`의 일반 cycle은 happy breakdown이 없으면 restart 전체 Arnoldi columns를 만든 뒤 least-squares를 한 번 푼다. time-JVP baseline은144개 stage에서각16 columns를 사용해2304 columns다. 구조적 후보는18개를 받아2016 columns로 줄였지만126회 거절 때문에 JVP는2718→2790, strict integer work는3060→3132로 늘었다. 그럼에도 wall time은1.186배 빨랐다. 이 값은 해당 호스트·문제의 MGS 비용 감소를 반영하며 모든 비용 목적함수의 동시 개선을 뜻하지 않는다.

현재 raw solve log에서18개의 zero-iteration entry는 모두 각8-stage attempt의8번째 위치다. aggregate18 accepted와 일치하지만 이 관찰을 모든 stage의 degree2 근사 관계나 일반 time-dependent 문제에 대한 정리로 바꾸지 마. stage RHS/initial residual/absolute threshold의 실제 크기를 먼저 조사해줘.

유망한 다음 구조 변경은 **full restart 전의 prefix candidate를 원래 residual로 검사하는 종료 규칙**이다. K2 residual-gap과 현재 Givens 연구를 재사용해 projected residual/Gram defect/roundoff를 분리하고, 채택 시 기존 original-residual threshold를 유지해줘. 매 prefix의 true action/QR 갱신 비용을 포함한 총비용식으로 검사 간격과 종료 조건을 설계해줘. projected residual만 낮다고 현재 native true-residual 계약을 생략하지 마. 기존 Givens profile 불일치도 먼저 exact represented-input oracle로 판정해야 한다. 이 후속 문제의 실제 controlling node와 남은 예산을 사전등록하고 소진된 PR-N01의 수정 횟수를 이름만 바꾸어 우회하지 마.

현재 두 action 후보를 native에 다시 만드는 대신 다음 차이도 연구해줘.

1. 현재 코드의 정확한 JVP 비용과 stage별 GMRES 차수, relation defect, warm-start 잔차, 성공/거절률을 같은 source에서 profile해줘. 후보 성공만 골라 총비용을 계산하지 마.
2. `rotation`, `rotation-large`, `damped-rotation`, `time-jvp`, `badcoeff`를 그대로 재현한 뒤 실제 문제의 low-degree invariant directions, nearly low-degree directions, nonnormal transient growth와 varying-J epochs를 조사해줘. 모델별 정리와 적용 범위를 먼저 확정해줘.
3. 자동 admission은 degree·coefficient·denominator·metric·source epoch와 비용 조건을 가져야 한다. 과거 stage의 빠른 성공만으로 미래 관계나 payback을 보장하지 마. selector가 만드는 probe와 refused work까지 비용 inequality에 넣어줘.
4. 높은 degree, shifted recycling, relation learning 중 실제 병목에 필요한 확장 하나를 선택해 K2/K4의 residual-gap 및 shifted-image 계약을 재사용해줘. one-pass MGS 전환 등으로 숨은 Gram/relation error를 없애지 마.
5. 실제 stage bits와 independent original residual oracle를 확장해 coefficient/range/normalization/epoch mutation, near-pole, invariant subspace 밖 작은 성분, nonnormal triangular model을 검증해줘. numerical target을 낮추어 성공률을 높이지 마.

이 단위의 수용 조건은 **동일한 물리 정확도 목표에서 전체 solve의 총비용 개선**이다. default route를 바꾸려면 scoped 모델 이외의 반례와 actual-domain 적용 근거가 필요하다. 부적합한 모델은 명시적으로 기존 GMRES 경로에 남겨줘.

### B. PR102 재사용와 actual physical certificate — 우선순위1

PR102 native scalar-affine provider는 이미 존재한다. `affine_event_certificate.rs` 및 `rodas5p_fast_dense.rs`의 accepted quartic/Bernstein/query/carry 구현을 읽고 새 isolated integration branch에서 재사용해줘. 두 branch의 실제 diff와 현재 main 상태를 다시 확인하고 **실제 combined tree**를 publish/build/test해줘. 이번 cloud structural source에 PR102를 병합·재실행한 것으로 표시하지 마.

PR102 원본 결과는 code0d722…의21 tests와381 checks(367 Fraction+14 Decimal90)다. 6 records 중2개,42 queries 중14개만 actual native다. 이 숫자를 현재 실행 수에 더하지 마. native-source proof는 caller가 선언한 물리 equation/source/clock/unit/IC의 진실성까지 증명하지 않는다.

`Gamma=deltaJ=0` affine 특수 경우부터 combined-tree 검증을 완료하고, 실제 nonlinear source/rate/domain provider로 넓혀줘. 다음 점을 유지해줘.

- 실제 accepted quartic을 cubic으로 줄이면 마지막 H 항을 잃는다. represented endpoint 차의 h enclosure와 source coefficient uncertainty를 원래 residual 안에서 다뤄줘.
- query displacement rho는 해당 returned observation에만 더하고 매 query마다 physical jump로 carry하지 마. 다음 slab로 넘기는 physical carry와 구분해줘.
- `D+E≤−αE+ΓE²+δ`, `ΓR²−αR+δ≤0`의 compact-domain 전제를 실제 outward provider로 공급해줘. 누락된 입력을 fitted 상수로 채우지 마.
- clock–fiber 모델은 neutral clock error를 유지한다. A4의 `delta_j+G delta0+L Phi(B)(C+s delta0)` forcing와 원좌표 회복항을 scalar requested-time affine 계약으로 대체하지 마.
- 직접 인증한 original dense-curve residual에 stage error가 이미 들어 있으면 S의 triangular stage-reconstruction 항을 다시 더하지 마.
- slab adjacency, norm/reset transfer, event margin/transversality, physical clock와 source/domain epoch를 chronological certificate로 연결해줘.

PR103은 chemical6 product simplex/ratebox에서 signed weighted-infinity logarithmic norm bound를 다룬다. 최종 read-only 검토의 확인 범위는 **정확 signed-row/vertex/rate-endpoint 정리는 scoped static 수용했다. 다만 일반 isotope 입력의 electron coefficient outward 준비, event-dual arithmetic guard, public interval 유효성 경계가 미완료여서 native 일반 입력 재사용은 HOLD다**. 조건부 bound14.2857% tightening을 ODE 실행시간 향상으로 바꾸어 쓰지 마. 얻은 mu가 양수이면 dissipativity가 증명된 것이 아니다. 원본 R6/C9 미실행은 원본 상태로 유지하고, 현재 lane의 native 재실행 수에 넣지 마.

이 결과는 D의 carrier/defect tube에 필요한 metric별 rate bound 입력으로 재사용할 수 있다. 실제 vector accepted-quartic reconstruction, 원래 nonlinear residual와 remainder, product-simplex/ratebox inclusion, weighted-infinity↔L2/WRMS norm transport를 함께 연결해줘. metric weights와 rate/domain epoch를 바꾸면 기존 margin을 자동 상속하지 마. positive mu의 finite-time growth bound와 contraction case를 구분하고, bound tightening 자체의 계산 비용까지 측정해줘. PR103도 이번 structural source에 merge/replay한 것으로 세지 않는다.

Native evaluator를 일반 입력에 연결하기 전에 세 가지 검토 finding을 닫아줘. (1) 정확 represented isotope `wli`의 전하 계수 `3*wli`를 nearest binary64 point로 바꾼 뒤 exact인 것으로 사용하면 안 된다. outward 정수곱 또는 명시적 discrepancy enclosure를 공급하고 비자명한 mantissa를 exact oracle로 검증해줘. (2) `electron_event_dual_upper`의 public entry에도 arithmetic-mode guard 또는 검증된 context를 연결해줘. (3) public `Iv` 필드로 우회 가능한 finite/ordered endpoint 조건을 event 경계에서 검증해줘. 원래7개 dyadic fixture가 이 범위 결함으로 실패했다는 주장은 하지 않는다.

Chemical RHS가 상태의 이차식이라는 사실은 Jacobian의 minimal polynomial degree가2라는 뜻이 아니다. PR103으로 quadratic stage solver의 a,b를 자동 생성하지 마. 얻은 양의 logarithmic norm을 D의 양의 contraction alpha로 대입하지 말고 growth 계약을 사용해줘. whole-domain full Jacobian bound가 이미 포함한 nonlinear variation을 같은 오차의 Gamma 항으로 다시 더하지 마.

### C. Adapter/switcher와 기존 dense/banded 회귀 — 우선순위2

phase1의 할당 감소와 banded7개 block 전부의 시간 회귀를 보존해줘. persistent implicit session을 추가한다면 dimension뿐 아니라 J/factor/FSAL/controller/Krylov cache의 epoch와 reset 동치를 증명해줘. strict sentinel/RHS-only는 accepted trajectory가 다른 정책이므로 비용비를 순수 wrapper overhead로 해석하지 마.

동일 정책 baseline/candidate부터 비교한 뒤 discarded work, replay, switching residence와 같은 정확도의 dense interior를 포함한 비용을 비교해줘. A3 local strict-win을 global speed theorem으로 바꾸지 마. 새로운 선택 정책은 정량적 variation/payback 또는 interval guarantee와 physical acceptance를 함께 제시해줘.

### D. Release/API/CI qualification — 독립 gate

Q-API E0063/E0004, cargo-semver baseline discovery, Givens RHS1e14의 profile별 차이, SUNDIALS/MPI compiler/include/link discovery는 각각 분리해줘. WFiltered15/Off7=2.143>1.3와 unequal interior error도 보존해줘. full authority 및 node checker의 최신 receipt를 읽고 skipped를 PASS로 바꾸지 마. 소진된 Q-LEDGER는 실제 authority object 취득 의무로 유지하며 allowlist나 과거 preregistration을 만들어 통과시키지 마.

## 재현·벤치마킹 규칙

각 worktree의 `CARGO_TARGET_DIR`를 분리해줘. phase1에 shared-target stale artifact가 발견되었으므로 build exit0만으로 provenance를 판단하지 마. Rust/compiler flags, CPU/affinity/threads, Cargo.lock/vendor hashes, native file hashes 전후, source/tree/binary/build receipt를 함께 저장해줘. 원격 publication/fetch/readback이 새 native scientific run보다 앞서야 한다.

구조적 비교는 **같은 binary**의 `--arm gmres|proposal`을 사용해줘. 다음은 source 고정 이후 실행할 개별 명령 예시이며 실제 runner의 전체 protocol과 receipts를 대체하지 않는다.

```bash
cargo build --profile measurement --locked --offline -p vigilode --example structural_quadratic_probe
cargo test --profile measurement --locked --offline -p rodas5p-integrators --lib directional_quadratic -- --test-threads=1 --nocapture
cargo test --profile measurement --locked --offline -p rodas5p-integrators --test directional_quadratic_contracts --test matrix_free_problem_contracts --test spd07_mf_step_warm_start --test time_fd_workspace_contracts -- --test-threads=1 --nocapture
```

환경 `VIGILODE_SOURCE_COMMIT`를 실제 published SHA로 설정한 상태에서 emitted bits를 저장하고 `code/verify_structural_bits.py --input ... --expected-source ... --output ...`로 독립 검증해줘. test 출력의 actual count를 확인하며 zero tests exit0을 PASS로 세지 마. `relative_residual>rtol`만으로 실패라고 판정하지 마. absolute atol이 지배할 수 있으므로 원래 norm residual과 `max(atol,rtol*||rhs||)`를 비교해줘. PR102 통합 뒤에는 그13 affine+8 dense contracts와 원래 source-specific bit oracle도 새 실제 output으로 재실행해줘.

cloud `run_structural.py`는 당시 source와 phase1 charge를 그대로 검증하는 immutable protocol이다. 이를 로컬 새 source에서 실행하려면 원본 파일·과거 receipt를 유지하고 새 source-transition/run-id/budget protocol을 먼저 사전등록해줘. 문자열 SHA나 inherited charge만 바꾸어 과거 실행처럼 표시하지 마.

시간 측정은 고정된 controller, represented initial state, grid, precision, tolerance와 동일 error cap을 사용해줘. phase1보다 엄밀한 reference enclosure 및 whole-slab 목표를 도입한다면 모든 arm에 동일하게 적용해줘. cold setup, solve, export/drop, query와 certificate 비용을 명시하고 ABBA/BAAB 전체 blocks, 느려진 표본, 실패·timeout을 남겨줘. min/max는 confidence interval이 아니다. `perf`/Callgrind/heaptrack은 현재 availability를 확인하고 실제 command/version/source를 기록해줘. 도구가 없으면 function/hardware profile은 미실행으로 남긴다. 기존 WorkCounters를 모든 FLOP 계측으로 해석하지 마.

## 최종 synchronize 및 전달

각 agent는 source pin, 가정, 정확한 정리와 식–코드 mapping, 실제 R1–R10/C1–C10 상태, 독립 review, 실패와 남은 의무를 반환해줘. 마지막에는 **방향별 stage 후보 → 원래 residual → stage/reconstruction 전달 또는 direct original defect → nonlinear/clock–fiber whole-slab 인증 → event/route-word → 반환 관측**을 하나의 source-bound lane으로 합성해줘.

원자료와 code를 새 branch에 non-force push하고 base=main draft PR로 제출해줘. 기존 branch·main은 merge/force-push하지 마. 검증한 native SHA와 문서 delivery SHA를 각각 기록하고, 실제 final tree의 테스트·정확도·총비용·profile·negative evidence·remaining gates를 보고해줘. scoped 성과가 전체 production 승격을 정당화하는지 마지막에 따로 판정해줘.

현재 authority receipt: source0a2a8eb9의 cached-object scopeFAIL(missing1/unreachable3), fullFAIL(missing59/repository-unavailable16/invalid-repository11/unreachable3). missing을 remote object 부재로 단정하지 마. 첫 full audit tool interruption과 현재 failure를 함께 보존해줘. `theory/STRUCTURAL_SYNCHRONIZATION.md`와 `evidence/STRUCTURAL_PROCESS_AUDIT.json`을 추가로 읽어줘.

최종 문서 staging에서는 node/append-only186rows PASS, scoped authority missing1/unreachable5 typed identities(9diagnostics) FAIL이다. 앞선 native-tree scoped3unreachable와 혼동하지 말고 `evidence/final_process`의 exact receipt와 stdout을 읽어줘. 같은 실패 SHA가 raw와 요약의 다른typedidentity로 중복 집계된 부분도 보존한다.
