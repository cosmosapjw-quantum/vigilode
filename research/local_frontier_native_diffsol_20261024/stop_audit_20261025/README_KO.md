# 로컬 벤치마크 중지 시점 외부 감사 패킷

사용자 요청에 따라 2026-10-25 21:32 KST에 벤치마크 작업을 중지했다. 새 solver 실행, 빌드, 정확도 코호트, timeout 진단은 시작하지 않았다. 중지 확인 때 일치하는 측정 프로세스는 없고 공유 실행 잠금은 비어 있었다. 원시 자료는 `/tmp/vigilode-local-run-20261024.geLVCp9Q`에 그대로 남겼다. 이 경로는 로컬 임시 디렉터리이므로 이후 호스트 정리 정책에 따른 소실 위험이 있으며, 이 Git 패킷은 대용량 raw의 백업이 아니다.

이 패킷은 기존에 게시된 [`audit_packet_20261025`](../audit_packet_20261025/README.md)를 보충한다. [`STOP_STATUS.json`](STOP_STATUS.json)은 중지 시점의 source, binary, host 관련 영수증 연결, 비용, 작업별 최종 상태를 담는다. [`evidence/SHA256_MANIFEST.json`](evidence/SHA256_MANIFEST.json)은 복사한 작은 파일 74개의 SHA-256, 크기, 로컬 원본 경로를 기록한다. 상대 경로는 이 패킷 디렉터리에서 해석한다. 큰 raw와 측정 바이너리는 Git에 넣지 않았다.

## 재계산

```bash
python3 -B research/local_frontier_native_diffsol_20261024/stop_audit_20261025/verify_packet.py
```

검사는 복사본 해시, 191개 비용 이벤트의 연결 해시와 9개 중첩 fixture 비용(총 4,107.270175초), case20 전체 1,024성분 4개 endpoint의 여섯 쌍 차이와 경험적 불확실도를 다시 계산한다. 기존 standard 최종 상태 요약과 Julia 8그룹/48시도 요약의 일관성, 정확도 V3 계획의 미승인 상태도 확인한다. 이 검사는 로컬 raw를 요구하지 않는다. 원본 이벤트가 참조하는 개별 raw 해시 전체 검증은 로컬 원시 자료가 필요하다.

## 결과와 한계

| 범위 | 관찰·판정 |
| --- | --- |
| 기존 standard | 1,539개 작업, 1,540개 시도. 최종 `PASS_SCOPED` 323, `ACCURACY_FAILED` 146, `SOLVER_FAILED` 57, `NO_RESULT` 208, `UNVALIDATED_REFERENCE` 805. 199개 최종 비계측 timeout과 54개 step-budget 실패는 그대로 둔다. 새 참조로 과거 행을 소급 승인하지 않았다. |
| 보고서 인계 export | 고정 보고 commit `abf9c568…`, tree `df44f97…`에서 읽기 전용 추출. 2,565행 중 raw 검증 2,417, evidence issue 124, unsupported/nonterminal 16, 읽기/형식 오류 8. 자세한 구분은 `LOCAL_REPORT_EXPORT_INDEX.json`에 있다. |
| Julia 비교 | cases 1/10/15/16의 Tsit5·Rodas5P, 8그룹 48시도 모두 해당 endpoint 오차 조건에서 `PASS_SCOPED`. 시간은 `diagnostic_only`; 다른 source/host와 새 속도 배수는 만들지 않았다. |
| 독립 참조 | case 1/2/3/5/8/9/20의 좁은 범위 참조를 재사용한다. case20 경험적 `U=3.5435704834308433e-9`; case11 시도는 `FAILED_REFERENCE`. case 4/12/13/14/17/18/21/23/24는 참조 작업 미완료다. 경험적 U를 엄밀한 인증 상한이라 부르지 않는다. |
| 정확도 V3 | source `75588552…`의 오프라인 측정 빌드 통과, driver 합성 계약 31/31 통과, 독립 소스 리뷰는 `APPROVED_SOURCE_PREPARATION_ONLY`. 12그룹/72시도 계획은 승인 전 사용자가 중지를 지시하여 **0시도**, `NOT_RUN_APPROVAL_PENDING_AT_STOP`. 계획 결합 오류 두 건의 원본도 보존했다. |
| Timeout 진단 | WIP runner는 독립 재리뷰에서 `BLOCKED_REPAIR_REQUIRED`. 9개 진단 시도는 **0회**, `NOT_RUN_IMPLEMENTATION_GATE_BLOCKED_AT_STOP`. 차단된 WIP 소스와 테스트는 `blocked_timeout_source`에 감사 증거로만 복사했으며 실행 코드로 통합하지 않았다. |

`evidence/state/LEGACY_STANDARD_TASK_INDEX.json`은 1,539개 각 작업의 최종 근거를 찾는 전체 인덱스다. `evidence/state/TIMEOUT_FAMILY_TRIAGE.json`, `TERMINAL_MATRIX.json`, `DEPENDENCY_STATUS_MATRIX_20261025.json`은 실패와 미완료를 포함한다. `evidence/case_references`와 `evidence/publication_supplement`는 작은 전체 벡터 참조, 독립 리뷰, 48시도 Julia 비교 인덱스를 포함한다. `evidence/blocked_timeout_source`는 미승인 코드의 상태를 재현하기 위한 사본이다.

## 중지 후 남은 판정

Broad production은 **HOLD**다. 정확도 V3 실제 실행, 199개 timeout의 제한된 진단, 나머지 9개 외부 참조, 전체 범위의 독립 비교가 끝나지 않았다. 중지 명령이 있으므로 자동 재개하지 않는다. 후속 분석 시에는 [`STOP_STATUS.json`](STOP_STATUS.json)의 고정 source/tree/binary/ref와 로컬 raw 경로를 먼저 확인한다. 원본 checkout, 후속 checkout의 dirty WIP, 고정 벤치마크 checkout, 모든 로컬 영수증을 변경하거나 삭제하지 않았다.
