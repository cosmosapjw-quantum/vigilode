# 로컬 병목 측정 패키지

이 패키지는 개발 PC에서 실제 실행한 결과를 보존하고, 정확도와 병목을 구분하기 위한 실행기다. 예상 환경은 Ryzen 5900X, 64 GB, Ubuntu 24.04이지만 실제 CPU 이름, 허용된 CPU 집합, 물리 코어, SMT 형제, L3 공유 범위, governor를 실행 시 수집한다. NCP 접속, 패키지 자동 설치, governor 변경, 권한 상승은 수행하지 않는다.

## 측정 범위

`local_bottleneck_suite`는 첨부 요청의 24개 슬롯을 실제 우변으로 구현한다. 이름이 모호하거나 부정확한 문제의 구체적 변형과 모든 매개변수는 `--list`와 `--describe`에 기록된다. 이 목록은 문헌의 원 문제를 모두 재현했다는 뜻이 아니다. 특히 2번은 허용된 HIRES 대안이며, 19번은 전체 비선형 천수방정식이 아닌 명시된 선형 구면 모드 시험이다.

시간 적분은 전체 구간을 한 번에 푸는 endpoint-only 계약이다. `serial`은 한 IVP의 지연시간, `data`는 동일 IVP의 우변 데이터 병렬화, `ensemble`은 같은 IVP의 독립 복제 처리량이다. 작은 문제를 동일하게 복제한 ensemble은 매개변수가 서로 다른 실험을 대표하지 않는다. 두 층의 속도비를 섞지 않는다. stage 병렬화는 별도 `stage_campaign.py`에서 측정한다. 어떤 경우에도 data와 ensemble과 stage의 스레드 수를 동시에 곱하지 않는다.

24개 문제 × 5개 스레드 수준은 **120개의 문제/스레드 설정**이다. 여기에 방법, 독립 프로세스 반복, 별도 serial 기준, 프로파일링이 추가된다. 실제 프로세스 수와 각 프로세스가 모두 timeout에 도달했을 때의 합계는 `plan`이 출력한다. Dense는 차원 256 초과에서 계획 단계부터 제외한다. 실제 머신의 허용 물리 코어가 모자라면 해당 설정은 실행하지 않는다.

## 준비와 실행

최종 게시 commit과 tree는 별도 게시 영수증/최종 handoff에서 받아야 한다. 문서 안에 자기 자신의 최종 commit을 넣을 수 없으므로 아래 placeholder를 추정해서 채우지 않는다. `prepare_worktree.sh`는 기존 checkout을 변경하지 않고 고정 commit의 별도 detached worktree를 만든다.

```bash
bash research/local_frontier_20261024/local/prepare_worktree.sh EXISTING_REPO EXPECTED_COMMIT EXPECTED_TREE NEW_CHECKOUT
```

이후 새 checkout에서 실행하며 출력과 Cargo target은 checkout 밖에 둔다. `build_local.py`는 `--locked --offline`을 기본 사용한다. 의존성 캐시 누락이면 실패 로그를 보존한다. 네트워크로 고정 lockfile의 패키지를 받도록 명시적으로 결정한 경우에만 `--online-locked`를 사용한다. Cargo.lock을 바꾸거나 임의 버전을 설치하지 않는다.

```bash
python3 -B research/local_frontier_20261024/local/build_local.py --expected-commit "$VIGILODE_COMMIT" --expected-tree "$VIGILODE_TREE" --output "$RUN_ROOT/build" --target-dir "$RUN_ROOT/target"
python3 -B research/local_frontier_20261024/local/campaign.py plan --config research/local_frontier_20261024/local/smoke.json --binary "$RUN_ROOT/target/measurement/examples/local_bottleneck_suite" --build-receipt "$RUN_ROOT/build/BUILD_RECEIPT.json" --expected-commit "$VIGILODE_COMMIT" --expected-tree "$VIGILODE_TREE" --output "$RUN_ROOT/SMOKE_PLAN.json"
python3 -B research/local_frontier_20261024/local/campaign.py run --plan "$RUN_ROOT/SMOKE_PLAN.json" --output "$RUN_ROOT/smoke"
python3 -B research/local_frontier_20261024/local/campaign.py analyze --output "$RUN_ROOT/smoke"
```

