# D5 S3 자동 metadata CI — 실제 완료 결과

**전체 CI는 FAILURE다. Metadata checker unit tests 24개와 research-node/append-only ledger 169행 검사는 PASS이고, authority-reference 검사만 FAIL이다.**
과학 source/results/R6를 다시 실행하거나 검증한 결과가 아니다.

## 실행 identity와 실제 단계 결과

- [실제 run 40156477518](https://github.com/cosmosapjw-quantum/vigilode/actions/runs/40156477518), attempt 1, event `pull_request`.
- Workflow: `.github/workflows/research-process.yml`; job `121058848743`.
- 실제 source HEAD: `8c2a18c2c4706056e9bf9c7132aae419b787e5b3`.
- 실제 run metadata의 tree: `5e48e13d04073dc1de5069852b5e9bb5ec673772`.
- Run 생성 2026-10-25 02:58:37 UTC, 완료 상태 갱신 02:59:19 UTC. Job은 02:58:40–02:59:18 UTC다.
- 첫 단일 상태 조회에서 이미 completed/failure였다. 재요청·재실행·busy polling은 없었다.

| 실제 단계 | 결과 |
|---|---|
| Research-node checker unit tests | 16개 PASS; 로그의 실행 시간 1.664초 |
| Authority checker unit tests | 8개 PASS; 로그의 실행 시간 0.901초 |
| Research node / append-only ledger | PASS, 169행 |
| 새 node 판정 | `d5_callback_contract_20261025` |
| Ledger 비교 base | `origin/research/d4-effect-journal-20261025` |
| Authority references | FAIL, exit code 1 |

위 24개는 metadata checker 시험이다. D5의 과학 계약 시험 35개 또는 D2–D4 solver/benchmark/NCP 캠페인을 다시 실행한 수가 아니다.
관측된 CI 시간으로 기존 closed BUDGET을 수정하지 않았다.

## Authority 원문과 실패 집계

실제 로그의 요약은 다음과 같다.

```text
check-authority-refs: FAIL (8261 citations, 896 distinct SHAs:
missing=56, missing+allowlisted=27, non-commit=336, reachable=477)
```

별도로 실제 FAIL 줄을 파싱하면 **56 distinct SHA, 199 occurrences**다.
27개의 allowlisted missing은 이 199개 실패에 포함하지 않는다.
파일 경로에 따른 현재 실패 분해는 다음과 같다.

| 현재 실패가 기록된 파일 집합 | Distinct SHA | FAIL occurrences |
|---|---:|---:|
| D4 이전 연구 경로 | 32 | 143 |
| D4 연구/최종 전달 경로 | 7 | 25 |
| D5 연구 경로 | 17 | 31 |
| 전체 | 56 | 199 |

### 저장된 D4 baseline과의 비교

읽기 비교에 사용한 과거 receipt는 `recovered_d4_delivery/delivery/CI_RECEIPT.json`이며,
과거 run `40138539924`, source commit `c6630c1c412d41eb47adf013d23b385192971798`의 결과다.
그때의 **38 distinct / 152 occurrences**는 D4 이전 32/143과 D4 6/9로 분해된다.
기존 run이나 checker를 다시 실행하지 않았다.

현재와의 증가 **+18 distinct / +47 occurrences를 모두 D5에 귀속하면 틀린다.**
D5가 상속한 D4 최종 전달 문서에는 과거 CI 이후의 중앙 HEAD 참조와 외부 source binding 문서가 포함돼 있다.
D4 경로에서만 +1 distinct/+16 occurrences가 생겼다. 새 중앙 참조
`2354d6e995fde736fffbcad2d0e40473218cb8c4`가 4회이고, 이미 있던 D4 외부 SHA의 추가 인용이 12회다.
D5 경로의 실제 추가분은 **+17 distinct / +31 occurrences**다.
과거 38개 missing SHA는 현재 실패 집합에도 모두 남아 있다.

## D5 31개 실패의 범위

| D5 파일 | Occurrences |
|---|---:|
| RELATED_ADDITIONAL_KO.md | 15 |
| RELATED_REPOS_KO.md | 3 |
| REPORT_KO.md | 2 |
| evidence/REUSED_CR_PATTERN_PINS.json | 11 |
| 합계 | 31 |

실제 실패 위치와 이미 저장된 intake의 pin 문맥을 연결하면 17개는 모두 다른 repository의 source 식별자다.
기존 분류상 commit 12개와 blob 5개이며, htt_base·ISM·bass_cr·WU088_HH·BASS_HE·REC·BASS의
PR/branch/content commit 또는 C21H/C21I source/document blob에 해당한다.
가장 반복되는 C21H/C21I branch commit은 각각 8회다.
**D5 compiler-pin 실패는 이번 실제 FAIL 목록에서 관측되지 않았다.**

이 분류는 기존 source pin 문맥의 재사용이다. 외부 객체의 현재 가용성을 새로 조회하거나 재검증하지 않았다.
각 SHA, 기존 repository/type 분류, 실제 파일·source 줄·raw-log 줄은 `evidence/CI_S3_RESULT.json`에 보존했다.
앞서 읽은 checker는 repository namespace를 따라 외부 저장소를 조회하지 않고 VigilODE의 로컬 Git object database를 조회한다.
따라서 이 failure를 곧바로 “외부 source가 게시되지 않았다” 또는 “과학 결과가 실패했다”로 바꾸어 해석하지 않는다.
동시에 실제 authority gate는 FAIL 상태로 남긴다. Checker와 allowlist를 수정하지 않았다.

## 원자료 보존과 상태

- `evidence/CI_S3_JOB.log`: connector가 반환한 decoded UTF-8 job log 전체, **100268바이트 / 688행**.
- 원래 UTF-8 BOM, LF 및 마지막 newline을 보존했고, 파일 전체 readback 문자열이 connector 반환 문자열과 일치했다.
- Log SHA256: `2c96915f8c65dea8c43203544091e5d88ac00e70715bf601784d1497169b082c`.
- 비교한 D4 raw-log SHA256: `95cbce76a588f3ebdb932508eabbfb2f1cb555023bee405a790d826086e4a3d4`.
- 비교한 D4 CI-receipt SHA256: `d988b3f23ff0ba85f31ab5268d830a443980d1063e4a2353c474757cffeda0fa`.

이번 관측은 GitHub read-only 호출 3회(run/jobs/log), 저장된 baseline 읽기와 로그 파싱이다.
Dispatch/rerun/local checker/과학 실행/외부 객체 재검증/예산 수정은 모두 0이다.
`whole_ci_green=false`, `production=HOLD`, `global_completion=null`을 유지한다.
