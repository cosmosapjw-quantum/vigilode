# 분기 통합 독립 검토

판정은 **검토한 working tree의 기능·증거 통합 PASS, 원격 계보·최종 tree 확인 대기**다. 연구 분기의 파일을 일부 복사했다는 이유만으로 통합을 완료했다고 판단하지 않았다. Git blob, 실제 파일, 공개 commit의 ledger를 별도로 대조했고, 기존 scientific campaign과 증명은 다시 실행하지 않았다.

| 독립 확인 항목 | 결과 |
|---|---:|
| 공통 조상→최신 연구 head 변경 경로 | 1,133 |
| 그중 현재 byte-identical | 1,109 |
| 명시적 호환성·교정으로 변경 | 24 |
| 누락된 incoming 변경 경로 | 0 |
| 최신 연구 branch 전체 경로 | 6,451개 모두 존재 |
| production 통합 기반 전체 경로 | 6,344개 모두 존재 |
| 독립 검증한 추가 복사 기록 | 788개, 13,268,925 bytes |
| 두 ledger 원문 | 각각142행, 공개 commit과 완전 동일 |
| commit-qualified ledger ID | 284개, 중복0 |
| 같은 ID의 의미 동일/충돌 | 135 / 7 |

24개 차이는 처음의14개 충돌 해결과 import 이후10개 교정이다. 상세 경로·현재 hash·원래 hash·결합 이유는 JSON에 있다. frozen PR101와 parameterized PR101의 분리, 두 packet 반경 getter의 공존, W-filter 경로와 `VerifiedResidual`의 공존, production/export와 연구/export의 합집합을 확인했다. 출력 API의 scoped 계약을 public facade 자동 라우팅으로 확대하지 않았다.

연구 tree의5,341개 경로 중5,337개는 현재도 byte-identical하다. 나머지는 ledger와 production 쪽의 기존 continuation 문서3개다. 후자3개는 공개 production head `f2f20c1`과 현재 파일이 완전히 같고 연구 branch가 공통 조상 이후 바꾸지 않은 파일이다. 따라서 더 최근 production 결정의 보존이며 연구 결과의 유실이 아니다. ledger는 두 공개 branch의 원본 archive를 각각 남겼다.

독립 검토 중 production ledger ID가 local-only merge commit을 가리키는 문제를 발견했다. 세 공개 head를 부모로 게시할 경우 그 local commit은 원격 계보에 없을 수 있다. 이를 공개 `f2f20c1d8cd606e70ddbd8409c0f3b4529e928a1`로 고쳤고 archive bytes,284개 행의 raw/canonical hash, 모든 qualified ID를 재확인했다. 원문 `supersedes`는 해당 archive 내부에서 해석해야 한다. production16개와 all-DAG14개 edge가 모두 자기 namespace에서 해소된다. 충돌하는 `L-0136`–`L-0142`는 양쪽 의미를 보존하며 서로를 supersede한다고 추정하지 않는다. qualified ID의 commit은 archive snapshot이며 원래 실행 commit을 대체하지 않는다.

component bridge에서는246개 과거 registry와254개 현재 registry를 같은 숫자로 비교하던 두 시험을 명시적 집합 비교로 교정했다. 기존246개와 정확히8개 새 모듈의 합집합을 확인하며,78개 campaign case와94개 variant를 유지한다. 전체 계약 시험 최종 재시도는 **90 passed,3 skipped**, focused RSS 계약은 **28 passed**다. skip은 frozen Hairer library 부재이며 pass에 포함하지 않는다. 실패했던 이전 로그도 그대로 보존했다. 이 숫자는 solver 성능 campaign의 신규 결과가 아니다.

RSS bridge는 GNU time 부재 또는 매우 짧은 process로 실제 sample을 얻지 못했을 때 `null/UNAVAILABLE`을 반환한다. 양수 관측은 보존하고 과거 metric 파일은 새 실행 전 제거한다. 이 값은 관측 최대치이며 전체 process-tree peak의 인증된 상한이 아니다. unknown memory는 P3 ceiling gate를 통과시키지 않는다.

최종 완료를 위해 root가 게시 commit에 `cc2e1a00`, `f2f20c1d`, `ee65d4db` 세 head의 조상 관계를 확인하고, 원격 tree와 최종 source manifest 및 두 archive hash가 검토한 최종 파일과 일치함을 확인해야 한다. 이 검토는 해당 push가 이미 성공했다고 주장하지 않는다. 통합 PASS를 production·전역 정확도·속도 PROMOTE로 해석해서는 안 된다.

재현 방식은 JSON의 `method`, 경로별 결과는 `changed`, 시험 원문은 `test_receipts`, ledger의 정확한 namespace 연결은 `ledger.supersession_edges`를 사용한다.

후속 신규 코드에 대한 구분: 위254개 registry와90개 pytest PASS는 분기 통합1차 checkpoint다. 이후 N03 `global_budget`와 N05 `stage_target_policy`를 구현하면서 현재 registry는256=과거246+명시적10개가 되었다. 기존 과거 map과1차 receipt는 바꾸지 않았으며 final 현재 시험은 새 두 모듈을 포함한 명시적 합집합을 검사한다. runtime 담당자가 보고한 최종 Python 결과는139 PASS/3 SKIP이며 전체 source 최종 gate는 root 기록을 따른다.
