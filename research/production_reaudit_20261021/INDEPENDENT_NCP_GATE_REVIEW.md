# NCP runner / promotion gate의 경계 감사

N02 구현 담당자가 다른 담당자가 작성한 NCP runner와 promotion gate를 read-only로 검토했다. 실제 source/resume/refinement/coverage 문제를 발견하고 작성자에게 전달했다. 마지막 corpus-lock 및 executable-path guard 두 개는 root의 명시적인 재배정 후 검토자가 직접 구현했다. 따라서 이 두 최종 수정까지 독립적인 사후 코드 리뷰를 받았다고 주장하지 않는다. 별도 runtime agent가 최종 matrix를 담당한다.

검토한 범위는 source identity, frozen PLAN, dependency/runtime identity, restart, raw receipt, 필수 case coverage, 오류 판정과 scope다. Scientific benchmark나 native comparator를 재실행하지 않았다.

| 발견 사항 | 최종 처리 |
| --- | --- |
| build receipt가 실행 파일 바이트를 결속하지 않아 resume 시 다른 binary를 사용 가능 | build artifact SHA 및 실행 전후·resume binding 검사 |
| unreceipted 오래된 WP raw file을 새 실행 결과로 오인 가능 | 기존 파일을 보존하며 fresh 실행 거절 |
| 새 독립 참조 tier를 CLI에서 필수화하지 않음 | N02 corpus 및 `--require-independent-reference`, `--source-root` 고정 |
| source symlink의 실제 target 바이트 누락 | NCP는 in-repo regular target path/bytes와 link text 결속; 외부/깨진/cyclic target 거절 |
| 같은 host boot에서 dynamic library 교체를 resume identity가 놓침 | ldd-resolved loader/library SHA를 build 및 실행 경계에 결속 |
| LD_PRELOAD 순서 등 runtime 의미를 바꾸는 환경 누락 | loader·allocator·thread 관련 환경을 identity에 포함 |
| reference PASS label에 실제 refinement levels가 없어도 통과 | 모든 level/상태/grid/threshold/finest baseline 재검사 |
| summary가 줄인 required coverage를 그대로 신뢰 | committed config/PLAN에서 validation/case 목록 유도 |
| raw file이 그 실행의 receipt에 속하지 않아도 통과 가능 | raw hash와 해당 command artifact manifest 일치 요구 |
| 다른 arm/case/thread raw의 교체 가능 | frozen case와 command/raw identity 대조 |
| integrity 실패가 case에만 남고 top-level integrity는 PASS 가능 | integrity 오류를 전체 이유 목록에도 반영 |
| caller corpus가 frozen config lock과 다를 수 있음 | supplied corpus SHA와 config lock 비교; 회귀 테스트 |
| comparison command와 다른 유효한 executable binding 사용 가능 | command executable path와 binding path 비교; 회귀 테스트 |

마지막 두 수정을 포함한 targeted gate 계약 테스트는 13 PASS다. 두 guard를 제거한 controlled mutation에서 해당 regression 두 개는 실제로 실패한다. 초기 missing-level false-PASS JSON도 그대로 남겼다. 위 결함들은 현재 검토 범위에서 해결되었고 추가 차단 결함은 찾지 못했다.

단, 여기의 `SCOPED_CERTIFICATE_PASS`/`EMPIRICAL_GRID_PASS`는 모델·endpoint/stage/grid 계약의 범위를 유지한다. 자동 broad production 판정은 HOLD다. 실제 NCP 실행 및 native bench 통합의 성공 여부는 이번 소스 리뷰로 대체하지 않는다. Source 경계 검사는 quiescent checkout을 전제로 하며 지속적인 변조 감시나 악의적인 host에 대한 증명이 아니다. ldd manifest는 그 시점의 resolved dependencies를 다루며 임의의 dlopen/data 자산 전체를 증명하지 않는다.

검토 파일의 SHA-256, 담당자 관계, evidence 경로와 종료 상태는 같은 이름의 JSON 및 `N08_IMPLEMENTATION.json`에 기록했다.
