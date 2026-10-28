# Native 재감사: 실제 완료한 작업과 검증 기록

## 1. 복구된 authority

기존 PR #70 (`audit/rvj-native-followup-20261003`)을 계승했으며 중복 PR은 만들지 않았다. 검토 base는 `5a8d7fe9ffc681bca98a98a2f9889a2a05505783`, 이미 게시된 PR source는 `77275e2b431e4280366060d44a883f1832909fe6`다.

read-only source export artifact 11247252078에서 원 base의 Git bundle과 해당 Cargo.lock의 vendor를 복구했다. ZIP SHA-256 및 내부 SHA256SUMS의 두 파일을 확인했다. 로컬에서 PR diff의 14개 파일을 복원한 뒤 전체 tree를 계산했으며 `20d57400028c7e6a595020d810977e07bcdb0876`으로 원격 PR tree와 정확히 일치했다. 단순히 보고서나 일부 source snippet만 계승한 것이 아니다. 이 이후의 보완은 별도 local overlay이며 게시된 PR source와 혼동하지 않는다.

첨부 standalone Rust를 `/mnt/data/rust-1.94.1-prefix`에 설치했다. 관찰한 rustc는 1.94.1, commit `e408947bfd200af42db322daf0fadfe7e26d3bd1`, cargo는1.94.1이다. user installer의 byte hash를 기록했지만 독립 trusted public key에 대한 asc signature verification은 하지 않았다. RUSTCORE.zip은 다른 프로젝트의 dependency set이므로 이 저장소의 vendor 대체로 사용하지 않았다.

native commands는 Cargo `--frozen`과 repo lock에 맞는 vendor를 사용했다. 원 repo의 Cargo.toml, Cargo.lock, dependency versions는 변경하지 않았다. Cargo 공식 문서에서 `--frozen = --locked + --offline`을 확인했다.

## 2. 게시된 PR 코드의 RED/GREEN 재현

수정 전 base에 회귀시험만 추가하여 native RED를 재실행하고, 이어서 실제 PR source를 복원해 같은 테스트를 실행했다. 아래 결과는 과거 실행 보고의 계승이 아니라 이번 runtime에서 직접 관찰한 것이다.

| 대상 | base RED | 복원한 PR GREEN | 분류 |
|---|---:|---:|---|
| reversed Laguerre interval/cache | 1 pass, 2 fail | 3 pass | 입력 검증 결함 |
| frozen MF callback/state/time와 failed refresh | 1 pass, 7 fail | 8 pass | stale operator 및 non-atomic refresh, public contract 우회 |
| negative absolute radius coefficient | 2 pass, 1 fail | 3 pass | 잘못된 state map에 의한 false closure |
| Python chart nonfinite/overflow domain | 1 pass, 2 fail | 3 pass | 입력·domain 검증 결함 |
| 새 native reconstruction contract | 대응 legacy API 없음 | 5 pass | 조건부 연구 interface, 기존 solver 교체 아님 |

캐시 수정은 `fresh=false`라는 힌트가 다른 t/y/callback을 강제로 재사용하지 못하도록 하고, fallible RHS 또는 f_t 갱신에 앞서 오래된 operator를 무효화한다. 같은 객체를 clone하고 h만 바꾸는 정상 rejected-step reuse는 유지한다. callback의 interior mutable data까지 자동 검출한다는 뜻은 아니며 manual fresh 계약은 남는다.

Laguerre 수정은 `[lo, hi]`의 순서가 뒤집힌 입력을 Bernstein 연산 중 넓어진 유효 interval로 받아들이지 않게 한다. 잘못된 입력이 cache에 들어가지 않는지도 검사했다.

## 3. 복구 후 추가로 수정한 결함

게시된 PR source에서도 `AffineMajorant`의 public backing arrays를 constructor 이후 바꾸면 indexed access가 panic을 낼 수 있었고, diagonal/upper entries를 추가해도 무시되는 경우가 있었다. 이것은 sign validation과 다른 구조 결함이다.

새 회귀시험 3개를 먼저 실행했으며 **0 pass, 3 fail**을 관찰했다. 두 시험은 실제 index-out-of-bounds panic, 한 시험은 non-strict-lower data의 잘못된 acceptance를 검출했다. 이후 `MajorantEntries::validate_storage` default hook을 추가하고 built-in AffineMajorant에서 H0,H1,alpha_abs,seed의 shape/sign/strict-lower 조건을 재검증하도록 했다. 같은 3개를 포함한 radius suite는 **6/6 pass**였다.

추가 product 변경은 `causal_majorant.rs`의15줄이며, 새로운 regression 부분은 radius contract file의67줄이다. custom trait 구현자는 선언한 dimensions에 대해 total/deterministic accessors를 제공해야 한다. 임의의 불량 외부 trait 구현까지 Rust type system으로 배제했다고 주장하지 않는다.

## 4. 수학/코딩 연구 결과

