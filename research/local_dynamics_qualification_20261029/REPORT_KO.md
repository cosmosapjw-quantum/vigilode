# VigilODE 누적 로컬 qualification 보고서

## 판정

**현재 판정은 HOLD다.** 이번 작업은 누적 로컬 검증을 정리한 것이며 broad production 또는 전체 interaction/DAG 통과를 승인하지 않는다. 컴파일·계약 시험, 제조된 F6 궤적의 scoped oracle 검사, smoke 결과, instruction profile은 서로 다른 증거다. 실패와 미완료 gate를 그대로 유지한다.

## 소스 identity와 범위

| Lane | Source identity | 용도 |
|---|---|---|
| 전달 문서가 고정한 delivery | `bf308589e21b524d0ddc6e571b260f5e94a853f0` | 인계 기준. 역사적 실행 SHA와 혼동하지 않음 |
| 역사적 dynamics 실행 | `58ec9b053163da6b6f0906010e0422c104db72fd` | 원래 evidence replay identity |
| 현재 최종 실행 소스 | `0255297e4f865bb78753cad9fe51d6f27536d188`, tree `4b1b7e7da6fa5e1fdcd064c0de1d19671c31e5b3` | Givens/SUNDIALS scoped tests, vendor build, coupled Clippy; F6 실행은 아래 별도 lane |
| suite receipt source | `a1a459f2be186d9f67ff86bf38c19ae24b769768` | 8개 suite receipt만; current native source와 다른 lane |
| F6 native source | `299310290c2e254149ee30f4a9834c1123066815` | F6 예제 실행; allocation followup는 suite source `a1a459f2` |
| Q-API baseline/current | `3064d453091af1dec7f99f0c8afabd98b447ea34 / 0255297e4f865bb78753cad9fe51d6f27536d188` | 독립 consumer compile fixture |

Cargo.lock SHA256은 `efb379071fa42c2b6bdbe969163ac2f41cf318c262c5c67d0910450298333351`이다. 실행 호스트 `cosmosapjw`, Linux 7.0.0-38-generic x86_64 / glibc 2.39, Rust/Cargo 1.94.1이다. Native F6는 CPU 0에 고정되고 suite runner는 `RAYON_NUM_THREADS=1`, `OMP_NUM_THREADS=1`, `OPENBLAS_NUM_THREADS=1`, 단일 테스트 스레드를 설정했다. 상세 run identity와 각 바이너리 해시는 [SUMMARY.json](SUMMARY.json)의 `source_lanes`와 raw manifest에 있다.

원본-source evidence replay와 현재 누적 native 수행은 다른 evidence lane이다. 기존 이론·source-locked 결과를 새 current native 실행으로 표시하지 않았다. DAG 범위는 23 nodes / 34 edges, 73 coverage paths, 10 interaction gates다. 이번 suite 통과를 이 전체 범위로 승격하지 않았다.

## 8개 등록 suite

| Suite | 상태 | Source | exit | 관찰 |
|---|---|---|---:|---|
| `facade-contracts` | `PASS_SCOPED_SOFTWARE_CHECK_ONLY` | `a1a459f2be186d9f67ff86bf38c19ae24b769768` | `0` | 297 tests passed; no scientific admission. |
| `coupled-contracts` | `PASS_SCOPED_SOFTWARE_CHECK_ONLY` | `a1a459f2be186d9f67ff86bf38c19ae24b769768` | `0` | 156 passed, 1 ignored; no scientific admission. |
| `all-targets` | `PASS_SCOPED_COMPILE_ONLY` | `a1a459f2be186d9f67ff86bf38c19ae24b769768` | `0` | workspace package all-target check; compile-only. |
| `release-contracts` | `PASS_SCOPED_SOFTWARE_CHECK_ONLY` | `a1a459f2be186d9f67ff86bf38c19ae24b769768` | `0,0` | both release test commands completed; aggregate stdout test rows include 2055 passed and 76 ignored; command scopes overlap other suites. |
| `docs` | `PASS_SCOPED_DOCS_ONLY` | `a1a459f2be186d9f67ff86bf38c19ae24b769768` | `0,0` | cargo doc 및 doctest 2개 통과. |
| `smoke-release` | `PASS_SCOPED_SMOKE_ONLY` | `a1a459f2be186d9f67ff86bf38c19ae24b769768` | `0` | 4 returned-state rows report pass; max reported state error 3.3718316472253207e-9; smoke only. |
| `local-benchmarks` | `DIAGNOSTIC_ONLY` | `a1a459f2be186d9f67ff86bf38c19ae24b769768` | `0,0,0,0,0,0,0,0` | 8 commands exit 0; results/profiles remain cohort diagnostics, not production qualification. |
| `local-profiles` | `DIAGNOSTIC_ONLY_GUEST_INSTRUCTIONS` | `a1a459f2be186d9f67ff86bf38c19ae24b769768` | `0,0` | Valgrind guest instruction counts with startup subtracted; not wall-time. |

