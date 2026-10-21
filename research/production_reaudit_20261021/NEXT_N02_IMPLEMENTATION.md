# N02: 실제 빌드 소스 결속과 독립 참조 계층 구현

N02의 코드 이식을 완료했다. 실행 중의 Git HEAD를 컴파일된 소스의 식별자로 사용하던 경로를 없애고, 빌드 시점의 실제 파일 내용으로 실행 파일을 결속했다. 별도의 RADAU5/CVODE 정밀화 프로토콜도 구현했다. 다만 전체 bench crate와 native adapter의 통합 컴파일은 이 환경에서 검증하지 못했다. 따라서 이번 결과는 **구현 완료 / native 통합 및 NCP 실측 대기**이며 production 승격이 아니다.

## 소스 결속

`src/source_identity.rs`와 `build.rs`가 모든 repository crate, 내부 Cargo path dependency, 루트 Cargo manifest/lock/toolchain, coefficient fixtures, native bridge 및 vendored source, Rust literal include의 재귀적 자산을 정렬된 SHA-256 manifest로 만든다. Git 없이도 동작한다. 외부 repository path dependency, symlink 입력과 지원하지 않는 computed include는 조용히 누락하는 대신 거절한다. Registry dependency는 Cargo.lock checksum 및 Cargo 검증을 신뢰한다. 시스템 공유 라이브러리의 바이트까지 이 source manifest가 증명하지는 않는다.

`wp` 시작과 종료 시 manifest를 대조하고 corpus 및 실행 파일 자체의 SHA-256도 비교한다. 소스·corpus·binary 변경을 발견하면 결과 JSON을 발행하지 않는다. 기본값은 dirty Git tree도 거절한다. 명시적인 `--allow-dirty-source`는 내용 검증을 유지하면서 예외를 기록하므로 상위 승격 gate가 제외할 수 있다. 빌드 Git commit은 build context로만 남기며 dirty 또는 미확인 build에는 `git_commit = null`이다. Gitless source archive의 정확한 내용 식별은 유효하다.

조건부 불변식은 다음과 같다. 동일 파일 목록 및 SHA-256 충돌이 없다는 통상적 전제 아래, `VERIFIED_BUILD_SOURCE`는 시작·종료 경계에서 검사한 source closure가 embedded build manifest와 같고, 읽은 corpus 및 실행 파일이 같은 바이트였음을 뜻한다. 실행 전체의 지속적 감시나 악의적인 host에 대한 attestation을 뜻하지 않는다. source checkout을 실행 중 정지된 상태로 유지해야 한다. 해시는 solver row의 측정 구간 밖에서 계산한다.

`harness::run_corpus`는 낮은 수준의 관측 함수로 유지된다. 그 출력 자체에는 인증된 provenance가 없다. 검증 receipt를 붙이는 경로는 `wp`이며, 상위 gate는 receipt를 요구해야 한다.

## 독립 참조의 새로운 계층

새 파일 `crates/vigilode-bench/protocols/n02_independent_reference_v1.json`에 다음을 사전 지정했다.

| 항목 | 고정 규칙 |
| --- | --- |
| 계층 | `radau5_cvode_refinement_v1` |
| 구현 | Hairer RADAU5와 SUNDIALS CVODE dense |
| rtol | `1e-11 → 1e-12 → 1e-13` |
| atol | 각 문제의 reference_atol × `1 → 0.1 → 0.01` |
| 공통 오차 | 기존 common WRMS, `error_rtol = 1` |
| 두 구현 사이 허용 차이 | 각 level에서 `1e-10` 이하 |
| 한 구현의 연속 refinement 차이 | 두 번째 level부터 각각 `1e-10` 이하 |
| 행의 baseline | 가장 정밀한 RADAU5 결과 |

모든 level과 두 구현이 전체 grid에 성공해야 한다. coarse level 실패가 finer level 성공으로 가려지는 first-success fallback은 없다. 실패·불완전 grid·비유한 차이 또는 threshold 초과는 `REFERENCE_DISPUTED`이다. 모든 실패 status와 시도 tolerance가 receipt에 남는다. `--require-independent-reference`는 기존 corpus를 잘못 선택한 실행도 사전에 거절한다.

판정은 모든 level의 조건의 논리곱이다. 따라서 한 level의 실패를 나머지 성공으로 승격할 수 없다. 두 구현이 모든 level에서 서로 같더라도 refinement가 불안정하면 거절된다. 이 조건은 독립 구현 사이의 grid 관측에 관한 증거다. 정확해에 대한 상계나 grid 사이 시간의 오차 enclosure로 바꾸어 해석하면 안 된다.

