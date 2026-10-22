from pathlib import Path
import json,math,statistics,hashlib,collections
r=Path(__file__).resolve().parent
s=json.loads((r/'merged-comparison/SUMMARY.json').read_text());x=json.loads((r/'external-comparison/CAMPAIGN.json').read_text());v=json.loads((r/'external-comparison/ADAPTER_REPLAY_SUMMARY.json').read_text()) if (r/'external-comparison/ADAPTER_REPLAY_SUMMARY.json').exists() else None
identity=json.loads((r/'EXTERNAL_ADAPTERS.json').read_text());env=json.loads((r/'CAMPAIGN_ENVIRONMENT.json').read_text());ex=json.loads((r/'EXTERNAL_EXECUTION.json').read_text());pre=json.loads((r/'frozen-execution/preflight/SUMMARY.json').read_text())
monitor=[json.loads(line) for line in (r/'CAMPAIGN_MONITOR.jsonl').read_text().splitlines()]
load_range=[min(z['loadavg'][0] for z in monitor),max(z['loadavg'][0] for z in monitor)]
rows=s['rows'];comps=s['common_error_comparisons'];receipts=json.loads((r/'external-comparison/RECEIPTS.json').read_text())
notes={'decay':'forward identity-mass 선형 감쇠 표본. 일반 비선형 문제군의 보증으로 확장하지 않는다.','driven':'시간 의존 forcing 표본. Native 관측점은 실제 step clipping, 외부는 보간이다.','logistic':'제한된 비선형 logistic 표본. 임의 비선형성 또는 양성 보존 보증이 아니다.','harmonic':'유한 주파수·시간의 조화진동 표본. 관측점 오차가 위상오차의 모든 시간 상계는 아니다.','rapid':'100–115 주파수 진동. tight cap에 대한 미달을 최선 achieved-error와 함께 그대로 남긴다.','damped':'감쇠 진동 표본. 작은 진폭의 상대오차는 이 normalized metric만으로 보증되지 않는다.','mixed-scales':'초기 진폭1e-8–1e3, scalar atol 캠페인. componentwise 상대오차 인증은 아니다.','nonnormal':'8차원 K=30 transient 표본. 일반 비정규 행렬·pseudospectrum 검증이 아니다.','large-vector':'반복되는8개 대각 감쇠율의4096/16384 상태. 일반 대규모 결합 문제의 복잡도 근거로 확대하지 않는다.','moderate-stiff':'12차원 대각 negative control. explicit 자동 stiffness 탐지·switching은 없다.','stiff-control':'1/1000/100000 감쇠율의12차원 negative control. DOPRI5 비용 증가를 그대로 보고한다.'}
next_steps={'decay':'동일 paired 환경에서 setup/callback 비용과 수락 step 수를 분리 측정한다.','driven':'비자율 forcing 평가와 출력 보간의 비용·오차 기여를 분리한다.','logistic':'비선형 상태 범위를 넓히는 별도 등록 검증을 준비한다.','harmonic':'장시간 위상·진폭 오차를 분리하고 관측점 밖 검증을 별도 등록한다.','rapid':'tight cap 미달의 위상오차/안정성 비용을 진단하고 필요 시 새 허용오차 grid를 사전등록한다.','damped':'작은 진폭 성분의 상대오차와 감쇠 envelope를 별도 검증한다.','mixed-scales':'scalar atol 결과와 별도로 vector atol·성분별 오차를 검증한다.','nonnormal':'transient 증폭량과 성분별 오차를 분리하고 K/시간범위를 별도 등록한다.','large-vector':'accepted snapshot·보간·메모리 복사와 BLAS/thread 비용을 paired 환경에서 분리한다.','moderate-stiff':'명시적 안정성 제한으로 인한 RHS/거절 비용을 stiff 경로와 대조한다.','stiff-control':'명시적 방법의 비용 손실을 유지하고 수락/거절·선형해법 비용을 별도 진단한다.'}
families=[]
for case,n in dict.fromkeys((z['case'],z['dimension']) for z in rows):
 rr=[z for z in rows if z['case']==case and z['dimension']==n];cc=[z for z in comps if z['case']==case and z['dimension']==n];best={}
 for mode in ['dopri5','rodas-dense','rodas-mf','vode','diffsol','julia']:
  eligible=[z for z in rr if z['mode']==mode and z['all_runs_successful'] and z['max_normalized_error'] is not None]
  best[mode]=min((z['max_normalized_error'] for z in eligible),default=None)
 families.append({'case':case,'dimension':n,'scope':notes[case],'best_sampled_normalized_error':best,'common_cap_cost_comparisons':cc,'configurations':rr,'next_improvement':next_steps[case]})
