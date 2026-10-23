# 로컬 Codex 인계 프롬프트

아래 블록을 복사하고 패키지·저장소 경로만 실제 값으로 바꿔 사용한다.

```text
목표: 현재 개발 중인 VigilODE/BASS/rei_bianchi 코드에 이번 정보기하 구면
연구의 검증된 부분을 순서대로 연결한다. NCP를 사용하지 않는다.

입력:
- PACKAGE=/absolute/path/to/spherical_information_20261023
- REPO=/absolute/path/to/current/vigilode
- 참고 snapshot: 14c74978b93339a79d7228a324fa77552dae548e

먼저 PACKAGE의 RESEARCH_REPORT_KO.md, DECISIONS.json, NEXT_STEPS.json,
analysis/INDEPENDENT_REVIEW_KO.md, REPRODUCE.md를 읽는다.
현재 REPO의 branch/HEAD/status 및 관련 파일을 확인한다. 기존 작업 트리를
reset하거나 다른 branch로 전환하지 않는다. 기존 local 개발 변경을 보존하고
현재 호출 구조에 맞춰 이식한다. 전체 연구를 처음부터 반복하지 않는다.

첫 작업은 이미 존재하는 thomson6_factors의 U,V와 실제 B=V^T U를 사용하는
cached exact collision action이다. continuum eigenvalue 0.1을 임의 FVM
cell-average grid에 대입하지 않는다. singular B와 큰 optical depth를
포함하고 inverse 대신 phi1/block exponential 또는 검증된 대칭 분해를 쓴다.
기존 RHS가 이미 rank6이므로 dense-vs-rank6 matvec 속도비를 개선율로 보고하지 않는다.

두 번째는 full occupation 또는 joint(n,rho) conservative remap이다.
현재 상태가 N(cell contents)인지 f(occupation)인지 확인하고 radial
state-counting weights와 angular area를 명시한다. rho-only thermal 평균은
입자수를 바꾸므로 photon-number 보존 경로에 그대로 넣지 않는다.
topology 변경은 accepted step 뒤에만 수행하고 모든 dimension/cache/error
norm identity를 갱신한다. 사후 observer bound와 실제 goal-tolerance 제어를 구분한다.

세 번째는 entropy moment inversion의 Fisher block 전처리다.
중심화된 radial variance와 triangular whitening factor를 사용하고,
극단적으로 작은 variance/weight를 임의 epsilon으로 성공 처리하지 않는다.
BE score margin, FD domain, 원래 좌표 residual 및 해 오차를 검사한다.
처음에는 기존 Newton/PCG에 연결한다. Newton-free iteration은 보고서의
admissible-ball/전체 susceptibility 조건을 확보한 경우에 별도 시험한다.

각 단계에서 작은 회귀와 물리 불변량 검사를 통과한 뒤 결합한다.
exact collision subflow를 Strang으로 붙이면 전역2차를 기준으로 검증한다.
기존5차 solver의 이름만으로 결합5차를 주장하지 않는다.
coupled matter opacity가 state-dependent이면 scalar optical-depth 적분의
정확성 조건을 다시 확인하고 IMEX/exponential coupling을 설계한다.

마지막 비교는 같은 초기조건, 같은 관측량 오차, 같은 host/thread설정에서 한다.
전체wall time, setup/remap 시간, RHS/선형반복수, 메모리, number/energy drift,
PSD/FD 상태, target-observable error를 함께 기록한다. 실패와 느린 사례도 남긴다.
반환물: source commit/diff, 입력해시, 정확한 명령, 종료코드, JSON원자료,
work-precision 도표, 미해결사항과 바로 이어갈 개발단계.
범용production 승격은 이번 작은 prototype PASS만으로 선언하지 않는다.
```

## 반환할 최소 JSON

```json
{
  "schema": "spherical-information.local-return.v1",
  "source_commit": "ACTUAL_COMMIT",
  "source_tree": "ACTUAL_TREE",
  "input_manifest_sha256": "ACTUAL_SHA256",
  "implemented": [],
  "commands": [],
  "checks": [],
  "work_precision_artifact": null,
  "failures": [],
  "next_actions": [],
  "ncp_used": false
}
```
