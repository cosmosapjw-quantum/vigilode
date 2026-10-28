from pathlib import Path
import json, hashlib, re, subprocess
R=Path(__file__).resolve().parents[1]
load=lambda p:json.loads((R/p).read_text())
osc=load('oscillation/PILOT_RESULTS.json')['rows']
adapt=load('adaptation/PILOT_SUMMARY.json')
switch=load('switching/results_r2/SUMMARY.json')
parallel=load('parallel/PILOT_SUMMARY_REPAIRED.json')
co=next(r for r in adapt if r['epsilon']==1e-6 and r['end']==2 and r['method']=='cofactor')
ch=[r for r in osc if r['case']=='catalog-15-exponential-chirp' and r['physical_local_atol']==1e-8]
selected=[r for r in switch['groups'] if r['h0']==1e-4 and r['mode'] in ('dopri','rodas','pressure-only','evidence')]
findings={'schema':'vigilode.adapter-research.v1','base_commit':'d79326648a7273c0decd641eaf94d94f5bea7c63','publication_branch':'research/adapter-diagnostics-20261025',
'overall':'SPECIALIZED_PILOTS_VALIDATED_GENERAL_PRODUCTION_NOT_YET_QUALIFIED',
'findings':[
{'id':'F01','topic':'rapid oscillation','evidence_state':['derived','numerically_checked','implementation_verified'],'decision':'specialized_declared_chart_candidate','sources':['oscillation/THEORY_AND_PORT_KO.md','oscillation/SOURCE_INVENTORY.json'],'results':ch,'remaining':['dynamic physical norm','time-grid/phase certification','structure registration and charge-aware residual handling'],'not_claimed':['automatic structure discovery','unconditional frequency independence','matched-achieved-error generic speedup']},
{'id':'F02','topic':'step accumulation','evidence_state':['derived','numerically_checked','implementation_verified'],'decision':'stability_limited_hypothesis_strongly_supported; cofactor_specialized_candidate','stability':load('adaptation/STABILITY_MATCH.json'),'cofactor_result':co,'implementation_defect':'fused exponential PI/history update fixed separately; not cause of historical DOPRI campaign'},
{'id':'F03','topic':'method switching','evidence_state':['implementation_verified','numerically_checked'],'decision':'opt_in_pilot_optimization_possible','results_h0_1e_minus4':selected,'changes':['AutomaticOde resource/permission/replay preflight','evidence_driven cost-screened repeated bounded probes','native post-acceptance step proposal carry across intermediate windows','exact-target attempt-quota success handling','typed decision telemetry'],'remaining':['persistent PI/FSAL/workspace/implicit state','calibrated true-cost proxy','rotation probe overhead','declared structural candidate registry'],'scope':'method fitness, not comprehensive physical stiffness classifier'},
{'id':'F04','topic':'parallel dispatch','evidence_state':['derived','implementation_verified','numerically_checked'],'decision':'opt_in_ensemble_pilot; extend_after_local_evaluation','results':parallel['results'],'repairs':['post-calibration sunk cost separated from future dispatch','lazy pool and singleton skip','charged warmup and raw paired observations'],'remaining':['pre-calibration economics','PDE coarse-chunk/data and coupled-stage integration','host drift calibration'],'initial_poor_selection_preserved':'parallel/initial/'},
{'id':'F05','topic':'benchmark design','evidence_state':['implementation_verified'],'decision':'use_same_checkpoint_selection_evaluation','tests':22,'artifact':'selection_benchmark/PROTOCOL_KO.md','strict_measured_window_input_status':'NOT_YET_COLLECTED; current native pilots are whole trajectory or heldout batch comparisons'}],
'local_raw':'/tmp/vigilode-local-run-20261024.geLVCp9Q','old_campaign':'STOPPED per published receipt; no local process action performed here','no_ncp':True}
(R/'FINDINGS.json').write_text(json.dumps(findings,ensure_ascii=False,indent=2)+'\n')
text='\n## 7. 실제 pilot 결과와 판정\n\n![네 경로의 실제 pilot 결과](pilot_overview.png)\n\n그림 A는 실제 chirp의 RHS 작업과 물리 endpoint 오차, B는 boundary-layer의 시도 수, C는 선택 정책의 전체 solve 시간, D는 실행 backend 선택을 보여준다. D의 1 미만 값은 같은 backend를 서로 다른 시점에 측정한 변동을 포함한다. 이를 adaptive wrapper가 동일 병렬 연산보다 본질적으로 빠르다는 주장으로 읽지 않는다. D 중앙값에는 calibration이 빠져 있으며 아래 표와 JSON의 총비용에는 포함되어 있다.\n\n'
text+='| 실제 실험 | 관측 결과 | 적용 판정 |\n|---|---|---|\n'
for r in ch:
 text+=f"| Chirp {r['method']}, 요청 atol1e-8 | RHS {r['rhs_calls']:,}, endpoint 오차 {r['actual_endpoint_linf_error']:.3g}, 시간 {r['median_seconds']*1e3:.4g} ms | 공통 endpoint1e-4 기준 통과; 국소 norm 조건·달성 오차는 다름 |\n"