`release-contracts` 두 명령은 모두 완료됐고 출력을 합산하면 2055 passed, 76 ignored로 파싱된다. 패키지 suite가 겹치므로 이 숫자는 전체 workspace 고유 test 수가 아니다. 최초의 다른 release 시도에는 1800초 timeout 후 917 pass가 기록되었으며 bounded retry는 1196 pass/54 ignored로 끝났다. 그 실패/재시도 비용과 결과는 역사 receipt에 보존되었다. 이번 current suite receipt의 새 release 명령들은 exit 0이다.

Smoke의 네 반환상태 row가 모두 pass flag를 냈고 최대 reported state error는 `3.3718316472253207e-9`였다. 이것은 해당 smoke 모델 범위의 소프트웨어/정확도 증거다. 전체 물리 모델 또는 production qualification이 아니다. Local benchmarks 8개 exit 0은 diagnostic runner 완료다. 같은 source의 paired benchmark 요약과 197개 원자료 파일 hash는 [BENCHMARK_SUMMARY_KO.md](evidence/f6-analysis/BENCHMARK_SUMMARY_KO.md) 및 [BENCHMARK_SUMMARY.json](evidence/f6-analysis/BENCHMARK_SUMMARY.json)에 있다. cubic 128×512 workload에서 generic/scalar checksum은 같았고 generic/scalar 중앙 비율은 1.244였다. Algebraic/rational arm은 generic과 checksum이 달라 출력 동치가 아니므로 그 arm의 속도 비율은 matched-output 개선 근거가 아니다. Structural workload는 common-grid 오차 cap `1e-6` 및 PR91 대비 grid bits 일치를 기록했고, 차원 2048, 단일 thread에서 fused/enclosed paired median은 1.157x/2.270x였다; 단일 diagnostic cohort다. Adapter 성능의 stiff-cubic observed PR91은 median 0.975x (범위 0.941–1.005)로 명확한 이득이 없고, reentrant-nonlinear observed strict sentinel은 RHS-only 대비 0.378x (0.376–0.386) 및 instruction slope +141.8%로 큰 손실이다. 이 모든 비교는 benchmark/profile 전용이다. Valgrind profile의 cubic arm 지표는 generic 3,018,359.75 대비 rational 2,104,980 instructions/repetition, structural 2048/1에서 enclosed는 PR91 310,184,546 대비 107,185,903 instructions/repetition이었다. 이는 startup 제외 guest instruction count이지 벽시계 속도나 전체 ODE 비용이 아니다.

## 추가 scoped 결과와 반례