기존 corpus는 optional protocol object를 생략하여 기존 의미를 유지한다. published E1b JSON, corpus, threshold 및 당시 verdict는 변경하지 않았다. 이번 새 계층의 필요조건을 역사적 결과의 당시 규칙에 소급 적용하지 않는다.

## 검증과 한계

| 실제 수행 | 결과 | 증거의 범위 |
| --- | --- | --- |
| source_identity standalone Rust tests | 8 PASS | dependency/fixture/lock 변경, 추가·삭제, native 입력, literal include, path dependency, symlink, corpus/binary 변경, Gitless snapshot |
| reference_policy standalone Rust tests | 6 PASS | 기존 policy 4개 + 새 strict refinement/실패 판정 2개 |
| 실제 harness/corpus/report를 mock adapter와 컴파일·실행 | 18 PASS | 새 control-flow 계약 5개, 기존 policy/metric/model 13개 포함 |
| 실제 build.rs의 rustc 컴파일 | PASS | build script의 Rust 타입/문법. native 컴파일을 실행했다는 뜻 아님 |
| 실제 repository source snapshot/verify | PASS | 해당 시점 closure의 일관성. 최종 package identity는 root manifest를 따름 |
| 전체 bench crate / native adapter | BLOCKED, 실행 안 함 | 41개 exact-lock dependency version 및 gfortran/SUNDIALS 개발 의존성 부재 |

mock test는 실제 `harness.rs`, `corpus.rs`, `report.rs`, `metrics.rs`, `model.rs`, `reference_policy.rs`, 실제 Arm/RunOutput 선언을 사용한다. Solver 실행 함수만 명백한 mock이다. 실패 coarse level, 공통 drift, success status의 짧은 grid, finest baseline 선택, legacy serialization을 검증했다. 이를 실제 RADAU5/CVODE 결과나 native integration PASS로 기록하지 않는다. 테스트 집합에 중복 항목이 있으므로 합산하여 고유 테스트 수로 보고하지 않는다.

증거는 `evidence/n02/`에 있고, source 및 receipt SHA-256은 `NEXT_N02_IMPLEMENTATION.json`에 있다. 변경된 scientific benchmark의 실측은 이 환경에서 수행하지 않았다.

## 수학/코딩 루프의 종료 상태

| 단계 | 수행 |
| --- | --- |
| 1–2: 질문·범위 | N02의 잘못된 runtime HEAD 귀속 및 독립 참조 tier 부재를 한정 |
| 3–4: 입력·가정 | 기존 corpus/result 재사용, build-source closure 및 reference 규칙 명시 |
| 5–6: 구성·구현 | source manifest/경계 검사 및 전체 refinement ladder 구현 |
| 7–8: 적대적 반례·수정 | source/lock 교체, 실패 coarse level 숨김, 공동 drift, incomplete grid를 검사 |
| 9: 재검증 | 실제 standalone 및 mock-adapter compilation/test receipt 확보 |
| 10: 인계 | native 미검증 범위를 유지한 채 local 검증과 NCP 실측 명령 고정 |

## 다음 실행 단계

1. Local Codex 6 Luna low는 exact locked dependency와 native 개발 도구가 준비된 환경에서 `cargo test --locked --manifest-path crates/vigilode-bench/Cargo.toml`을 실행한다. 남은 변경 범위는 컴파일/통합 오류 및 작은 수정이다. 참조 threshold와 공개 결과는 변경하지 않는다.
2. NCP에서는 source-clean checkout과 repository 밖의 `CARGO_TARGET_DIR` 및 결과 폴더를 사용한다. measurement profile로 빌드하고 새 corpus에 `--require-independent-reference`를 지정하여 실행한다. 구체적 명령은 `crates/vigilode-bench/protocols/README.md`에 있다.
3. 상위 promotion gate는 source/corpus/binary 및 raw/receipt hash closure를 확인한다. 독립 reference PASS와 원래 tolerance에서의 모든 행·반복 성공이 있어도 그것은 empirical grid 자격이다. broad production·전역 정확도 또는 speed 승격은 별도 필요한 증거를 모두 충족해야 한다.

추가 구현을 local에 미루기 위한 계획서가 아니라, 위 코드와 테스트가 이미 있는 상태의 검증 인계다. native 의존성 부재로 확인하지 못한 부분을 PASS로 대체하지 않는다.