특정 Darboux chart `y=x²(w+1/kappa)`의 conditional physical-error transport를 native outward arithmetic으로 확인했다. exact coordinate errors의 공급원, 전체 step의 ODE local/global error, nonzero initial layer의 resolution을 자동으로 제공하는 기능은 아니다. 전체 error box가 regular chart에 남아야 하며 실제 저장된 y의 reconstruction rounding도 포함한다.

정확한 전개 및 quotient-rule 검사는 `DERIVATIONS_KO.md`와 `verify_algebra.py`에 있다. 4개 exact symbolic checks가 통과했다. 기존 RVJ counterexample를 RODAS5P의 결함으로 바꾸어 보고하거나, 기존 method의 negative controls를 제거하지 않았다.

## 5. 검증의 범위

최종 source의 집중 native 회귀와 인접 기능은 12개 test binaries로 실행했다. **37 pass, 0 fail, 2 ignore**이며, 이 중 새 PR/보완 regression은22개다. 나머지는 raw-stage target, radius/action, method-labelled negative controls, matrix-free problem, GCRO-DR state validity와 workspace 계약이다. 두 ignored fixture writers는 실행하지 않았으며 PASS에 포함하지 않았다. direct test binary의 SHA-256과 정확한 명령을 `evidence/related_tests.json`에 남겼다.

`cargo test --workspace --all-targets --all-features --no-fail-fast --frozen`도 시도했다. 그 전체 상태는 `FINAL_VERIFICATION.json`의 `full_workspace`와 로그를 확인한다. 전체 명령이 timeout으로 끝났다면 중간 harness의 PASS를 전체 workspace PASS로 승격하지 않는다. `cargo clippy` 및 fmt, Python tool tests도 각각 독립적으로 기록한다. 여러 실행에 중복되는 테스트 수를 더해서 독립 검증 개수로 광고하지 않는다.

측정용 profile의 ignored MF performance study, 새 matching-error campaign, held-out data, wall-time promotion은 이번에 실행하지 않았다. L-0038 FAIL, 과거 ledger, coefficient fixture, timing HOLD는 유지한다.

## 6. 원격과 로컬의 차이

중단 이전 코드와 chart transport는 PR #70의77275e2에 이미 게시되어 있었다. 이번 runtime은 이를 검증하고 추가 구조 guard 및 보고서를 만들었다. 현재 노출된 GitHub connector에는 read functions만 있고 `create` discovery도 결과가 없었다. direct GitHub 연결은 DNS resolution failure였으며 gh/standard GitHub token도 없다. 따라서 이 보완분의 **새 remote push/PR body update는 미완료**다. 사용자 승인이 없어서 막혔다고 분류하지 않는다.

기존 exact-head CI의 `action_required`를 통과로 취급하지 않는다. 별도 CodeRabbit CLI도 설치되지 않았고 설치 URL DNS가 실패하여 실행하지 않았다. 여기의 audit을 독립 CodeRabbit review로 표시하지 않는다.

전달한 incremental patch는77275e2의 tree 위에 적용한다. 전체 patch는5a8d7fe9 base에서 PR 코드를 함께 복구하는 용도다. 다른 PR/branch에 무조건 적용하거나 force push하면 안 된다. 실제 remote 상태 및 artifact hash는 별도 delivery receipt에서 확인한다.

## 최종 검증 및 동시 변경 확인

전체 all-target/all-feature 테스트는 컴파일을 완료한 뒤 총600초 budget에서 종료했다(exit124). 완료된50개 harness의 요약은269 pass,0 fail,3 ignore이지만 전체 PASS가 아니다. 집중 replay script는37 native pass,2 ignore,3 Python pass,4 symbolic checks 및 Clippy를 포함해 exit0으로 완료했다. 별도로 전체 Python tools211/211, 전체 workspace/all-target/all-feature Clippy `-D warnings`, fmt가 통과했다.

첫 Clippy 호출은 global config가 external subcommand에 적용되지 않아 hex를 찾지 못했고, isolated Cargo home 및 subcommand 뒤의 config 인자로 해결했다. --frozen을 해제하거나 dependencies를 바꾼 것이 아니다. 첫20초 container replay 호출의 종료도 보존하고,180초 bounded replay에서 exit0을 확인했다.

원격을 마지막으로 다시 읽었을 때 PR head는 `312aa57ada821f383f2a8b4da7b2800e4d227d54`였으며77275e2 이후에는 read-only export workflow만 바뀌었다. 이 변경을 그대로 보존한 tree `fb2bc19fccdf34153b86b1dc69fabc4404a0e478`도 로컬에서 일치했다. 이 HEAD의 CI는 관찰시6개 success,1개 in_progress였다. 더 오래된77275e2의 action_required 상태와 구분하며, 이 CI를 로컬 보완의 CI로 취급하지 않는다. 전달 incremental patch의 기준은312aa57이다.
