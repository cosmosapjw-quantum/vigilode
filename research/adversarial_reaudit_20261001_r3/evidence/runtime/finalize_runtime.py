#!/usr/bin/env python3
from pathlib import Path
import json,datetime,subprocess,collections,hashlib
root=Path(__file__).resolve().parents[3];ev=Path(__file__).resolve().parent
native=json.loads((ev/'recovered_native/summary.json').read_text())
assert native['native_campaign_status']!='RUNNING','native campaign must finish before summary'
source=subprocess.check_output(['git','-C',str(root/'vigilode'),'rev-parse','HEAD'],text=True).strip()
assert source==native['source_commit']
first=json.loads((ev/'full_suite_attempt1_incomplete/suite_summary.json').read_text());recovery=json.loads((ev/'full_suite/suite_summary.json').read_text())
changed=native['changed_target_results'];changed_passed=sum(x.get('rust_summary',{}).get('passed',0) for x in changed)
d={
 'schema':'vigilode.r3.final_runtime_summary.v1','finalized':True,'recorded_utc':datetime.datetime.now(datetime.UTC).isoformat(),
 'source_commit':source,'source_tree':subprocess.check_output(['git','-C',str(root/'vigilode'),'rev-parse','HEAD^{tree}'],text=True).strip(),
 'full_workspace_pass':False,'scientific_speed_claim':False,'prior_pass_inherited':False,
 'native_campaign_status':native['native_campaign_status'],'discovered_native_harnesses':native['discovered_binary_count'],'discovered_named_tests':native['discovered_named_test_count'],
 'test_counts':native['test_counts'],'binary_counts':native['binary_counts'],'observed_assertion_failures':native['test_counts'].get('FAIL',0),
 'changed_surface_contracts':{'target_count':len(changed),'passed_tests':changed_passed,'target_results':changed},
 'doctests':'NOT_RUN','ignored_tests':'NOT_EXECUTED','timeout_semantics':'unresolved within audit execution budget; neither assertion failure nor PASS',
 'toolchain':{'rustc':'1.94.1','cargo':'1.94.1','dependencies':'118/118 locked registry packages exact version+package checksum, offline','source':'setup_evidence.json'},
 'build_claim':'UNRESOLVED_FULL_WORKSPACE_BUILD_EVIDENCE',
 'build_attempts':[{'role':'one full build','budget_seconds':600,'elapsed_seconds':first['build']['elapsed_seconds'],'reported_exit_code':first['build']['exit_code'],'status':'INVALID_AS_COMPLETION_EVIDENCE: no build-finished and no executable records in redirected stream','receipt':'full_suite_attempt1_incomplete/suite_summary.json'}, {'role':'bounded cached artifact metadata recovery','budget_seconds':30,'elapsed_seconds':recovery['build']['elapsed_seconds'],'exit_code':recovery['build']['exit_code'],'status':'TIMEOUT; actual recompile observed, no further Cargo','receipt':'full_suite/suite_summary.json'}],
 'native_artifact_admission':'Exact current-turn Cargo metadata workspace target -> unique native test fingerprint -> executable --list successful; complete stdout/stderr captured to memory before persistence. Full build success is not inferred.',
 'native_campaign_budget_seconds':300,'native_campaign_elapsed_seconds':native['elapsed_seconds'],
 'profile':native['profile'],'audit_helper_correction':{'empty_artifact_vacuous_pass_rejected':True,'invalid_original_receipt':'full_suite_attempt1_incomplete/suite_summary_before_empty_guard.json','raw_invalid_driver':'full_suite_driver.log','guard':'run_bounded_suite.py requires nonempty native artifacts and successful build-finished record'},
 'source_manifests_and_lock_unchanged_in_first_campaign':first.get('source_manifests_and_lock_unchanged'),
 'evidence_paths':['setup_evidence.json','dependency_match.json','runtime_incidents.json','targeted_cache_recovery.json','recovered_native/summary.json'],
 'memory_events_after':native['memory_events_after']}