- **F6 manufactured trace:** native executable SHA256 `789d54cd69eba31981c81546b8fe2c686f72ad723d3c281d7dbc86613ff5ef95`. 992 slabs / 65,672 exact-oracle checks, native exit 0, oracle `PASS_SCOPED_ALL_EXPORTED_SLABS_CHECKED`; 8/8 corruption/semantic negative controls 거부. Physical whole-path cap `1e-8`은 grid 16–128에서 충족하지 않았고 256에서 처음 native/outward 및 external exact upper 모두 충족했다. 작은 차원·특정 다항식 manufactured trace 결과이며 실제 chemical client 또는 전역 trace theorem이 아니다. route별 setup은 측정되지 않았다.
- **Q-GIVENS:** current source `0255297e`, measurement profile test exit 0. 정확 잔차 oracle 세 입력(`1e-14`, `1`, `1e14`)에서 `PASS_SCOPED`; 범위 밖 Krylov 전반의 qualification은 아니다.
- **Q-API:** baseline의 두 downstream fixture는 exit 0. current fixture는 `SwitchingConfig` exhaustive literal에서 E0063, `SwitchingEvidence` exhaustive match에서 E0004를 재현(exit 101 각각). 이것은 source compatibility break가 존재한다는 증거이며 호환성 승인이나 migration 해결이 아니다. Baseline `SwitchingConfig`는 Deserialize를 구현하지 않아 JSON deserialize 비교는 `NOT_APPLICABLE`.
- **Q-LINT:** first strict coupled Clippy attempt는 필요한 target environment 누락 상태의 불완전 `running` receipt로 남아 있다. 별도 retry가 source 0255297e에서 exit 0이다.
- **Q-LEDGER:** scoped typed repair/test 자체가 전체 authority를 닫지 않는다. 실행 source `0255297e`와 작성 중인 비추적 보고서가 함께 존재한 worktree에서 full authority 검사 exit 1: 10,368 citations / 1,299 distinct identities 중 missing 31, repository-unavailable 16, invalid-repository 11, unreachable 4이며 별도 allowlisted 항목과 non-commit 상태가 있다. [영수증](evidence/final-authority-check/RECEIPT.json)과 [전체 진단](evidence/final-authority-check/stdout.log)을 보존했다. 이전 main/PR97/PR94 관측은 각 source별로 분리 보존되며 allowlist나 과거 citation 변경으로 녹색 처리하지 않았다.
- **SUNDIALS/MPI:** `crates/vigilode-bench` adapter suite에서 7 tests, exit 0은 관측했다. 배포 vendor/header/link qualification은 여전히 `PENDING`; 이 부분 성공을 외부 runtime 완성으로 승격하지 않았다.
- **Q-BENCH:** adapter test 범위는 scoped pass다. 독립 vendor/deployment qualification은 pending이다.

Raw F6 분석에는 grid별 오차·RHS 수·setup/certificate times 및 negative controls가 있으며 [f6-analysis/SUMMARY.json](evidence/f6-analysis/SUMMARY.json)의 SHA256은 `0bd0b5fb3f4c66bfaffd739c3d21fd8f479ae91d02fc1c4dfbe4509d4f81b25b`이다. 모든 receipt/stdout/stderr/manifest는 로컬 외부 evidence root에 불변 보존되어 있다. 그 중요 artifact의 absolute path, size, SHA256은 아래 표 및 `SUMMARY.json`에 있다.

## DAG 23 nodes