process={}
for mode in ['diffsol','julia']:
 rs=[z for z in receipts if z['mode']==mode];dur=[z['elapsed_process_seconds'] for z in rs if z.get('elapsed_process_seconds',0)>0];prep=[];fail=[]
 for z in rs:
  p=r/'external-comparison'/z['stdout']
  try:
   d=json.loads(p.read_text());a=d.get('adapter',{});q=a.get('preparation_seconds')
   if q is not None:prep.append(q)
   if d.get('success') is not True:fail.append({'receipt':z,'message':d.get('message')})
  except Exception:pass
 process[mode]={'statuses':dict(collections.Counter(z['status'] for z in rs)),'process_seconds':{'min':min(dur,default=None),'median':statistics.median(dur) if dur else None,'max':max(dur,default=None)},'preparation_seconds':{'min':min(prep,default=None),'median':statistics.median(prep) if prep else None,'max':max(prep,default=None)},'solver_failure_records':fail}
res={'schema':'vigilode.nonstiff-external-completion.v1','native_problem_source_commit':identity['native_problem_source_commit'],'native_problem_tree':identity['native_problem_tree'],'adapter_source_commit':identity['adapter_source_commit'],'adapter_source_tree':identity['adapter_source_tree'],'native_code_changed':False,'root_cargo_graph_changed':False,'original_checkout_preserved':True,'reused':{'native_contract_tests':25,'baseline_process_records':576,'ensemble_process_records':16,'solver_reruns':0},'new_campaign':x,'strict_preflight':pre,'merged_replay':s['replay'],'process_details':process,'sampled_host_loadavg_1m_range':load_range,'families':families,'timing_authority':'diagnostic_only','comparison_rule':'Fastest eligible registered configuration under the same sampled achieved-error cap; actual achieved errors may differ. All4 warmup/timed receipts must pass strict replay. Baseline and external cohorts differ.','claim_ceiling':{'general_production':'HOLD','statistical_timing_authority':'HOLD','global_or_uniform_time_error_certificate':False,'independent_trajectory_parallelism_only':True},'independent_review':'See REVIEW.json for final publication decision; this generated report is not that decision.'}
(r/'RESULTS.json').write_text(json.dumps(res,indent=2)+'\n')
def num(z):return 'N/A' if z is None else f'{z:.4g}'
def ratio(f,m,t):
 c=next((c for c in f['common_cap_cost_comparisons'] if c['comparator']==m and c['normalized_error_threshold']==t),None)
 return num(c.get('comparator_over_explicit_time')) if c and c['status']=='COMPARABLE' else 'N/A'
def tab():
 t=['|문제·차원|DOPRI5 최선 오차|diffsol 최선 오차|Julia 최선 오차|diffsol/DP 1e-6|diffsol/DP 1e-8|Julia/DP 1e-6|Julia/DP 1e-8|','|---|---:|---:|---:|---:|---:|---:|---:|']
 for f in families:
  b=f['best_sampled_normalized_error'];t.append('|'+ '|'.join([f"{f['case']} · {f['dimension']}",num(b['dopri5']),num(b['diffsol']),num(b['julia']),ratio(f,'diffsol',1e-6),ratio(f,'diffsol',1e-8),ratio(f,'julia',1e-6),ratio(f,'julia',1e-8)])+'|')
 return '\n'.join(t)