(ev/'FINAL_RUNTIME_SUMMARY.json').write_text(json.dumps(d,indent=2)+'\n')
counts=native['test_counts'];bs=native['binary_counts']
rows='\n'.join('| `'+x['target']+'` | '+str(x.get('rust_summary',{}).get('passed',0))+' | '+x['status']+' |' for x in changed)
body=f'''# R3 실제 런타임 검증 결과

고정 소스 `{source}` / tree `{d['source_tree']}`. **전체 workspace PASS로 판정하지 않는다.** 이전 실행 결과를 상속하지 않았으며 이번 turn의 toolchain compile, native 실행, 실패 로그를 각각 보존했다.

- Native harness **{d['discovered_native_harnesses']}개**, `--list`로 발견한 이름 있는 test **{d['discovered_named_tests']}개**.
- 실제 test 결과: **{counts.get('PASS',0)} PASS**, **{counts.get('FAIL',0)} assertion FAIL**, **{counts.get('UNRESOLVED_BOUNDED_TIMEOUT',0)} timeout 미확정**, **{counts.get('NOT_RUN_TOTAL_BUDGET',0)} 전체 시간 상한으로 미실행**, **{counts.get('IGNORED_NOT_RUN',0)} ignored 미실행**. 기타 상태가 있으면 아래 JSON 원장을 따른다: `{json.dumps(counts,ensure_ascii=False)}`.
- Harness 상태: `{json.dumps(bs,ensure_ascii=False)}`. 전체 native 시간 상한 300초, 실제 {native['elapsed_seconds']:.3f}초. 개별 harness 30초 또는 알려진 장시간군 60초까지; 전체 잔여 시간으로 줄어들 수 있다.
- **변경 표면 7개 harness의 37개 계약 모두 실제 PASS**. 이 숫자는 전체 PASS 수의 부분집합이며 중복 합산하지 않는다.

| 변경 target | PASS test 수 | 관측 상태 |
|---|---:|---|
{rows}

Rust/Cargo 1.94.1을 사용자 첨부 archive에서 설치하고 hello를 실제 compile/execute했다. Registry 의존성 118개 모두 source lock과 exact version/package checksum이 맞고, network fetch 없이 locked/offline로 실행했다. Python 3.12.14, NumPy 2.3.5, SciPy 1.17.0, Matplotlib 3.10.8 import는 실제 PASS; SymPy/mpmath는 없었다. Native 설정은 opt-level 0/debug info, dev/test codegen-units 1, incremental off, test threads 1이다. 성능 또는 wall speedup 측정이 아니다.

## 전체 build와 native 실행의 구분

최초 full workspace/all-targets/all-features no-run 작업은 600초 상한 안에서 {first['build']['elapsed_seconds']:.3f}초 후 wrapper exit 0을 보고했지만, live redirect된 Cargo JSON이 compiler/build-script records 중간에서 끝나 native executable이나 `build-finished`를 담지 못했다. 따라서 build completion 증거로 인정하지 않았다. 기존 audit runner의 빈 목록 `all([])`이 일시적으로 낸 PASS는 **무효**로 명시하고 원본 receipt를 보존했다. 수정된 runner는 native artifact 비어 있음과 `build-finished` 부재를 거부한다. 과거 R2의 131개 실제 binary 실행 기록은 빈 목록이 아니어서 이 helper 결함의 적용 대상이 아니다.

Metadata만 회수하려던 cached no-run 재호출에서도 실제 recompilation이 관측되었고 30초 상한에서 exit 124로 끝났다. 더 이상의 Cargo 실행은 하지 않았다. 이미 존재하는 current-turn test binaries는 source의 Cargo metadata, 유일한 `test-*` fingerprint, native `--list` 성공을 연결해 별도로 선정했다. 실제 실행 stdout/stderr를 memory에서 완전히 수신한 다음 저장했다. 이 개별 native 관측이 전체 build evidence의 빈 부분을 소급해서 PASS로 만들지는 않는다.

앞선 설치 중 SO 추출 잘림, shared cache zero-byte object, `fatal library error, lookup self`를 환경/도구 사건으로 보존했다. 지정된 generated cache만 정리하고 incremental을 끈 뒤 standalone probes는 실행 성공했다. all-features build에는 codegen-units 1을 사용했다. 원인 미확정인 I/O/cache 이상을 solver 결함으로 분류하지 않았다. OOM kill counter는 0이다.

Timeout은 수치 또는 assertion 실패로 해석하지 않는다. ignored와 doctest는 실행하지 않았고 PASS 합계에 넣지 않는다. 독립 연구 probe의 결과는 각 lane에서 별도로 보고하며 위 workspace native 합계에 중복 가산하지 않는다.

기계 판독 SSOT는 `FINAL_RUNTIME_SUMMARY.json`; 전체 test별 상태와 명령/exit는 `recovered_native/summary.json`과 대응 `.list.log`/`.run.log`에 있다.
'''
(ev/'RUNTIME_RESULTS_KO.md').write_text(body)
(root/'r3/RUNTIME_SUMMARY_KO.md').write_text(body)
inc=ev/'runtime_incidents.json';x=json.loads(inc.read_text());x['status']='NATIVE_CAMPAIGN_FINALIZED_FULL_BUILD_UNRESOLVED';x['final_summary']='FINAL_RUNTIME_SUMMARY.json';x['incidents'].append({'event':'incomplete redirected Cargo stream and empty-list false PASS','classification':'AUDIT_EVIDENCE_HELPER_ERROR','original_completion_claim_invalid':True,'native_test_inventory_recovered':d['discovered_named_tests'],'guard_repaired':True,'final_full_workspace_pass':False});inc.write_text(json.dumps(x,indent=2)+'\n')
print(json.dumps({k:d[k] for k in ['finalized','full_workspace_pass','discovered_native_harnesses','discovered_named_tests','test_counts','binary_counts']},indent=2))
