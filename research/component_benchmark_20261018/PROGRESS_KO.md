# 전체 구성요소 benchmark 진행 기록

- 승인 source: 26f0b3a187ab41f7d68922491cc489b268c0caa1; 원격 fetch 확인.
- 기존 /tmp/vigilode-all-dag-validation-20261018 checkout을 사용한다.
- /home/cosmosapjw/vigilode의 audit 브랜치 미커밋 작업을 그대로 보존한다.
- 계획과 과학적 계약은 이전 Astra ultra 결과와 사용자 승인으로 동결되었다.
- 현재 단계: P0 구현 및 실행 환경 준비. NCP 캠페인 lease는 아직 시작하지 않았다.
- 기존 NCP host에 read-only 접속 성공: 64 vCPU, 약126 GiB RAM. 다른 REI
  positive_native/refinement 작업이 실행 중이다. 해당 작업을 변경하지 않는다.
- 초기 경로 탐색에서 tools/benchmarks/diffsol/Cargo.toml이 없음을 확인했다.
  기존 standalone harness의 실제 build 경로를 읽어 사용한다. 수치 실패가 아니다.
- 첫 구현 분담: 공통 runner/registry; corpus/reference/Python/Julia; Rust/Diffsol.
- 다음: 세 구현을 통합하고 표적 검증한 후 EXEC_SHA를 고정하여 캠페인 실행.

## 구현·환경 준비 checkpoint
- 원본 A01–A12 identity 검사 exit 0: 414 source files, 12 nodes.
- Runner 표적 검사 16개 통과; native/Diffsol 공통 어댑터와 34개 모델 probe 기록 생성. 수치 parity의 최종 허용 규칙은 별도 계약 addendum으로 동결한다.
- 4개 workspace library test 실행 파일을 measurement profile로 컴파일(exit 0); 아직 NCP에서 실행하지 않았다.
- NCP P0 lease와 누적 원장 시작. 기존 REI 작업 보존, 성능 측정 미시작.
- Julia 전송의 mutable compiled cache 변경으로 tar exit 1: 환경 포장 실패. 원시 로그를 보존하고 compiled/logs를 제외한 한 번의 진단 재시도 exit 0.
- Python NumPy2.4.2/SciPy1.17.0/mpmath1.3.0 전용 NCP venv 설치 exit 0.
- Hairer 원본은 기존 build.sh의 pinned Assimulo3.0 SHA로 검증 후 별도 shared library 컴파일 exit 0.

## HPC 실행 준비 checkpoint
- 246개 구성요소(152 module +94 알고리즘 변형), 78 CaseSpec, 47 solver/config arm 등록.
- CPU affinity/cgroup/memory 기반 최대56 worker, P3 고정 lane/barrier 구현.
- 통합 harness 검사83개와 마지막 변경 검사23개 통과.
- Native measurement/Diffsol release 및 Rust component test 31개 바이너리 NCP 전송 exit0.
- 독립 Astra 검토: 순수 사전등록 소스 checkpoint 게시 PASS; 과학적 승격 없음.