| ID | Node | inherited | current | 근거/제한 |
|---|---|---|---|---|
| `S-PRS` | Inspect PRs after PR91 | `complete_scoped` | `PENDING_OR_INCOMPLETE` | No evidence in the current serial-suite receipt snapshot closes this DAG node; software checks do not imply scientific PASS. |
| `R-STRUCT` | Charged structural attempts and full residual enclosure | `complete_scoped` | `PENDING_OR_INCOMPLETE` | No evidence in the current serial-suite receipt snapshot closes this DAG node; software checks do not imply scientific PASS. |
| `R-HOMO` | Scalar homotopy and rational cubic inverse | `complete_scoped` | `PENDING_OR_INCOMPLETE` | No evidence in the current serial-suite receipt snapshot closes this DAG node; software checks do not imply scientific PASS. |
| `R-ADAPT` | Exact return evidence and causal proposal reuse | `complete_scoped` | `PENDING_OR_INCOMPLETE` | No evidence in the current serial-suite receipt snapshot closes this DAG node; software checks do not imply scientific PASS. |
| `I-PROD` | Explicit production configuration and callback permissions | `complete_scoped` | `PENDING_OR_INCOMPLETE` | No evidence in the current serial-suite receipt snapshot closes this DAG node; software checks do not imply scientific PASS. |
| `V-CLOUD` | Native contracts, small paired benchmarks and profiles | `complete_scoped` | `PENDING_OR_INCOMPLETE` | No evidence in the current serial-suite receipt snapshot closes this DAG node; software checks do not imply scientific PASS. |
| `P-PUSH` | Publish implementation, evidence and local handoff | `complete_scoped` | `PENDING_OR_INCOMPLETE` | No evidence in the current serial-suite receipt snapshot closes this DAG node; software checks do not imply scientific PASS. |
| `L-RELEASE` | Pinned native release verification on the deployment machine | `pending_local` | `PASS_SCOPED_SOFTWARE_ONLY` | Both current release test commands at suite source a1a459f2 exited 0; bounded suite receipt only, not deployment qualification. |
| `L-HARDWARE` | Matched-accuracy measurements on target hardware | `pending_local` | `DIAGNOSTIC_ONLY` | Local benchmark/profile commands completed at suite source a1a459f2. Profiles are guest instruction counts; no hardware or broad timing authority. |
| `PR-N01` | Fused polynomial actions, resolvent alternatives and small-Krylov cost | `pending_local` | `NOT_RUN` | F6 고정 manufactured 경로는 native 및 exact oracle을 수행했고 generic causal closure와 실제 client 범위는 HOLD다. |
| `PR-N02` | Tighter finite-step cubic accuracy planning | `pending_local` | `NOT_RUN` | F6 고정 manufactured 경로는 native 및 exact oracle을 수행했고 generic causal closure와 실제 client 범위는 HOLD다. |
| `PR-N03` | Adaptive structural consumer with physical-error accounting | `pending_local` | `NOT_RUN` | F6 고정 manufactured 경로는 native 및 exact oracle을 수행했고 generic causal closure와 실제 client 범위는 HOLD다. |
| `PR-N04` | Owned session continuation and generic homotopy workspace | `pending_local` | `NOT_RUN` | F6 고정 manufactured 경로는 native 및 exact oracle을 수행했고 generic causal closure와 실제 client 범위는 HOLD다. |
| `PR-N05` | Generic causal closure and parallel work/span selection | `pending_local` | `NOT_RUN` | F6 고정 manufactured 경로는 native 및 exact oracle을 수행했고 generic causal closure와 실제 client 범위는 HOLD다. |
| `PR-N06` | Temporal residence and exploration-loss theory | `pending_local` | `NOT_RUN` | F6 고정 manufactured 경로는 native 및 exact oracle을 수행했고 generic causal closure와 실제 client 범위는 HOLD다. |
| `R-QUALIFY` | Scoped release qualification for the actual client | `pending_local` | `HOLD` | Qualification remains blocked by authority/API and incomplete route/client evidence. |
| `Q-LEDGER` | Research registration and published-reference qualification | `registration_repaired_full_authority_pending` | `FAIL_FULL_AUTHORITY_PENDING` | Typed repair/checker work does not close repository/object authority: latest full authority command still exits 1 with missing/unavailable citations. Global HOLD. |
| `Q-GIVENS` | Portable Givens residual fixture | `pending_local` | `PASS_SCOPED_CURRENT` | The 0255297e exact differential test passed; 3 Fraction-oracle fixtures report PASS_SCOPED. |
| `Q-FAIR-AB` | Attempted, admitted and refused work | `pending_local` | `PRIOR_PASS_SCOPED_CURRENT_RECHECK_PENDING` | Earlier callback-accounting scope passed; broader interaction/provider callback admission remains open. |
| `Q-BENCH` | Qualified external comparator build | `pending_local` | `PASS_SCOPED_ADAPTER_ONLY_ENVIRONMENT_PENDING` | 7 adapter integration tests pass; package/vendor/SUNDIALS deployment qualification remains pending. |
| `Q-LINT` | Broader integrator Clippy | `pending_local` | `PASS_SCOPED_CURRENT` | Coupled all-target strict Clippy at source 0255297e exited 0; first attempt lacked target environment and remains an incomplete attempt. |
| `Q-API` | Public SwitchingConfig compatibility | `pending_local` | `BREAKING_CHANGE_REPRODUCED` | Baseline fixtures pass; current fixtures reproduce targeted E0063/E0004. This is a real downstream source break, not compatibility approval. |
| `Q-NEGATIVE` | Retained negative evidence and ignored suite | `pending_local` | `REPRODUCED_SCIENTIFIC_FAIL_RETAINED` | Historical W-filtered cost failure 15/7 > 1.3 and inaccurate Off arm remain failures; a negative regression does not promote the route. |

## 10 interaction gates

