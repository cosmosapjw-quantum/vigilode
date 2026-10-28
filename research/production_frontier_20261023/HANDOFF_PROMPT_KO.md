# 로컬 Codex 인계 프롬프트

아래 블록 전체를 복사하면 된다. 공개 브랜치의 실제 최신 tip을 확인하도록 작성했으므로, 이전 로컬 checkout이나 측정용 local commit에 맞추어 reset할 필요가 없다. 상세 작업 계약은 같은 폴더의 `NEXT_STEPS.json`에 있다.

```text
너는 VigilODE의 후속 구현·검증 담당 Codex다. 저예산 모드에서 이미 증명·구현·측정된 연구를 재사용하고, 남은 production 병목만 구체적인 코드 변경과 제한된 검증으로 처리하라. 기존 자료를 처음부터 다시 연구하거나 전체 캠페인을 무조건 재실행하지 마라.

대상 저장소:
https://github.com/cosmosapjw-quantum/vigilode

시작 브랜치:
research/production-frontier-20261023

반드시 포함되어야 할 공개 조상:
- native crate 구현: df5c6c56da0eb93b58147d6630a0f094cf4af212
- 최종 standalone PDE 수정: 37817308c12384f33ecb9b8ff64509f971eb3b2b

최종 보고서 commit은 이 프롬프트에 고정하지 않았다. fetch한 브랜치 tip과 tree를 직접 기록하고 위 조상 및 보고서 파일을 확인하라. native 측정의 local55ad7ec6…와 공개df5c6c56…는 PUBLISH_MAP.json의 동일 tree 매핑을 사용한다. local hash를 원격 조상이라고 가정하지 마라.

1. 기존 checkout을 보존하고 시작점을 고정하라.

기존 작업 트리가 다른 브랜치이거나 dirty여도 reset/checkout 변경/stash/강제 삭제하지 마라. 저장소 안에서 origin이 위 저장소인지 먼저 확인하고, 새 worktree를 만들어 작업하라. 예시 Bash 명령은 다음과 같다. 명령 실패 시 성공한 것으로 넘어가지 마라.

  set -eu
  git remote get-url origin
  git fetch --no-tags origin research/production-frontier-20261023
  frontier_tip=$(git rev-parse FETCH_HEAD)
  frontier_root=$(mktemp -d "${TMPDIR:-/tmp}/vigilode-frontier.XXXXXXXX")
  git worktree add --detach "$frontier_root/checkout" "$frontier_tip"
  cd "$frontier_root/checkout"
  git merge-base --is-ancestor df5c6c56da0eb93b58147d6630a0f094cf4af212 HEAD
  git merge-base --is-ancestor 37817308c12384f33ecb9b8ff64509f971eb3b2b HEAD
  test -f research/production_frontier_20261023/REPORT_KO.md
  test -f research/production_frontier_20261023/NEXT_STEPS.json
  python3 research/production_frontier_20261023/src/verify_package.py
  git rev-parse HEAD
  git rev-parse 'HEAD^{tree}'
  git status --porcelain=v1

shallow history 때문에 조상 확인이 안 되면 해당 브랜치의 필요한 이력만 추가 fetch하여 재확인하라. 계속 확인할 수 없으면 identity 문제를 기록하고 그 checkout에서 native 실행을 시작하지 마라. 현재 사용자의 원래 checkout을 맞추려고 바꾸지 마라.

코드를 수정할 경우 이 고정 tip에서 충돌하지 않는 별도 follow-up 작업 브랜치를 만들고 이름을 기록하라. 검증만 수행할 경우 detached worktree를 유지해도 된다. 기존 공개 연구 브랜치나 main을 강제로 이동하지 마라.

2. 먼저 읽을 자료와 현재 판정.

아래는 모두 research/production_frontier_20261023/ 아래의 파일이다.
- REPORT_KO.md
- NEXT_STEPS.json: 19개 작업의 의존성, 파일, 계약, 시험, 예산, 완료·중단 조건
- DECISIONS.json, THEOREMS.json, RESEARCH_TREE.json
- PUBLISH_MAP.json, REPRODUCE.md
- BENCHMARK_PLAN.json: 18개 문제군과 과거40개 개발 항목의 대응
- analysis/ADVERSARIAL_FINDINGS.json 및 analysis/PRODUCTION_VALIDATION.md
- 담당 작업에 해당하는 analysis/SWITCHING, AFFINE_QUOTIENT, CAUSAL_TRAJECTORY, POLYNOMIAL, PDE_SCALABILITY 문서

기본 판정은 broad production HOLD이다. 새 native 회귀59개와 standalone PDE9개 등의 통과는 전체 workspace 또는 외부 비교의 qualification 완료를 뜻하지 않는다. 현재 결과를 재사용하라:
- switching: 120측정 중114완료,105행이 공통 표본 오차 상한 통과. 6실패는 RHS-only Prothero 시도 예산 소진이다.
- rapid JVP: 심사 전환이 기존 JVP 대비 중앙값9.46배 개선. 반면 rapid FD는 공통 오차 예산에서2.03배, 강성 전이 FD는3.05배 느렸다. 초기 pilot 하나가 이후 강성 전이를 놓쳤다.
- causal: 210궤적 완료,195개 공통 오차 상한 통과. 넓은 스펙트럼은33/35회 fallback으로 속도 이득이 사실상 없다. 짧은20radian 시험은 장시간 위상 시험이 아니다.
- 1/2thread 비교의84쌍 출력은 bitwise 일치했지만,42설정 중1.2배 이상의 two-thread 이득은 없었다.
- affine/harmonic은 선언된 고정 carrier와 외력의 특화 경로다. 일반 시간 외력과 느린 carrier의 손실도 남아 있다.
- RKL2는 가중 안정성과 질량 보존에도 불구하고 비음 초기값에서-5/512를 만든다. 양성 보장 경로로 무조건 선택하지 마라.
- 온라인 terminal event, 즉시 native fatal 종료, 지속되는 허용 세션, 장시간 위상 및 일반 비선형 PDE는 미완성이다.

원시 JSON은 RAW.tar.gz 등으로 묶여 있을 수 있다. REPRODUCE/manifest가 지정한 archive를 읽고, 개별 loose 파일이 없다는 이유로 자료가 누락됐다고 판단하지 마라. 필요하면 scratch에 선택적으로 읽되 저장소에 전체 raw를 다시 풀지 마라. 이미 발행된 원시 실패·manifest는 덮어쓰지 마라.

3. 자원·재사용 규칙.

- NCP, SSH, cloud 자원은 사용하지 않는다. 과거 external_audits의 NCP setup/launch 스크립트도 실행하지 않는다.
- 설치된 Rust1.94.1과 제공된 vendor/cache를 먼저 찾는다. cargo metadata --locked --offline으로 준비 상태를 확인한다. package 누락은 환경 실패로 기록하며 과학적 실패나 통과로 바꾸지 않는다.
- 빌드 jobs<=2, 동시 Cargo 빌드1개. 코드 작업은 파일 소유권이 분리된 최대2작업만 병행한다. lib.rs 및 공통 driver 연결은 한 담당자가 통합한다.
- 기본 solver thread1, 선택적2thread. 시간 측정은 동시에1개만 실행한다. 측정 중 소스 수정·컴파일·다른 벤치마크를 돌리지 않는다.
- solver RSS<=1GiB, timed process<=90초, 최종 qualification 전체<=5400초/500프로세스. 빌드1회는 별도1800초 이내에서 중단·기록한다.
- NEXT_STEPS의 작업별 timebox는 상한이지 완료 약속이 아니다. 여러 작업의 예산을 합쳐 전체 campaign 한도를 임의로 늘리지 마라.
- 변경된 소스/가정/지원 기능/출력 계약/host qualification이 있을 때만 관련 시험·캠페인을 다시 실행한다. 변경되지 않은 증명과 기존 campaign은 파일·hash·범위를 인용해 REUSED로 둔다.
- 같은 허용오차 입력은 같은 실제 오차가 아니다. 동일 관측 시각과 물리 norm의 달성 오차를 먼저 확인하고 비교하라.
- 실패, timeout, unsupported, missing dependency, error-cap 실패를 원시 기록에 남긴다. 실패한 시도·버린 pilot·fallback·setup을 비용에서 빼지 마라.

4. 모든 병목을 계획에 포함하고, 의존성 순서로 구현하라.

모든 작업 ID에 상태와 근거를 남겨라. 한 노드만 수정한 뒤 전체 요청을 완료했다고 보고하지 마라. 독립 작업은 진행하되, 예산이 끝나면 정확한 중간 상태와 다음 시작점을 저장한다. 상세 조건은 NEXT_STEPS.json을 따른다.

우선순위 A — 안전한 세션 기반:
- PR00: 공개 source/tree/lock/compiler/binary identity 및 기존 근거 확인.
- FA01: fatal과 recoverable을 native에서 구분. fatal 사용자 callback1회, 이후 native trial0회, 마지막 수락 상태·작업 기록 보존. 현재 Guard가 둘 다 NonFinite로 바꾸어 추가 short-circuit 시도를 만드는 문제를 해결한다. 단순히 Err를 조기 반환해 부분 결과를 잃는 변경은 금지한다.
- SW02: DOPRI5와 RODAS의 accepted-checkpoint/advance_to 세션. FSAL·다음 h·controller·유효한 factor/preconditioner를 보존하고 model/provider epoch·method·dimension·norm·h에 따라 캐시를 무효화한다. 바뀌지 않은 단일 방법의 상태/출력/카운터 일치를 먼저 검사한다.
- EV01: 수락 스텝 안에서 event를 검사·localize하고, terminal 검출 이후 추가 물리 trial을 실행하지 않는다. 검출 전 해당 trial의 stage가 사건을 넘을 수 있다는 점은 별개다. 정의역 벽을 넘는 stage 자체를 막으려면 client의 safe-domain bracket/step cap이 필요하며 없는 경우 보장하지 않는다.
- GE01: time residual, spatial defect, approximate flow/root, remap consistency, roundoff를 분리한 예산 ledger. 검증되지 않은 표본 residual을 certified bound라 부르지 않는다. G로 나누기 전0<G<infinity를 검사하고 remap/flow jump 순서를 명시한다.

우선순위 B — 실제 손실을 줄이는 선택·구조 연결:
- SW03: 지속 세션 위에 epoch/비용 추세 변화에 따른 제한된 재탐색. 현재 one-pilot, epoch-trigger, trend-trigger의 최대3후보만 비교한다. 이후 강성 전이를 놓치는 기존 반례와 위치가 다른 독립 전이 사례를 함께 사용한다. 총 탐색 예산은 유한해야 한다.
- DV01: 미분 제공자 허용과 성능 근거를 분리한다. Forbidden은 완전 RHS-only, ProvidedOnly는 숨은FD/dfdt 금지. 실제 user callback 횟수와 native build 요청을 따로 기록한다. 합성 작업 지표를 벽시계 시간이라고 부르지 않는다.
- QF01: 기존 constant/harmonic phi 작용은 재사용한다. shifted coefficient cache를 epoch와 간격에 묶고, 매끄러운 비선형 잔여항의 stage-consistent wrapper는 순서 조건을 유도한 후 구현한다. resonance/zero/subnormal 회귀와 generic-forcing 손실을 유지한다. 외력을 단순히 동결해서5차라고 주장하지 않는다.
- QA01: B=sA 관계식과 witness를 s/operator/norm/epoch/dimension에 묶는다. 전 공간 근거와 원래 잔차 검사는 유지한다. d-1후보작용+1잔차작용=d회다. 실제 local-collision block부터 연결하고 일반 격자 Laplacian의 낮은 최소다항식 차수를 가정하지 않는다.
- CF01: causal root의 조건부 inverse/Lipschitz/전달 안정도와 trajectory budget을 연결한다. 원공간 잔차·embedded 기준을 유지하고 workspace를 재사용한다. 넓은 스펙트럼 fallback 비용을 줄일 후보도 실제 수락 궤적으로 비교하라. frozen-base JVP가 남는 경로를 완전 derivative-free라 부르지 마라.
- CP01: 기존 작은 문제에는 serial forward를 유지한다. 실제 비싼 RHS가 확보된 경우에만1/2thread의 batching 교차점을 새로 시험한다. gain이 없으면 해당 범위에서 prune하고 그 결과를 완료된 연구 판단으로 남긴다.

우선순위 C — 실제 격자·물리 지원 범위:
- SG01: weighted conservative graph flow를 transactional API로 이식한다. 비음 요구 시 RKL 후보를 그대로 commit하지 말고 검증된 positive 정책이나 typed refusal을 사용한다. setup과 hot-loop 할당은 별도로 측정한다.
- PDE23: 2D/3D·구면 manufactured reaction-diffusion 및 transport-collision부터 실제 수락 궤적으로 검증한다. BASS/rei_bianchi의 실제 변수·경계·광자수·에너지 정의가 있어야 물리 client 적용을 주장한다. actual client가 없으면 BLOCKED_CLIENT_DATA로 두고 manufactured 범위만 진행한다.
- PDE-POLAR: full RKL, longitude-exact split, 명시된 보존 격자 변경의 최대3후보. 더 높은 경도 모드·극점 집중 자료·이방성에서 공간/시간/splitting 오차를 따로 측정한다. FFT쌍과 stencil작용을 같은 연산량으로 합치지 마라.
- PDE-FLOAT: 이미 고친Q=0/t=0 identity 외에 비영 연산자의 극단 진폭 recurrence를 검사한다. finite 참결과에 대해 finite 출력 또는 typed range refusal을 제공하고, 실패 시 상태를 보존한다.
- AMR01: accepted-step 사이에서만 remap한다. 상태/격자/캐시 epoch를 한 번에 commit하고 물리적 공동 불변량, consistency defect, 누적 오차 예산을 검사한다. 보존만으로 remap오차0 또는 자동5차를 선언하지 마라.

우선순위 D — 범위별 qualification:
- LP01:100/1000cycles 위상·진폭 시험.4097관측 또는 충분한 analytic branch tracking을 사용한다. 진폭 영점 mask를 기록하고 phase<=1e-3rad, 상대 amplitude<=1e-5를 제안 기준으로 검사한다. 이는 기존20radian 시험의 재실행이 아닌 새 범위다.
- CAP01: mass/autonomy/tolerance/dense/event/restart/dimension 조합의 실제 typed capability표. unsupported는 callback 전에 거절한다. dense interior 오차와 landing 오차를 구분한다.
- PB01: 위 의존성의 구현 또는 명시된 좁은 범위 결정을 확인한 뒤 BENCHMARK_PLAN의18개 문제군에서 제한된 subset을 실행한다. 미해결 필수 기능이 있으면 broad HOLD를 유지하고 검증된 좁은 범위만 판정한다.

5. 검증·비교의 구체적 기준.

개별 변경은 해당 crate의 의미 있는 계약 시험부터 실행한다. 전체 workspace를 매번 다시 돌리지 마라. 새 테스트 이름/파일은 NEXT_STEPS에 proposed_files로 적힌 계획이며 아직 존재한다고 가정하지 마라. 변경 후 focused validation 명령과 exit code를 보존하고 한 번 통과한 불변 영역을 반복하지 마라.

타이밍 전에 모든 코드를 고정하고 실제 commit/tree/relevant-source/binary hash를 기록하라. common observation error cap, 출력 계약, reference uncertainty<=cap의10%를 적용한다. 새 qualification은 warmup1회+timed5회, 실행 순서 순환, median/min/max와 최악 손실을 보고한다. 변경 전후의 같은-host 비교군이 없다면 속도 배수를 만들지 마라.

외부 비교 대상은 VODE/DVODE, diffsol, Julia만 사용한다. 비교 solver/version/build/RHS/norm/output/precision/thread를 고정한다. Julia JIT와 steady solve를 분리하고 양쪽 setup 포함 비용도 기록한다. 없으면 NOT_RUN으로 남기며 다른 solver나 과거 NCP 숫자로 대체하지 마라. 기존 archive 아래의 local adapter 코드는 읽고 재사용할 수 있으나 cloud launcher는 실행하지 않는다.

PDE는 temporal/spatial/splitting/remap refinement를 각각 최소3수준으로 분리한다. 질량 오차 기준은 상대1e-11+절대1e-13, 양성 요구 경로의 음수 allowance는 명시된 scaled roundoff1e-13이다. Fourier center sample과 cell average를 혼동하지 마라. 평활 점근 구간의 관측 차수만 formal order와 비교하고 limiter/강성 order reduction/AMR은 별도로 보고한다.

최종 campaign 한도5400초는 contract smoke600초, accuracy/scaling1800초, 외부 비교2400초, read-only replay/summary600초로 나눈다. 단계가 준비되지 않았으면 그 구간을 NOT_RUN으로 남기고 근거 없는 promotion을 하지 마라. 이 한도는 전체19개 코드 개발이90분에 끝난다는 약속이 아니다. 구현은 bounded session별로 checkpoint하여 이어간다.

6. 전달·발행 산출물.

새 follow-up 디렉터리에 다음을 남겨라. 기존 research/production_frontier_20261023의 발행 원시 근거를 덮어쓰지 마라.
- source identity, 환경, 정확한 command/exit/stdout/stderr, 불변 raw 실패와 hash manifest
- 19개 ID 전체의 실행/재사용/차단/미실행 판정 JSON
- 바뀐 코드·가정·지원 범위와 관련 시험의 대응
- 실제 오차, 모든 시도 비용, 시간 변동·최악 손실, reference uncertainty
- production-qualified-scope / specialized-only / optimization-possible / hold / pruned-in-cohort의 범위별 판정
- 짧은 한국어 보고서와 다음 재개 지점

중요한 correctness 실패는 해당 route의 승격을 막지만 독립 노드의 읽기·구현까지 무조건 중단하지 않는다. 모든 노드를 검토·분류하기 전에 한 노드만 고쳤다는 결과로 전체 작업을 완료했다고 하지 마라. 코드가 검증되면 별도 follow-up 작업 브랜치에 commit/push하되 force push나 main merge는 하지 마라. 검증 전용 실행이라면 receipt-only 결과도 허용한다. 현재 source를 임의로 최신 코드와 섞지 말고, 동시 원격 변경이 발견되면 새 identity로 구분한 후 영향을 받은 검증만 다시 수행하라.

완료 보고에서는 '컴파일 통과', '계약 시험 통과', '표본 정확도 통과', '범위별 성능 개선', '범용 production 승인'을 서로 구분하라. 현재 broad HOLD를 바꾸려면 미해결 필수 계약과 독립 비교 의무가 실제로 닫혀야 한다.
```
