# Native DVODE / diffsol 비교 후속 작업

기준 VigilODE 소스는 `aaf1f834d4c589e734ca7851a56c11bb44f7047a` / tree
`03b12c682c688155e0f8b08c52508e04738acb93`의 별도 깨끗한 worktree다.
이 디렉터리의 코드는 별도 후속 브랜치에 두고, 실행 출력과 Cargo target은
checkout 밖의 새 RUN_ROOT에 둔다.

`native/native_dvode.cpp`는 Netlib `vode.f`의 `DVODE` 진입점을 C++ 콜백으로
직접 호출한다. `adams`는 MF=10, `bdf`는 MF=22(내부 유한차분 Jacobian)이다.
`vode.f`와 LINPACK `dgefa/dgesl/dgbfa/dgbsl` 원본은 결과 디렉터리에서
별도 내려받아 SHA-256으로 고정한다. timed 구간에는 Python 콜백이 없다.

`compare.py`는 기준 바이너리의 네 모델(1, 10, 15, 16) 설명자와 RHS probe를
검증하고 각 native 입력을 만든다. 한 물리 코어에서 VigilODE 세 방법,
native DVODE 두 방법, diffsol BDF를 회전 순서로 warmup 1회 + timed 5회씩
실행한다. 모든 프로세스의 stdout/stderr, 종료 코드, resource, timeout 및
원래 잔차·endpoint 오차 판정을 독립 파일로 남긴다. Robertson(case 1)은
독립 endpoint 참조가 없어 속도 순위에서 제외한다. 나머지 세 모델의
공통 endpoint 절대 오차 상한은 `1e-4`다. 이 비교는 장시간 위상이나
broad production 승인 근거가 아니다. 단일 프로세스 시간은 진단용이다.

예시(실제 경로·hash·명령·종료 코드는 RUN_ROOT 영수증 참조):

```bash
python3 -B research/local_frontier_native_diffsol_20261024/compare.py \
  --frozen "$FROZEN_CHECKOUT" --vigilode "$VIGILODE_BINARY" \
  --native "$NATIVE_BINARY" --diffsol "$DIFFSOL_BINARY" \
  --netlib-source-dir "$RUN_ROOT/native_diffsol_followup/source" \
  --output "$RUN_ROOT/native_diffsol_followup/standard" --size standard
```

native DVODE와 diffsol의 실행 내부 시간은 각 바이너리에서 계측한다.
Python은 프로세스 시작과 결과 검증에만 쓰인다. 시작/종료 및 setup 비용을
포함하는 프로세스 시간은 별도로 보존한다. 비교 대상별 setup 범위는
완전히 동일하지 않으므로 시간 배수는 이 네 문제의 진단 수치로만 해석한다.