| Gate | registered | current | 상태 이유 |
|---|---|---|---|
| `I-STATE` | `PROSPECTIVE_NOT_RUN` | `PENDING_NOT_RUN` | No interaction-level receipt found in the current suite snapshot; F6 manufactured certificate and component suites do not close this gate. |
| `I-EPOCH` | `PROSPECTIVE_NOT_RUN` | `PENDING_NOT_RUN` | No interaction-level receipt found in the current suite snapshot; F6 manufactured certificate and component suites do not close this gate. |
| `I-WORK` | `PROSPECTIVE_NOT_RUN` | `PENDING_NOT_RUN` | No interaction-level receipt found in the current suite snapshot; F6 manufactured certificate and component suites do not close this gate. |
| `I-BUDGET` | `PROSPECTIVE_NOT_RUN` | `PENDING_NOT_RUN` | No interaction-level receipt found in the current suite snapshot; F6 manufactured certificate and component suites do not close this gate. |
| `I-POLY` | `PROSPECTIVE_NOT_RUN` | `PENDING_NOT_RUN` | No interaction-level receipt found in the current suite snapshot; F6 manufactured certificate and component suites do not close this gate. |
| `I-ROOT` | `PROSPECTIVE_NOT_RUN` | `PENDING_NOT_RUN` | No interaction-level receipt found in the current suite snapshot; F6 manufactured certificate and component suites do not close this gate. |
| `I-STRUCTURE` | `PROSPECTIVE_NOT_RUN` | `PENDING_NOT_RUN` | No interaction-level receipt found in the current suite snapshot; F6 manufactured certificate and component suites do not close this gate. |
| `I-NEGATIVE` | `PROSPECTIVE_NOT_RUN` | `PENDING_NOT_RUN` | No interaction-level receipt found in the current suite snapshot; F6 manufactured certificate and component suites do not close this gate. |
| `I-PROFILE` | `PROSPECTIVE_NOT_RUN` | `PENDING_NOT_RUN` | No interaction-level receipt found in the current suite snapshot; F6 manufactured certificate and component suites do not close this gate. |
| `I-CLIENT` | `PROSPECTIVE_NOT_RUN` | `PENDING_NOT_RUN` | No interaction-level receipt found in the current suite snapshot; F6 manufactured certificate and component suites do not close this gate. |

10개 interaction 모두 `PENDING_NOT_RUN`이다. suite 결과나 component test는 interaction receipt로 대체하지 않는다.

## Correction counters (누적)

| Task | cumulative used / max | counter source |
|---|---:|---|
| `R-STRUCT` | 3/3 | `LOCAL_CODEX_HANDOFF_KO.md` |
| `R-HOMO` | 1/3 | `LOCAL_CODEX_HANDOFF_KO.md` |
| `R-ADAPT` | 3/3 | `LOCAL_CODEX_HANDOFF_KO.md` |
| `I-PROD` | 1/3 | `LOCAL_CODEX_HANDOFF_KO.md` |
| `Q-LEDGER` | 3/3 (frozen 1) | `LOCAL_CODEX_HANDOFF_KO.md` |
| `PR-N03` | 2/3 | `LOCAL_CODEX_HANDOFF_KO.md` |
| `PR-N04` | 1/3 | `LOCAL_CODEX_HANDOFF_KO.md` |
| `PR-N05` | 3/3 (frozen 2) | `LOCAL_CODEX_HANDOFF_KO.md` |
| `PR-N01` | 0/3 | `cumulative_validation_20261028/SUMMARY.json` |
| `PR-N02` | 0/3 | `cumulative_validation_20261028/SUMMARY.json` |
| `PR-N06` | 0/3 | `cumulative_validation_20261028/SUMMARY.json` |
| `Q-GIVENS` | 0/3 | `cumulative_validation_20261028/SUMMARY.json` |
| `Q-FAIR-AB` | 0/3 | `cumulative_validation_20261028/SUMMARY.json` |
| `Q-BENCH` | 0/3 | `cumulative_validation_20261028/SUMMARY.json` |
| `Q-LINT` | 1/3 (frozen 0) | `cumulative_validation_20261028/SUMMARY.json` |
| `Q-API` | 1/3 (frozen 0) | `cumulative_validation_20261028/SUMMARY.json` |
| `Q-NEGATIVE` | 0/3 | `cumulative_validation_20261028/SUMMARY.json` |

Q-LEDGER 3/3, PR-N05 3/3, Q-LINT 1/3, Q-API 1/3은 이번 누적 episode에 추가 기록한 값이다. 괄호의 frozen 값은 전달된 이전 snapshot이다. 기존 controlling task와 budget identity를 유지했으며 이름 변경이나 budget reset은 없었다. 다른 counter는 전달된 값 그대로다. Negative evidence와 rejected controls는 그대로 유지한다.

## Raw artifact hash index

큰 native raw 결과는 저장소에 복사하지 않았다. 아래는 대표 raw/receipt hashes이고 전체 인덱스와 42개 artifact identity는 `SUMMARY.json`에 포함되어 있다.

