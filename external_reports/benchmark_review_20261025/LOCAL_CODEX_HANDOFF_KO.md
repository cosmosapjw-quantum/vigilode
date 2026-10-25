# 로컬 Codex 인계 프롬프트

아래 지시를 그대로 사용한다. 이 작업은 진행 중인 벤치마크의 설정 변경이나
재시작 요청이 아니다. 현재 실행을 보존하면서 보고서에 필요한 완료 자료를
추출하고, 다음 새 비교에 한정하여 입력 검증 수정을 반영하는 작업이다.

```text
저장소: https://github.com/cosmosapjw-quantum/vigilode
보고서 브랜치: report/independent-benchmarks-20261025
보고서 경로: external_reports/benchmark_review_20261025/
이번 보고서의 기준 commit: 2a4bf90d1445b67542bdac5030b4c5d10da20445
기존 전체 시험의 solver commit: aaf1f834d4c589e734ca7851a56c11bb44f7047a

1. 실행 중인 solver, checkout, binary, config, 결과 폴더를 그대로 보존한다.
   reset, checkout 전환, 프로세스 종료, 재빌드 또는 NCP 실행을 하지 않는다.
   현재 측정 batch가 끝난 뒤 다음 읽기 전용 자료 추출을 수행한다.

2. 보고서 브랜치를 fetch하고 원래 checkout 밖에 별도 detached worktree를 만든다.
   보고서 전달 메시지의 게시 commit과 일치하는지 확인하여 그 SHA를 고정한다.
   브랜치가 이후 바뀌었으면 전달된 고정 commit을 사용한다.
   README.md, BENCHMARK_FINDINGS.json, tools/README_KO.md를 읽는다.

3. 표준 Python만으로 exporter 자체15개, 독립9개 검사를 실행한다.
   README.md의 두 exporter test 명령을 따른다. 실패하면 원 로그를 보존한다.
   사용자 환경의 실제 RUN_ROOT를 기존 실행 identity/영수증에서 확인한다.
   repo 경로나 과거 보고서의 임시 경로를 RUN_ROOT로 추측하지 않는다.

4. 새 worktree를 cwd로 하여 다음 명령을 실행한다.
   python3 -B external_reports/benchmark_review_20261025/tools/export_local_measurements.py \
     --run-root "$RUN_ROOT" --output "$NEW_EXPORT_DIRECTORY" \
     --task-index research/local_frontier_native_diffsol_20261024/audit_packet_20261025/LEGACY_STANDARD_1539_STATUS_INDEX.json

   NEW_EXPORT_DIRECTORY는 기존 campaign과 checkout 밖의 새 경로다.
   task-index는 원래1539개 작업을 연결할 때 사용한다. 다른 campaign은 별도
   export하거나 적절한 자체 index를 사용한다. 종료되지 않은 작업은 incomplete로
   남기고, 종료 후 새 출력 디렉터리에 다시 export한다. 기존 export를 덮어쓰지 않는다.

5. MEASUREMENTS.jsonl과 EXPORT_SUMMARY.json을 보존한다. summary/파일 해시와
   terminal 상태를 확인하여 완결 export인지 점검한다. 원자료 누락, 읽기 중 변화,
   시간 초과, 정확도 미달, 참조 부족을 구분한다. raw_evidence_key 및 receipt hash로
   같은 실행의 중복 wrapper를 제거하고, 실패 시도도 남긴다. 계측 시간과 일반
   측정 시간을 합치지 않는다. 기존 결과를 자동 재분류하지 않는다.

6. 추가로 CPU 모델, 물리/논리코어, affinity, RAM, compiler/profile, 실행 source/tree,
   binary와 reference hash, 정확도기준, timing_scope, 사용한 thread환경을 반환한다.
   측정 당시 기록이 없는 항목은 현재 관측과 구별하여 MISSING_AT_MEASUREMENT로 쓴다.

7. 다음 새 native 비교에만 compare.py의 유한값 검증 수정을 반영한다.
   수정 파일: research/local_frontier_native_diffsol_20261024/compare.py
   SHA256: d74c2732d70871349574c3c84fdbf5637e459fe59a50df66a4f0179c85065396
   test_compare.py 8개를 먼저 실행한다. 기존 source-lock/runner_sha256 계약을 갱신한
   새 cohort 디렉터리로 시작한다. 실행 중인 canonical continuation 디렉터리는
   수정하지 않는다. 보고서에 복사된 archived controller는 실행 대상으로 쓰지 않는다.
   NaN/Inf 종료 시각·상태를 거부하는 수정이며 허용오차를 완화하지 않는다.

8. 이번 인계만으로 새 장시간 benchmark를 시작하지 않는다. 기존 계획의 남은
   실행과 예산을 유지한다. 추가 진단을 할 때는 별도 작업으로 구분한다:
   - gyromotion/chirp의 여러 시점 성분·진폭·위상 오차와 tolerance별 비용
   - Robertson의 명시적/음해적 체류시간·전환 탐색 비용
   - KS 및3D문제의 수락 스텝·도달시간, RHS·선형풀이 비용
   - 같은 문제의1/2/4/8스레드 CPU·OS시간·문맥 교환
   - 같은 구조의 재사용1/4/16/64/256회, 준비·잔차검사·재구성 비용
   이때 원래 정확도 기준을 유지하고 reference보다 작은 차이는 불확실성과 함께 쓴다.

9. 비교 대상은 VODE/DVODE, diffsol, Julia로 유지한다. 새로운 참조를 사용할 때는
   모델/IC/구간/성분순서/hash를 연결한다. 사례21/23은 셀별u,v교차배치다.
   연속화나 다항식 커널의 배수를 전체 ODE 배수로 쓰지 않는다.

10. 최종 반환: 새 export 경로, 실제 command/exit, 원본 보존 여부, source/binary/ref
    identity, 완료/미완료/누락 개수, 중복 제거 기준, 사용한 예산과 남은 작업.
    별도 요청 없이 remote push 또는 production 기본값 변경을 하지 않는다.
```