`standard.json`은 3개의 독립 프로세스 반복, 1/2/4/8/12개 물리 코어, 네 방법을 요청한다. 프로세스당 60초, 캠페인 accounted budget 7200초, 주소 공간 상한 32 GiB다. 모든 설정을 2시간 안에 완료한다는 보장은 없다. 예산 때문에 실행하지 못한 설정을 결과에 표시한다. 재개 시 이전 완료 프로세스 시간과 중단된 시도의 전체 timeout을 차감하고 현재 invocation의 wall time을 더한다. 이전 invocation의 제어기 overhead는 복원하지 않으므로 이는 모든 실행 사이 공백까지 합한 엄밀한 총 wall-clock 제한은 아니다. 실시간 2시간 종료가 필요하면 외부 `timeout 7200s`로 한 invocation을 감싸고, 중단된 영수증은 보존한다.

동일 plan/output으로 재실행하면 완료/실패 영수증을 덮어쓰지 않고 건너뛴다. 실패를 다시 시도하려면 `--retry-failed`를 명시한다. 새로운 attempt 번호로 저장된다. commit/tree, dirty 상태, binary SHA256, plan hash가 달라지면 기존 캠페인에 섞을 수 없다. 게시 측정은 성공한 build 영수증의 source와 binary hash도 검증한다. `--allow-dirty-smoke`는 개발 중 smoke에만 허용되며 성능 비교에서 제외한다.

## 정확도와 외부 비교

7개 슬롯은 해석해 또는 명시된 상계로 endpoint를 검증할 수 있다. 나머지 모델의 성공적인 종료는 `UNVALIDATED_REFERENCE`이며 성능 순위에 들어가지 않는다. 독립 참조해를 나중에 확보할 경우 동일 시간, 매개변수, 초기값, 노름과 참조 오차를 먼저 등록해야 한다. 성공 플래그나 보존량 하나만으로 전체 오차를 인증하지 않는다.

`accuracy.json`은 진동 사례 9/10/15/16/19/20에서 16개 양의 물리 시각으로 별도 cold solve를 수행한다. 초기값까지 17개 시각이지만 하나의 연속 적분 궤적을 관찰한 것이 아니다. 이 소요시간들을 하나의 trajectory latency로 합산하지 않는다. endpoint 또는 wrapped phase의 일치만으로 빠뜨린 진동 주기가 없음을 보장할 수 없다. 시간 사이의 위상 오차를 검증하려면 더 조밀한 독립 참조 또는 연속 오차 추정이 필요하다.

`external_compare.py`는 현재 계약의 1/10/15/16번만 대응한다. descriptor의 매개변수와 초기값을 읽고 임의 두 상태/시각의 RHS probe를 비교한 뒤 실행한다. VODE는 SciPy가 감싼 double-precision VODE이며 Python callback 비용을 포함한다. 별도 native Fortran 프로그램의 속도로 해석하지 않는다. Julia는 설치된 executable과 `Project.toml`/`Manifest.toml`이 있는 프로젝트를 명시할 때만 Tsit5/Rodas5P를 실행한다. Julia warmup과 import는 별도 기록/제외되며 이 환경에서는 Julia 실행을 확인하지 않았다. diffsol은 현재 24모델 계약에 맞는 adapter가 아직 없어 **NOT_RUN**으로 남긴다. 이전의 다른 IVP 수치를 가져와 채우지 않는다.

```bash
python3 -B research/local_frontier_20261024/local/external_compare.py --binary "$RUN_ROOT/target/measurement/examples/local_bottleneck_suite" --build-receipt "$RUN_ROOT/build/BUILD_RECEIPT.json" --expected-commit "$VIGILODE_COMMIT" --expected-tree "$VIGILODE_TREE" --output "$RUN_ROOT/external" --size smoke --repeats 3
```

오차 기준은 동일하게 최대 성분 절대 endpoint 오차 ≤ 1e-4다. 이는 단위가 다른 전체 24문제 사이의 공통 물리 정확도를 뜻하지 않는다. 외부 비교는 동일 문제 안에서만 해석한다. 1번 Robertson은 독립 endpoint 참조가 없어 이 실행에서 순위를 매기지 않는다.

## 병목을 읽는 방법