| Absolute path | bytes | SHA256 |
|---|---:|---|
| `/home/cosmosapjw/evidence-20261028T170155Z/gate-draft.json` | 41057 | `8381c6dc0db14bac4dbaba9040b9f9558582b38268ed9b3e06dd44f86805fded` |
| `/home/cosmosapjw/evidence-20261028T170155Z/f6-analysis/SUMMARY.json` | 34871 | `0bd0b5fb3f4c66bfaffd739c3d21fd8f479ae91d02fc1c4dfbe4509d4f81b25b` |
| `/home/cosmosapjw/evidence-20261028T170155Z/f6-analysis/REPORT_KO.md` | 5791 | `6563423618aaabd61ad346fde31f9cc9fe74bce81785392d7c78f371ff4587cf` |
| `/home/cosmosapjw/evidence-20261028T170155Z/final-f6-native/RECEIPT.json` | 1602 | `e49b35a2e32cf94ced258d1d5fae9da902664b3b624c2076eec8a4abc4b7bf41` |
| `/home/cosmosapjw/evidence-20261028T170155Z/final-f6-native/stdout.log` | 5759216 | `2f37066ca4ca39821b5f61cc2ad6b14f5dc10a053f70bbfd809658fbe1e5ed9f` |
| `/home/cosmosapjw/evidence-20261028T170155Z/final-f6-native/stderr.log` | 2260 | `e04712e6ef0b69392f69c94ef011331edbd24f860e6a51030d84480dba562f66` |
| `/home/cosmosapjw/evidence-20261028T170155Z/final-f6-native/SOURCE_INPUTS.json` | 140569 | `7b5aed93c45d0e7a7ca4b0e3d4c0263fb2398db4da7810d36404b74225ebcd78` |
| `/home/cosmosapjw/evidence-20261028T170155Z/final-f6-oracle/RECEIPT.json` | 1691 | `c5f49b6f01fa032c9deaf11ba9a8a601b0574ecaa5a34758759c027caac572a3` |
| `/home/cosmosapjw/evidence-20261028T170155Z/final-f6-oracle-result.json` | 10677416 | `d93f79a45077afeb0270c14d112f4ec149a2ebd4896637ff27eaa7aca620ae91` |
| `/home/cosmosapjw/evidence-20261028T170155Z/qapi-final3/SUMMARY.json` | 37013 | `69baa345d385b324fb92a576f5772e1af6dd81be8a173b5ebd2d681ef4c473f3` |
| `/home/cosmosapjw/evidence-20261028T170155Z/qapi-run3.stdout` | 203 | `aa5b9ffea490def9d4ea1824c37d44620d48d970338f2bcc8b8f3c3a946fe26c` |
| `/home/cosmosapjw/evidence-20261028T170155Z/qapi-run3.stderr` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `/home/cosmosapjw/evidence-20261028T170155Z/qapi-run3.exit` | 2 | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |
| `/home/cosmosapjw/evidence-20261028T170155Z/givens-final-025/RUN.json` | 926 | `b96c88b5323258e18eddbaec43bba5300eab0f87c43e2244d23eedf963f49743` |
| `/home/cosmosapjw/evidence-20261028T170155Z/givens-final-025/givens-exact/RECEIPT.json` | 1631 | `7b5ce55f3bb9252a6083ef4f2bec4505dba665dadd1ff029e76c3d8b180a308a` |
| `/home/cosmosapjw/evidence-20261028T170155Z/givens-final-025/givens-exact/stdout.log` | 714 | `28e3016e26000a7d6da832749d94c4ee9cdf2caf74345a0acc28b152d6e1b8b2` |
| `/home/cosmosapjw/evidence-20261028T170155Z/givens-final-025/givens-exact/stderr.log` | 553 | `f14d4dbd2bf8ba0454b44efa9c9d3b0dca4346110a9f1e38349aa752a11ff382` |
| `/home/cosmosapjw/evidence-20261028T170155Z/givens-final-025/givens-exact/SOURCE_INPUTS.json` | 140782 | `6aef9f5317bc938892cb5699489522d8360d4263e40dd127b5ca9099bb7e6c24` |
| `/home/cosmosapjw/evidence-20261028T170155Z/suites-a1/facade-contracts/command_00/RECEIPT.json` | 1613 | `44e9030002a73fbc3202489c40a11c1b22eeee5c5d567b21568a5215ccf514fa` |
| `/home/cosmosapjw/evidence-20261028T170155Z/suites-a1/coupled-contracts/command_00/RECEIPT.json` | 1620 | `2074b9d31b88053e96fff77145c30631c1a469ed13ad2fbc2afa1863d5bdde95` |
| `/home/cosmosapjw/evidence-20261028T170155Z/suites-a1/all-targets/command_00/RECEIPT.json` | 1605 | `b04bc41744f902668d78fab03b0c40f17a92f9007fbf1ce6b414f1dc82ed4a3e` |
| `/home/cosmosapjw/evidence-20261028T170155Z/suites-a1/release-contracts/command_00/RECEIPT.json` | 1689 | `ecd46e0064f40a50c055328bb90f61be8188a3f55687b6987d1e60d8a3f2dba7` |
| `/home/cosmosapjw/evidence-20261028T170155Z/suites-a1/release-contracts/command_01/RECEIPT.json` | 1605 | `22d5e80f60fe96268ba619b7523757fb5bb35074c1413faf531605f939b274d5` |
| `/home/cosmosapjw/evidence-20261028T170155Z/suites-a1/docs/command_00/RECEIPT.json` | 1601 | `738fd662891eaa202d25c2d793d85d19d7b25f10987fa87c8415500fca1eb4b4` |
| `/home/cosmosapjw/evidence-20261028T170155Z/suites-a1/docs/command_01/RECEIPT.json` | 1597 | `23929597cb973fb1bd89665be16c7629ed3e753a61b5fb389f2729c0ff5adf11` |
| `/home/cosmosapjw/evidence-20261028T170155Z/suites-a1/smoke-release/smoke-release/RECEIPT.json` | 4174 | `6a53014fd79a1930eafff9d9269dfff8e2f3dbfd1ace8bef900f55f062f8ef1e` |
| `/home/cosmosapjw/evidence-20261028T170155Z/suites-a1/smoke-release/smoke-release/SUMMARY.json` | 854 | `f72a785876f2d08782606b7f451070654e2675dc16935e9648ad84ece2d0f36d` |
| `/home/cosmosapjw/evidence-20261028T170155Z/suites-a1/local-benchmarks/command_00/RECEIPT.json` | 1590 | `b6aa02e5b15c48f693ba6c9ff67dfae732d36ed3e38e8e1fc83d9afd02f954bc` |
| `/home/cosmosapjw/evidence-20261028T170155Z/suites-a1/local-benchmarks/command_07/RECEIPT.json` | 1785 | `5d59679cbcaa2d8873f42b800cd0daf516f9cb6939a148795ab19f23ba230275` |
| `/home/cosmosapjw/evidence-20261028T170155Z/suites-a1/local-profiles/cubic/SUMMARY.json` | 488 | `e000e9b9a52b6fd1bc16adec0070ac6ee23a2bee335932c9ad25a686c6a1139e` |
| `/home/cosmosapjw/evidence-20261028T170155Z/suites-a1/local-profiles/structural/SUMMARY.json` | 420 | `7d79b6378f2b165c3a7e4a65860a6367f4a9a8d191f9c78367e62180472057ce` |
| `/home/cosmosapjw/evidence-20261028T170155Z/qledger-20261029/full_authority_final.stdout` | 46224 | `63e49b2e637b8c3c073a191bbfb6024e5109e06b835bce1a16d69a6c0dca48f3` |
| `/home/cosmosapjw/evidence-20261028T170155Z/qledger-20261029/full_authority_final.exit` | 2 | `4355a46b19d348dc2f57c046f8ef63d4538ebb936000f3c9ee954a27460dd865` |
| `/home/cosmosapjw/evidence-20261028T170155Z/qledger-repair-20261029/full_authority.stdout` | 46224 | `63e49b2e637b8c3c073a191bbfb6024e5109e06b835bce1a16d69a6c0dca48f3` |
| `/home/cosmosapjw/evidence-20261028T170155Z/qledger-repair-20261029/full_authority.exit` | 2 | `4355a46b19d348dc2f57c046f8ef63d4538ebb936000f3c9ee954a27460dd865` |
| `/home/cosmosapjw/evidence-20261028T170155Z/final-025-clippy-retry/RECEIPT.json` | 1603 | `68a091662264e0c75997dbb66b4be2a5fc23ca5819388d40cecb30222fc8fb65` |
| `/home/cosmosapjw/evidence-20261028T170155Z/vendor-fresh-build/RECEIPT.json` | 1683 | `ab36d109ef6c09b96ef33db00830d94b001a428ba190cf9aab2c09a699f31938` |
| `/home/cosmosapjw/evidence-20261028T170155Z/sundials-mpi-adapters/RECEIPT.json` | 1609 | `7e45dd9bf5a098c79f19a259aa8c2065f266da0616409d8e877083b3cc1b2c7a` |
| `/home/cosmosapjw/evidence-20261028T170155Z/sundials-mpi-adapters/stdout.log` | 502 | `3cd76c14e37e672364600e8f5746bdde36372a80bc2b0f2dd855039fc3a6f309` |
| `/home/cosmosapjw/evidence-20261028T170155Z/EXEC_SOURCE_IDENTITY.json` | 1026 | `077e0e396063412678ce48753c8304cd032f7e1339491cc37da1e3c2908b8a45` |
| `/home/cosmosapjw/evidence-20261028T170155Z/IDENTITY.json` | 1069 | `1ee4b8efacb1014e37cc5949224dc29c4ac5366e166b083a02c359431e6fe378` |

