# PR97 게시 후 CI: 실제 실패와 문서 수정

Initial results delivery는 `25e76dd84f1ca71fd460201bdf31476d3cce9545`이며 [PR97](https://github.com/cosmosapjw-quantum/vigilode/pull/97)의 base는 PR96이다. 이 문서는 게시 후 새로 관측한 CI 결과를 기존 과학적 원자료와 구별한다. 등록 native 성능·오차 결과의 source는 계속 `64004e23042f2cf076da5378be724f50c3856f4c`이다.

## 1. 이번 변경이 만든 rustdoc 실패를 수정

CI `docs` job은 `cyclic_closure.rs`의 세 `[0,1]` literal을 intra-doc link로 해석해 `-D warnings` 아래 실패했다. 동일 모듈의 `[lower, upper]` literal까지 **문서 주석 네 줄을 code span으로 감쌌다**. 유효한 `integrate_rk4` 함수 링크는 유지했다.

실행 Rust 코드·상수·테스트·oracle는 수정하지 않았다. 변경 전후 파일에서 `///`와 `//!` 문서 주석 줄을 제외한 모든 줄이 동일함을 확인했고 exact patch와 양쪽 SHA-256을 `evidence/ci25e/CARGO_DOC_RECEIPT.json`에 기록했다. 문서 attribute 문자열까지 byte-identical하다는 주장은 하지 않는다. `FINAL_SOURCE_BINDING.json`은 이 수정 전 initial delivery의 역사적 검증 snapshot이다.

같은 Rust 1.94.1과 locked/offline dependency, 공통 lock 아래 다음 **build-only** 검증을 완료했다.

```bash
RUSTDOCFLAGS="-D warnings" cargo doc \
  -p vigilode -p vigilode-coupled --no-deps --locked --offline
```

결과는 **exit 0**, 약 **68.874 s**이다. 문서 build를 성능 재실험으로 세지 않는다. Manual registered native campaign은 다시 실행하지 않았으며 이미 통과한 원래 source 결과를 소급 변경하지 않는다.

보수적으로 이 수정도 기존 N03의 두 번째 correction episode에 포함한다. **N03=2/3, N05=1/3**이고 R-STRUCT/R-ADAPT=3/3, Q-LEDGER=1/3은 유지한다. 전체 scientific claim이나 production 판정을 확대하지 않는다.

원래 실패 job: https://github.com/cosmosapjw-quantum/vigilode/actions/runs/40676899171/job/122777508109.

## 2. Authority 실패는 여전히 열린 gate

게시 후 CI의 더 완전한 checkout에서는 authority checker가 `missing=56`, `missing+allowlisted=27`, `non-commit=224`, `reachable=506`을 기록했다. Local partial-clone cached-object 감사의 missing178과는 보유 object 및 remote-ref 상황이 다르다. 이전 PR96의 missing49와도 동일 환경의 단순 regression count로 해석하지 않는다.

Byte-preserved PR95 입력의 외부 BASS commit은 VigilODE commit처럼 검사되어 실패 citation에 포함된다. Missing blob이 실제로 존재하면 non-commit으로 바뀌는 등 현재 checker의 object typing이 판정에 영향을 준다. 이 문제를 원본 수정·삭제나 blanket allowlist로 숨기지 않았다. Repository와 object kind를 보존한 authority schema, 필요한 object 확보와 기존 historical authority 정리가 local Q-LEDGER의 의무다.

원래 실패 job: https://github.com/cosmosapjw-quantum/vigilode/actions/runs/40676899131/job/122777507289.

## 3. 외부 comparator build의 실제 setup 실패

CI benchmark job은 native numerical benchmark를 수행하기 전에 CVODE C bridge build에서 실패했다. 직접 원인은 다음 include 경로이다.

```text
gcc ... cvode_bridge.c
/usr/include/sundials/sundials_types.h:57:10:
fatal error: mpi.h: No such file or directory
```

설치된 SUNDIALS header가 요구하는 MPI include를 실제 `gcc` invocation이 찾지 못했다. 관련 `.github/workflows/production.yml`, `crates/vigilode-bench/build.rs`, `native/bridge/cvode_bridge.c`는 PR96에서 변경되지 않았다. 이것은 이번 cyclic/trace 수치 회로의 accuracy 실패나 benchmark speed 결과가 아니다. 이번 환경에서 이 comparator setup을 해결·검증했다고 주장하지 않는다.

Local에서는 설치된 SUNDIALS의 serial/MPI 구성, 실제 compiler, include/link flags와 ABI를 먼저 확인하고 작은 bridge build를 검증한 뒤 comparator campaign을 실행해야 한다. 현재 build script의 C compiler 선택을 확인하지 않고 `CC=mpicc` 설정만으로 해결된다고 가정하지 않는다. 이 작업은 LOCAL-04/LOCAL-05의 실제 남은 build gate에 연결된다.

원래 실패 job: https://github.com/cosmosapjw-quantum/vigilode/actions/runs/40676899171/job/122777508650.

## 4. 증거와 현재 상태

네 job의 connector가 반환한 decoded log는 `evidence/ci25e/{docs,research-process,bench,test}.log.txt`에 보존했다. Native 등록 원자료의 byte-preserving gzip evidence와 별도 자료이다. GitHub PR workflow는 합성 merge checkout을 사용할 수 있으므로 PR head, 실제 job checkout와 등록 benchmark source를 혼동하지 않는다. 관련 job URL과 logs가 CI 실행 출처다.

Initial CI의 `clippy`와 `fmt`는 통과했다. 다른 broad CI 작업의 상태는 source-bound snapshot으로 읽고 최신 상태를 PR에서 확인한다. 새 numerical source에 대한 complete production PASS는 없다. **현재 production HOLD**, 최초 scientific results는 `RESULTS.json`, 최신 delivery/budget는 `CURRENT_STATE.json`을 따른다.

### 기존 Givens outcome pin의 새 CI 실패

CI `test` job도 기존 `givens_production_differential_contracts`에서 실패했다. RHS scale1e14에서 candidate가 `Ok(3640)`을 반환했지만 역사적 outcome table은 Err를 기대했다. 이 테스트는 반환 해의 별도 binary64 true-residual 검사를 status assertion보다 먼저 수행하므로 해당 검사는 통과한 뒤 outcome pin에서 중단됐다. 그러나 이 CI 관측을 exact Fraction oracle 성공으로 승격하지 않는다. 테스트 및 Krylov runtime 파일은 PR96에서 바뀌지 않았다. Runtime 생성 행렬의 정확한 A/b bits, debug/release 및 platform/libm 차이, 실제 returned x의 exact residual을 먼저 비교해야 한다. 역사적 성공/실패 표를 universal correctness theorem처럼 취급하거나 단순히 expected flag를 뒤집어 CI를 통과시키지 않는다. 필요하면 source/profile별 관측 pin과 보편적인 returned-residual 계약을 분리하되 negative control과 exact oracle를 유지하는 후속 수정으로 등록한다. 현재 Q-GIVENS는 이전 scoped PASS와 새 cross-environment HOLD를 함께 보존한다.

실패 job: https://github.com/cosmosapjw-quantum/vigilode/actions/runs/40676899171/job/122777508533. 전체 decoded log는 `evidence/ci25e/test.log.txt`이다.
