# 로컬 Codex: 선택 정책 중심의 정밀 진단과 후속 구현

대상 repo: https://github.com/cosmosapjw-quantum/vigilode
전달 branch: research/adapter-diagnostics-20261025
이 문서가 든 게시 commit을 먼저 고정하라. 최종 ChatGPT 인계에 제공된 전체 SHA를 RESEARCH_COMMIT으로 사용하고, branch tip과 다르면 그 차이를 기록하라. 기존 checkout을 reset하거나 교체하지 말고 새 detached worktree에서 진행하라. NCP는 사용하지 않는다.

## 실행 목표와 현재 상태

장기 method×thread 전수 반복을 재개하지 않는다. 실제 선택된 방법·좌표·실행 backend가 같은 checkpoint의 적격 대안보다 느리거나 부정확한 이유를 조사하고, 근거가 있는 수정으로 개선한다. 이번 게시에는 native pilot, 테스트, 제한된 측정, 선택 평가기가 포함되어 있다. 전체 production 인증은 아직 완료되지 않았다.

기준 stop packet은 d79326648a7273c0decd641eaf94d94f5bea7c63이며, 그 부모는755885522e7e487e5f0d89085cfa15d972df0ea7이다. 이전 보고서는 abf9c5683d810a703d818f8f0c8abec54fbe8850이다. BASS/REI 연구는72836e0290b66d33f4ca1771284b1c8070669a74와 해당 파일의 provenance를 참조했다. 이 브랜치를 통째로 merge하지 않는다. 기존 개발 파일 삭제 등 무관한 차이가 있다.

먼저 README_KO.md → REPORT_KO.md → FINDINGS.json → NEXT_STEPS.json → selection_benchmark/PROTOCOL_KO.md를 읽는다. 정밀한 수학 문제가 생겼을 때만 각 분과 문서 전체를 읽는다.

## 모델·플러그인 라우팅과 토큰 절감

- 자료 목록·해시·receipt 추출, 정해진 build/test, 작은 확정 수정, 로그 요약: 사용 가능한 Codex 6 Luna low 또는 동등한 저비용 경로를 기본으로 한다. 셸/Python으로 계산할 것을 LLM에게 장문으로 반복 입력하지 않는다.
- 구조가 알려진 중간 난도 코드 수정: 가용 workhorse의 low/medium으로 한정한다. 우선 재현 테스트와 수정할 함수만 전달한다.
- 새 증명, 시간 의존/비선형 chart의 물리 오차 수송, 비가환 회전, 불명확한 수치 원인, solver 안정성 논쟁: 해당 한정 문제만 Astra ultra로 승격하고 physmath-research-loop 및 필요한 SciSpace/Exa/Wolfram을 사용한다. plugins/skills를 평범한 로그 처리·빌드마다 재호출하지 않는다.
- 요청한 모델/effort가 실제 로컬 도구에 노출되는지 먼저 확인한다. 없으면 이름을 사용했다고 주장하지 말고 그 정밀 작업만 라우팅 미지원으로 기록한다. 새로운 API 계정·키·유료 계산 자원을 만들지 않는다.
- 고비용 worker에 주는 입력은 질문1개, 관련 수식/함수, source identity, 재현 입력, 대표 결과와 미해결점으로 압축한다. raw 전체·과거 대화 전체를 넘기지 않는다. 한 번의 독립 검토 후 실제 결함을 고치고 해당 시험만 재실행한다.

## 원자료 보존과 최신 상태 조사

1. cwd/branch/HEAD/tree/status와 사용 가능한 CPU·메모리·affinity를 수집한다. 사용자 수정·기존 dirty WIP는 그대로 둔다. stop packet이 기록한 과거 중지 상태를 현재 상태로 가정하지 말고, 이 프로젝트의 실행 PID·실행 잠금만 읽기 전용 확인한다. 재개된 작업이 있으면 임의로 종료하지 않는다.
2. 실제 raw root `/tmp/vigilode-local-run-20261024.geLVCp9Q` 및 continuation_20261025T080718Z를 조사한다. 경로가 사라졌으면 repo와 기존 보존 위치를 탐색하되 새 실행으로 과거 raw를 대체하지 않는다.
3. raw를 장기 보존 위치에 create-only 복사/아카이브한다. 먼저 실제 용량과 가용 공간을 확인하고 원본은 삭제하지 않는다. 기존 보고서 export와 source/binary/reference/receipt 해시를 대조한다. compact Git packet은 원시자료 전체 백업이 아니다.
4. 1539개 역사 작업의 판정을 소급 변경하지 않는다. 새 참조와 새 코드로 판정한 결과는 새로운 cohort로 보존한다. Julia48회, accuracyV3 0회, timeout진단0회라는 기존 상태를 그대로 유지한다.

## 네 가지 정밀 작업

A. 진동 / 구조 활용
- 실제 case15 chirp와 gyromotion의 시간별 진폭·위상·물리 상태 오차를 추출한다. chirp에서는 속도 성분을 버리지 않는다.
- `ExponentialChirpChart`의 모든 출력은 원 변수(x,v)로 복원한다. phase rounding, chart conditioning, 추가 residual의 algebraic charge를 확인한다. constant/variable frequency, nonlinear equivariant/non-equivariant negative controls를 포함한다.
- 과거 PP05/PP06의 noncyclic Fourier convolution과 원-target residual certificate를 재사용한다. FFT를 단순히 샘플링 시간루프의 대체물로 쓰지 않는다. harmonic×formal-degree padding과 conjugate sector 독립 amplitude를 유지한다.
- 다음 이식은 동적 physical-error norm 및 slow residual solver 선택이다. 높은 damping까지 carrier만으로 해결됐다고 판단하지 않는다. 위상/좌표 구조는 선언·검증된 provider로 등록한다.