## 재현

Current-source scoped runner (새 external output 경로를 사용):

```bash
python3 research/dynamics_symbolic_validation_20261028/code/run_scoped.py \
  --source-sha 0255297e4f865bb78753cad9fe51d6f27536d188 \
  --remote-ref origin/validation/local-dynamics-exec-20261028T170155Z \
  --output <NEW_EXTERNAL_EVIDENCE_DIR> --cargo cargo
```

Q-API fixtures는 source publication 이후 아래 명령으로 실행한다. 기존 `qapi-final3`은 재사용 가능한 완료 결과이므로 다시 실행할 필요는 없다.

```bash
python3 research/local_dynamics_qualification_20261029/api_fixture/run_fixtures.py \
  --repo "$PWD" --source-sha 0255297e4f865bb78753cad9fe51d6f27536d188 \
  --remote-ref origin/validation/local-dynamics-exec-20261028T170155Z \
  --output <NEW_EXTERNAL_EVIDENCE_DIR>
```

Suite 명령의 정확한 argv, 시작시각, 종료상태, stdout/stderr hashes는 `/home/cosmosapjw/evidence-20261028T170155Z/suites-a1/<suite>/command_*/RECEIPT.json`에 기록됐다. Historical and current executable identities, inputs, reference, results and costs must be joined from `SUMMARY.json`, not mixed across source lanes.