text+=f"| Case5 cofactor, epsilon1e-6,T2 | {co['attempts']}시도, 오류 {co['endpoint_max_abs_error']:.4g}, 시간 {co['median_seconds']*1e6:.4g} μs | 선언된 구조의 전문 경로; 원 좌표20만회 미완료와 속도비를 만들지 않음 |\n"
text+='\n방법 선택 비교는 h0=1e-4에서 다음과 같다. 모두 endpoint1e-5 기준을 통과했으며 워밍업1회 후3회 중앙값이다.\n\n| 문제 | 직접 DOPRI(ms) | 직접 RODAS(ms) | 기존 pressure(ms) | 새 evidence(ms) |\n|---|---:|---:|---:|---:|\n'
for c in ['decay','rotation','stiff-forced']:
 d={r['mode']:r for r in selected if r['case']==c};text+='| '+c+' | '+' | '.join(f"{d[m]['median_seconds']*1e3:.4g}" for m in ['dopri','rodas','pressure-only','evidence'])+' |\n'
text+='\n회전에서는 불필요한 implicit 상주를 피했지만 직접 DOPRI보다 여전히 비싸다. 강제 강성 문제에서는 RODAS로 전환했지만 직접 RODAS보다 비싸다. 따라서 다음 최적화는 더 많은 후보를 추가하기 전에 세션 재시작과 반복 pilot 비용을 줄이는 것이다. 3문제×2초기h×5경로의 작은 비교는 이력 전달·선택 정책의 ablation이며 전범위 method×thread campaign을 재개한 것이 아니다.\n\n'
text+='| Ensemble 길이 | 선택 | Held-out 최선 | 선택 불일치율 | 준비·보정 포함 adaptive 총시간(ms) | 직렬 총시간(ms) |\n|---:|---|---|---:|---:|---:|\n'
for r in parallel['results']:
 text+=f"| {r['batch_len']} | {'parallel' if r['selected_parallel_fraction']==1 else 'serial'} | {r['heldout_faster_arm']} | {r['choice_disagreement_fraction']:.0%} | {r['adaptive_total_with_calibration_and_startup']*1e3:.4g} | {r['serial_total']*1e3:.4g} |\n"
text+='\n각 batch의20회 held-out 관측에서 선택과 관측된 빠른 backend가 일치했다. 이 한 host 실험으로 미래 비용을 보장하지 않는다. 초기에는64·256 batch 모두 좋은 병렬 경로를 놓쳤으며 그 원본도 보존했다.\n\n'
text+='## 8. 검증과 다음 구현\n\n최종 source의 scoped Rust 검사59개와 선택 분석기 Python22개가 통과했다. 별도의 대수 검산5개, 독립 chart 회귀13개 및 선택 회귀17개도 통과했으며 일부는 같은 계약의 독립 재실행이므로 이를 모두 독립 시험 수로 합산하지 않는다. 전체 workspace suite나 전체 production benchmark의 통과를 주장하지 않는다.\n\n독립 검토에서 극단값 cofactor 산술, 작은 beta의 phase, 중간 window의 다음 h 갱신, 정확한 endpoint에 마지막 예산으로 도달한 상태를 수정했다. 초기 환경·타입·공유 artifact 오류와 초기 잘못된 선택은 보존했다. 최종 multi-example build와 겹친 `results_r1` 시간은 분석에서 제외하고, build exit0 이후의 `results_r2`를 본 표에 사용했다.\n\n다음 개발은 `NEXT_STEPS.json`의 원자료 보존→구간 계측→지속 세션→동적 물리 norm/구조 provider→실제 비용 선택→큰 격자 실행→선택 중심 검증 순서다. 정밀 이론 문제에만 Astra ultra와 연구 도구를 배정하고, 나머지는 저비용 실행·수정 경로로 분리하는 인계문서를 제공한다.\n'
p=R/'REPORT_KO.md';base=p.read_text().split('\n## 7. 실제 pilot 결과와 판정')[0];p.write_text(base+text)
# Final command-derived counts and source identities.
checks=[]
for name in ['native_final','exponential_final','selection_tests','build_final','format']:
 log=R/'evidence'/f'{name}.log'; exitp=R/'evidence'/f'{name}.exit'
 if log.exists(): checks.append({'name':name,'exit_code':int(exitp.read_text()) if exitp.exists() else None,'log':str(log.relative_to(R)),'sha256':hashlib.sha256(log.read_bytes()).hexdigest()})
counts=[int(v) for name in ['native_final','exponential_final'] for v in re.findall(r'test result: ok\. (\d+) passed', (R/'evidence'/f'{name}.log').read_text())]
assert sum(counts)==59
v={'schema':'vigilode.adapter-validation.v1','rust_tests_passed':sum(counts),'rust_tests_failed':0,'python_selection_tests_passed':22,'algebra_tests_passed':5,'independent_review':'reviews/INDEPENDENT_REVIEW.json','checks':checks,'scope':'focused affected/adjacent tests; no full workspace qualification','toolchain':'Rust1.94.1 supplied archive; locked dependencies fetched; actual tests offline','final_build_input_manifest':'evidence/SOURCE_FINAL_BUILD_INPUTS.json','final_runtime_metadata':'evidence/RUN_METADATA_FINAL_COMPLETED.json','final_switch_measurement':'switching/results_r2/SUMMARY.json','excluded_measurement':'switching/results_r1/MEASUREMENT_LIMITATION.json','source_policy':'earlier child validations retain their historical source bytes; root final build manifest and final tests bind formatted final native sources'}
(R/'VALIDATION.json').write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n')