B. adaptation / 안정성
- case5와 KS의 실제 attempted_h, requested_h, accepted/rejected, embedded norm, 현재t, clipping, callback 비용을 조사한다. 기존 trace가 없으면 짧은 별도 계측 실행을 만든다.
- 이번 평균 h와 DOPRI 안정경계의0.07% 이내 일치는 기작을 좁히는 근거다. 단계별 자료로 검증하고 nonlinear transient의 다른 원인을 분리한다. 작은 h를 무조건 크게 만들지 않는다.
- case5 cofactor 공급자의 항등식·초기조건·primitive와 원 변수 복원을 검증한다. 새 `ExponentialCofactor`는 구조 탐지기가 아니다.
- fused exponential PI/history/clipping 수정은 별도 경로의 구현 결함이다. 기존 DOPRI 캠페인 실패의 원인으로 소급 연결하지 않는다.

C. method switching
- 실제 `GeneralOde`와 새 `AutomaticOde`를 구분한다. 후자는 관측된 method fitness를 선택하며 물리적 stiffness classifier의 완성본이 아니다.
- 유지되는 것은 DOPRI의 다음 h 제안뿐이다. PI/FSAL/workspace 및 implicit controller를 유지하는 세션이 다음 핵심 개발 대상이다. method/model/chart epoch가 바뀌면 해당 이력을 무효화한다.
- rotation pilot은 불필요한 RODAS 상주를 피하지만 manual DOPRI보다 여전히 느리다. 반복 implicit pilot, workspace 재생성, proxy work와 실제 시간의 차이를 구간별로 분해한다.
- 판정은 같은 checkpoint·물리 interval·출력·오차 기준에서 수행한다. 한쪽의 accepted prefix와 다른 쪽의 다른 길이 구간을 몰래 비교하지 않는다. 초기에 관찰한 비용이 나중에도 같다고 가정하지 않는다.
- callback이 pure/replay-safe인 경우만 shadow를 실행한다. BASS/REI의 effects journal/ACK를 우회하지 않는다. 모든 pilot 비용을 청구한다.

D. parallel adapter
- 같은 적분법과 같은 출력에서 serial과 유망한 병렬 후보1개만 먼저 비교한다. 모델/RHS/workload/host epoch에 맞게 calibration을 무효화한다.
- 실제 물리 답과 solver 종료 상태가 같아야 비용 후보로 인정한다. nested layer에서 각 단계가 다시 스레드 풀을 만들지 않는다.
- single-item·싼 RHS는 serial 우선이다. calibration/warmup/pool 준비 비용과 재사용 horizon을 포함한다. 보정 때의 outlier·cold cache가 선택을 왜곡하는지 held-out 구간으로 확인한다.
- ensemble pilot 결과를 stage/data 병렬의 성능 증거로 확대하지 않는다. 다음에는 큰 격자 callback의 coarse chunk 및 persistent pool을 같은 방식으로 연결한다.

## 실행 순서·성공 조건

첫 라운드는 raw분석+대표5~6문제의 짧은 진단으로 제한한다. 각 원인에 대해 baseline → 한 가지 수정 → 같은 checkpoint 대안 비교를 수행하며, 한 작업의 수정 루프는 최대3회로 한다. 장기 전수 campaign은 이 작업에 포함하지 않는다.

- 신규 branch/worktree에서 Cargo.lock을 유지해 필요한 native 계약검사와 이번 예제를 빌드한다. 기존 환경의 offline cache가 부족하면 정확한 missing dependency를 기록하고 이미 승인된 개발환경의 잠긴 의존성을 준비한다. lock을 바꾸어 우회하지 않는다.
- `selection_benchmark/capture_run_metadata.py`로 실제 source/build/binary/host identity를 수집한다. 스크립트는 이전 binary가 현재 소스로 만들어졌다는 사실을 증명하지 않으므로 actual build receipt와 연결한다.
- `selection_window.schema.json` 형식으로 실제 결정 시점과 대안1개의 paired replay를 기록한다. `analyze_selection.py`로 실패·정확도·선택 비용비를 계산한다. 지금의 whole-trajectory pilot을 paired-window 자료인 것처럼 변환하지 않는다.
- 정확도는 원 물리 상태·동일 출력 요구에서 평가하고, 가능하면 시간 grid 및 위상 관측도 추가한다. 빠르지만 기준을 넘는 방법은 적격 대안이 아니다.
- 수치적 불확실성과 엄밀한 오차 상한을 분리한다. 성능은 같은 정확도 상한 비교인지, 달성 오차까지 맞춘 비교인지 명시한다.
- 선택이 틀린 사례를 숨기거나 단지 selector 임계값을 맞춰 훈련한 문제를 재평가하지 않는다. 조정에 쓰지 않은 구간/매개변수에서 검증한다.

반환물: 실행source/tree/binary/host, raw보존 위치와 해시, 실제 명령·exit, 대표 trace, 원인별 재현 및 패치, 선택별 적격성/비용/실패분류, 테스트·짧은 벤치마크, 아직 못한 항목과 다음 최소 작업. 코드와 작은 증거를 push하되 대용량 raw는 장기 보존 위치에 유지한다. 중지된 이전 캠페인을 자동 재개하지 않는다.