재현 전 전용 고정 실행 ref를 fetch하고 별도 detached worktree에서 확인한다: `git fetch origin validation/local-dynamics-exec-20261028T170155Z`; `git worktree add --detach <NEW_WORKTREE> 0255297e4f865bb78753cad9fe51d6f27536d188`. 새 worktree에서 `HEAD`와 `origin/validation/local-dynamics-exec-20261028T170155Z`가 둘 다 해당 SHA인지 확인한 뒤 위 명령을 실행한다. 결과 branch는 이 ref와 별개다.

### Q-LEDGER 마지막 correction

독립 검토가 NumPy revision을 SciPy repository에 귀속하는 결함을 확인했다. 마지막 3/3 episode에서 의존성 이름별 귀속과 미상 이름 fail-closed를 적용했고 targeted red/green과 전체 41개 회귀를 보존했다. 실제 여섯 경로의 귀속은 분리되지만 full authority는 exit 1이다. [패치와 원시 결과](evidence/qledger-attribution-final/SUMMARY.json). 추가 수정은 같은 예산을 초기화하지 않는다.

### 게시 후 checker source 검증

`c9c734529cf5bbbc33683987006bf71fca3f122d`, tree `32243dae409795171a63210e38a2709b605a4bf6`를 non-force push하고 fetch로 확인했다. `0255297e` 이후 변경은 authority checker와 그 테스트 두 파일뿐이므로 이전 native source와 구분한다. 이 source의 회귀 41개는 exit 0, full authority는 exit 1이다. 해당 full 검사는 비추적 결과 보고서가 존재한 worktree에서 실행되었다. [회귀](evidence/qledger-c9-tests/RECEIPT.json), [authority 영수증](evidence/qledger-c9-authority/RECEIPT.json), [raw 진단](evidence/qledger-c9-authority/stdout.log). 독립 감사는 해당 source·증거에 대해 PASS_SCOPED, broad production에 대해 HOLD로 판정했다.

최종 checker source와 결과 문서가 staged된 worktree에서 `check-research-node --base bf308589...`는 183 ledger rows와 새 노드를 대상으로 exit 0이었다. `--base origin/main`은 기존 40 diagnostics로 exit 1이었다. 각 [scoped 영수증](evidence/final-node-check/RECEIPT.json)과 [main-base 진단](evidence/final-main-node-check/stdout.log)은 별도다. 이 metadata gate는 full authority pass가 아니다.