comparable=[c for c in comps if c['comparator'] in ('diffsol','julia') and c['status']=='COMPARABLE'];loss=[c for c in comparable if c['comparator_over_explicit_time']<1];miss=[c for c in comps if c['comparator'] in ('diffsol','julia') and c['status']!='COMPARABLE']
report=f'''# Nonstiff 외부 비교 완료: diffsol/Julia 로컬 어댑터와 감사 자료

사용자가 승인한 호환 어댑터 설치·개발과 빠진 외부 비교만 수행했다. Native solver,
기본 dispatcher, 공개 API 지원 범위 및 root Cargo.toml/Cargo.lock은 변경하지 않았다.
기존25개 계약 시험,576개 native/VODE process record,16개 ensemble record는 재실행 없이
원래 해시와 source identity로 재사용했다. 과거 보고서의 NOT_RUN은 당시 사실로 보존되며,
이번 별도 외부 cohort가 그 누락을 보완한다.

## 실제 실행 identity

- Native/problem source: `{identity['native_problem_source_commit']}`
- Native tree: `{identity['native_problem_tree']}`
- Adapter/runner 실행 source: `{identity['adapter_source_commit']}`
- Adapter/runner tree: `{identity['adapter_source_tree']}`
- Diffsol measurement binary SHA256: `{identity['diffsol']['binary_sha256']}`
- Native binary SHA256: `50534f6dc5da3c7c123e74c83c2d6d437474a94282decd274711534292586ac1`
- Julia executable SHA256: `{identity['julia']['runtime_executable_sha256']}`

후속 보고서 HEAD는 실행 소스가 아니다. 실제 명령·exit code·input/output hash는
EXTERNAL_ADAPTERS.json, evidence/frozen-execution/COMMANDS.json, EXTERNAL_EXECUTION.json,
각 PLAN/RECEIPTS/RAW_MANIFEST와 AUDIT_MANIFEST.json에 있다. 바이너리·depot cache는
Git에 넣지 않았으며, 고정 commit·lock·runtime hash로 재빌드한다.

## 구현과 시험

Diffsol0.16.1/faer standalone workspace는 비강성 Tsit45, 두12차원 강성 대조군은
BDF+analytic JVP/FaerLU를 사용한다. Julia1.12.7의 Tsit5/2.1.5와 Rodas5P/2.7.5는
exact Project/Manifest로 고정했다. Rodas5P는 analytic state Jacobian과 package 기본
linear solver/AD 설정을 사용한다. 두 explicit 경로는 대규모 dense Jacobian을 만들지 않는다.
외부 관측은 한 adaptive history의 보간/saveat이며 native의 관측점 step clipping과 다르다.

Diffsol 최종19개 프로토콜 검사와 strict preflight32개(24 positive case/mode쌍+
8 부정 대조)가 통과했다. Julia 수정 검사는 정상12개·예상 max-step 실패 대조3개·잘못된 요청9개와
모든 RHS/초기값/기준해 및4096/16384 shape를 다뤘다. 정상12개는 수정 전후 상태
비트·시간·작업량이 같았다. 이것은 전체 저장소·모든 production 문제군의 검증이 아니다.
외부 heap allocation은 미계측(null)이며 native allocation 시험을 외부로 확장하지 않는다.

독립 리뷰에서 엄격 검증의 비용 자격 누락, 빈 실패 출력의 통과, Julia rejected trial의
accepted-state 오표기, mode/dimension 거절 누락을 발견하여 한 제한 수정 회차로 고쳤다.
최초 Julia MaxIters 증거는 t=0의 trial 벡터가 초기값과1.0859190353467283만큼 달랐으며
DEVELOPMENT_EVIDENCE.tar.gz에 보존했다. 이제 accepted step 관찰 callback이 preallocated
snapshot을 복사한다. 상태 변경·추가 관측점은 없으며 그 setup/copy 비용은 timed interval에
포함한다. strict-invalid 데이터는 원본 그대로 보존하고 파생 분석에서 비용 자격을 제외한다.

## 측정 결과와 정확도

새 캠페인:96개 설정·384회 계획, 실제 receipt {x['receipt_count']}개,
완료 process {x['completed_processes']}개, 기타 process 결과{x['other_process_outcomes']}개.
하네스 관측 시간{x['elapsed_seconds']:.6f}s;7200s 누적 예산과45s/process 한도를 유지했다.
사전 설치·빌드·preflight는 이 solver 캠페인 예산 밖이며 전체 작업이2시간 이내라는 주장은 하지 않는다.
기존144설정과 새96설정의 병합은240설정이며 replay 결과는 `{json.dumps(s['replay'],ensure_ascii=False)}`다.
수치적 성공은 exit0과 분리해 저장 상태·기준해·오차 요약·source/method/시간격자/초기값을 재검증했다.

sampled normalized error는 저장된5개 시각·성분에서 `abs(y-reference) / max(1,abs(initial),abs(reference))`의 최댓값이다.
아래 최선 오차는 등록된 네 rtol 중 strict-valid 성공 구성의 최소 sampled normalized error다.
비용비는 공통 오차 상한1e-6/1e-8 아래 각 방법의 가장 빠른 적격 구성을 선택한
`comparator median seconds / DOPRI5 median seconds`다.1보다 크면 그 표본에서 DOPRI5가
빠르다. 동일 상한 충족은 동일 실제 오차를 뜻하지 않는다. N/A를 임의 수치로 채우지 않았다.

{tab()}

![전체 work–precision 진단](evidence/merged-comparison/WORK_PRECISION.png)

새 외부 비교48개 중 적격 비교{len(comparable)}개, N/A{len(miss)}개이며,
적격 비교 중 DOPRI5 비용 손실(비용비<1)은{len(loss)}개다. 손실·미달·timeout·solver failure를
RESULTS.json과 원시 기록에서 삭제하지 않았다. Native dense/matrix-free 및 VODE 비교까지
포함한 전체 수치와 선택 구성 ID는 evidence/merged-comparison/SUMMARY.json에 있다.

## 적용 범위와 남은 일

'''+ '\n\n'.join(f"- **{f['case']} · {f['dimension']}**: {f['scope']} 최선 오차/비용 이득·손실은 위 표와 machine-readable family 행을 따른다. 다음: {f['next_improvement']}" for f in families)+f'''

후속 개선은 rapid의 tight sampled-error 미달·강성 negative control의 비용 증가,
대규모 표본의 adapter/BLAS/메모리 비용 차이를 각각 좁게 조사하는 것이다. 현재 측정 결과에
맞춰 rtol grid·방법·오차 임계값을 바꾸거나 캠페인을 재실행하지 않았다. 통계적 성능 주장에는
동일 시간대의 paired 설계와 추가 반복이 필요하다.16384 VODE 비용 급증 원인은
NOT_MEASURED이며 이 cliff를 복잡도나 보편적 가속비로 해석하지 않는다.

Dopri5는 사용자가 직접 선택하는 forward identity-mass ODE 경로다. 비자율 RHS와
scalar/vector atol을 지원하지만 이번 캠페인은 scalar atol이다. Jacobian/JVP/dfdt,
factorization/Newton solve를 호출하지 않는다. mass matrix 선언/event/dense output은
RHS 전에 거절한다. 관측시각에는 실제 step을 잘라 도달하며 자동 stiffness 탐지나
method switching은 없다. Jacobian 부재가 nonstiff 판정 근거는 아니다. 한 solve의 RHS
수학적 모델은 FSAL을 위해 동일해야 한다. partial solution은 success=false와 마지막
accepted 상태를 보존하고 호출자는 ensure_success() 또는 명시적 success 검사를 해야 한다.

기존 ensemble 결과는16개 독립4096차원 궤적의1/2worker throughput이며 각 궤적의
시간격자·오차 제어는 독립적이다. worker 사이 상태·작업량 일치를 보존했다. RK stage,
단일 궤적 또는 homotopy 가속 증거가 아니다.

## 시간 해석과 환경

CPU `{env['cpu_model']}`, affinity0–23, 실제 cgroup CPU quota/memory limit는 max다.
Python{env['python_version']}/NumPy{env['python_packages']['numpy']}/Matplotlib{env['python_packages']['matplotlib']}/SciPy{env['python_packages']['scipy']}.
Rust/Cargo1.94.1 measurement profile은 opt-level3/LTO=false/codegen-units16이다.
공유 host의 load·quota·메모리·runtime/sysimage hash는 CAMPAIGN_ENVIRONMENT.json에 있다.
주기적으로 관측한1분 load 범위는{load_range[0]:.3f}–{load_range[1]:.3f}이며 연속 관측의 엄밀한 최솟값/최댓값은 아니다.
Julia는1thread/OpenBLAS1이며 기존 VODE 환경의 NumPy OpenBLAS24 관측을 바꾸지 않았다.

Julia는 매 process에서 동일 요청의 method/type/shape 준비 solve1회를 먼저 수행하고,
새 상태·문제·작업공간·counter로 측정한다. 이 JIT 준비는 campaign warmup과 별개다.
imports/준비/JSON을 포함한 process wall은45s 한도 안에 있고 solver seconds와 별도로
RESULTS.json에 min/median/max를 기록했다. 준비 시간은 solver timer 밖이다. 측정에는
문제/초기값 구성·callback·거절/실패 작업·저장 상태 수집·accepted snapshot 복사가 포함되고,
기준해 평가·error scoring·JSON serialization은 제외된다. VODE Python callback overhead는
포함하며 Adams/BDF Fortran VODE 비교이지 RK45/Radau/LSODA 대체 비교가 아니다.

세 timed 반복의 min/max는 신뢰구간이 아니다. 모든 wall-time은 diagnostic_only다.
Native/VODE와 새 외부 어댑터는 다른 시간대 cohort이므로 비용비는 탐색적 진단이다.
4개 rtol grid의 최선 후보는 전역 최적 비용을 뜻하지 않는다. rtol은 전역오차가 아니고,
endpoint/5개 관측점 오차는 모든 시간의 엄밀한 보증이 아니다. ConditionalL2나 interval/global
certificate의 승격도 수행하지 않았다. 범용 production·통계적 timing authority는 HOLD다.

## 감사 자료

원래 checkout과45037fc2 detached source의 HEAD/tree/status 최종 확인은 FINAL_CHECKS.json에
있다. NCP/SSH/reset/clean/강제 checkout은 사용하지 않았다. 기존 결과는 덮어쓰지 않았다.
개발 첫 실패, 설치·lock·targeted 시험은 DEVELOPMENT_EVIDENCE.tar.gz/DEVELOPMENT_MANIFEST.json,
외부 및 병합 원시 출력은 각 RAW_OUTPUTS.tar.gz/RAW_MANIFEST.json,
PLAN/RECEIPTS/SUMMARY/REPLAY와 PNG/SVG/PDF 도표는 evidence 아래 보존했다.
BASELINE_MAPPING/SOURCE_CAMPAIGNS는 원래 ID·cohort·path·hash를 추적한다.
최종 독립 판정은 REVIEW.json을 따른다. 외부 감사용 공개는 범용 production 승격이 아니다.
'''
(r/'REPORT_KO.md').write_text(report)
print(json.dumps({'families':len(families),'new_eligible_comparisons':len(comparable),'new_not_comparable':len(miss),'dopri_cost_losses':len(loss),'process':process},ensure_ascii=False)[:4000])