| 항목 | 직접 수집 | 해석의 한계와 다음 조치 |
|---|---|---|
| RHS/JVP/Jacobian/시간 미분 | callback 횟수와 opt-in 시간 | callback timer가 켜진 실행은 깨끗한 시간 비교에서 제외한다. |
| projection/factorization/선형해법 | solver 통계 및 perf stack | 개별 내부 시간 계측이 없는 경로를 임의 비율로 나누지 않는다. |
| controller/rejection/pilot/fallback | 단계 통계, 별도 stage 표본 | callback 밖의 나머지 시간 전부를 controller라고 부르지 않는다. |
| allocation/메모리 | GNU time peak RSS, perf call stacks | peak RSS는 할당 횟수가 아니다. allocator 이벤트 측정은 별도 필요하다. |
| pool/synchronization | pool 시작 시간, thread scaling, perf | ensemble callback 시간은 겹치므로 wall time에서 단순 차감하면 안 된다. |
| I/O/프로세스 시작 | process wall과 solve timer의 차이 | import/reference/JSON/warmup 등이 함께 포함되며 I/O만의 시간이 아니다. |

`perf`가 없거나 권한이 막히면 callback 계측과 `/usr/bin/time -v`만 유지한다. sudo, perf_event_paranoid 변경, 자동 패키지 설치는 하지 않는다. CPU counters가 허용되지 않아 perf가 실패한 raw 출력도 보존한다. 프로파일링은 별도 프로세스로 수행하며 `perf-record`의 sampling overhead를 성능 우위의 증거로 쓰지 않는다. 타이밍이 없는 부분은 `UNMEASURED`로 남긴다.

`plot_campaign.py --analysis ANALYSIS.json --output NEW_FIGURES_DIR`는 완료된 정확도 적격 반복의 median/range만 그린다. 비교 가능한 표본이 없으면 그림을 꾸며내지 않고 설명 파일만 만든다. 런타임 결과는 production 자격을 자동으로 승격시키지 않는다.

## Stage 연구 실행

`build_local.py --example reconsideration_bench`로 별도 build receipt를 만든다. `stage_campaign.py`는 few-mode, nonlinear-changing-rank, nonlinear-step-regime, meanfield-manufactured의 causal/Picard/residual-first/guarded 후보를 1/2/4/8/12 스레드에서 비교한다. 마지막 사례는 실제 비분리 상호작용과 제조 외력을 계산하며 합성 연산량 태우기를 사용하지 않는다. 각 프로세스는 방법마다 한 번 warmup 후 한 표본을 만들며 프로세스를 기본 3회 반복한다. 이 예제의 17개 output landing은 endpoint-only 24문제 시험과 계약이 다르므로 결과를 합쳐 순위를 내지 않는다.

## 검증

`python3 -B -m unittest discover -s research/local_frontier_20261024/local -p 'test_*.py' -v`는 timeout, 잘못된 출력, source/build/binary 불일치, 중복 설정, 물리 코어 제한, 재개, 미완료 반복 제외, 참조 오차 필수, RHS probe 불일치를 검증한다. 이 테스트는 제어기 신뢰성 검사이며 solver의 정확도 증명이 아니다.

## 조건부 후보의 정확도와 강건성 실험

`CONDITIONAL_CAMPAIGN_KO.md`는 chart/continuation/structured residual/invariant module의 별도 캠페인을 설명한다. `build_local.py --example`는 네 예제를 각각 올바른 crate로 빌드한다. 이 실험에서는 속도가 다소 느려져도 원래 좌표의 오차와 잔차 보증, 거절 후 안전한 복구, 조건 변화에 대한 강건성 이점을 함께 측정한다. 조건부 kernel 실험의 결과를 전체 ODE 성능 승격으로 해석하지 않는다. 실행 전 `python3 -B research/local_frontier_20261024/src/verify_package.py`로 게시 파일의 content identity를 확인한다.

## 먼저 실행할 병목 전용 계획

긴 timing campaign이 시간 예산을 먼저 소진하여 마지막 profile들이 실행되지 않는 경우를 피하려면, smoke 다음에 **`profiling.json`을 별도 plan/output으로 먼저 실행**한다. 표준 크기24문제의 지원되는 방법별로 직렬 계측만 실행한다. 이 configuration의 `accuracy_only=true`는 기존 제어기의 timing 반복 생략 기능을 사용하기 위한 것이며, 이 계획에는 accuracy probe가 없고 모든 task가 instrumented profile이다. 최대81개 profile process, process당20초, accounted budget1800초다. perf-record가 불가능하면 callback 계측을 유지하고 실패/미계측을 보존한다.

이30분 profile 예산은 standard의2시간과 별도다. 전체 실행을2시간으로 제한하려면 checkout 밖에 configuration 복사본을 만들고 profile예산1800초+standard예산5400초처럼 분배해 plan에 기록한다. 원본 configuration이나 source를 편집하지 않는다. profiling 결과를 clean latency 비교에 넣지 않는다.
