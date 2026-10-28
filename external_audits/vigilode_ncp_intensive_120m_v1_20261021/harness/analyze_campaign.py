import json, pathlib, statistics, math, hashlib, shutil, re, os
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

p=pathlib.Path('/home/cosmosapjw/vigilode-reaudit-1qrziM3n/receipts/ncp-intensive-return-20261021T062109Z')
J=lambda x: json.loads((p/x).read_text())
def rows(fn): return [json.loads(s) for s in (p/fn).read_text().splitlines() if s.strip() and not s.strip().startswith('\\n')]
def dump(name,obj): (p/name).write_text(json.dumps(obj,indent=2,ensure_ascii=False,allow_nan=False)+'\n')
def med(xs): return statistics.median(xs) if xs else None
def pct(xs,q): return float(np.percentile(xs,q)) if xs else None
def errpass(r):
 a=r.get('accuracy')
 if isinstance(a,dict): return a.get('accuracy_pass_scaled_le_1')
 return None

dump('SCREENING_COVERAGE.json',J('COVERAGE.json'))
dump('DEPENDENCIES_INITIAL.json',J('DEPENDENCIES.json'))
# Recover exact result streams without changing originals.
conf=rows('CONFIRMATION_RESULTS.jsonl'); rotv2=rows('CONFIRMATION_ROTATION_V2.jsonl')
(p/'CONFIRMATION_RESULTS_RECOVERED.jsonl').write_text(''.join(json.dumps(x,separators=(',',':'),allow_nan=False)+'\n' for x in conf+rotv2))
# Derive Julia screening-throughput exact accuracy for its existing shared-grid values.
th=rows('THROUGHPUT_RESULTS.jsonl');
def exact_rotation(states,times,w=10):
 if not states or not times: return None
 es=[]
 for t,y in zip(times,states):
  ex=[math.cos(w*t),math.sin(w*t)]
  for a,b in zip(y,ex): es.append(abs(a-b)/(1e-10+1e-6*abs(b)))
 return max(es) if es else None
threc=[]
for r in th:
 z=dict(r)
 if z.get('solver')=='Julia OrdinaryDiffEq' and z.get('accuracy') is None:
  if z.get('case_id')=='C-rotation-w10':
   me=exact_rotation(z.get('states'), [i*.01 for i in range(len(z.get('states') or []))])
   z['accuracy']={'max_scaled_error':me,'accuracy_pass_scaled_le_1':(me<=1 if me is not None else None),'reference':'analytic exact rotation; shared grid reconstructed from frozen request'}
  else:
   # Derive accuracy using saved request exact values where possible by mapping screening case definitions; Julia throughput has no independent accuracy field.
   # Do not manufacture reference arrays: leave unavailable for generic nonlinear/high-dimensional cases.
   z['accuracy']={'max_scaled_error':None,'accuracy_pass_scaled_le_1':None,'reference':'not emitted by throughput parent; see confirmation/work-precision cohorts'}
 threc.append(z)
(p/'THROUGHPUT_RESULTS_RECOVERED.jsonl').write_text(''.join(json.dumps(x,separators=(',',':'),allow_nan=False)+'\n' for x in threc))
# Corrected rotation throughput data block metadata only in derived file.
trv=rows('THROUGHPUT_ROTATION_V2.jsonl')
for x in trv: x['block']='rotation_concurrent_3_worker_adapter_recheck'; x['source_block_metadata_correction']='worker copied main 28-worker label; actual 3 workers CPUs 12-14'
(p/'THROUGHPUT_ROTATION_V2_RECOVERED.jsonl').write_text(''.join(json.dumps(x,separators=(',',':'),allow_nan=False)+'\n' for x in trv))
# Recover rotation tolerance aggregation missing the Python raw subset.
rt=rows('ROTATION_TOLERANCE_RESULTS.jsonl')+rows('ROTATION_TOLERANCE_PY_RAW.jsonl')
(p/'ROTATION_TOLERANCE_RESULTS_RECOVERED.jsonl').write_text(''.join(json.dumps(x,separators=(',',':'),allow_nan=False)+'\n' for x in rt))
# recover diffsol newline framing; rows already normalized in derived source
# campaign coverage by phases; distinguish adapter/protocol issues from solver status.
coverage={
 'planned_measurements':1070,
 'completed_measurements':1070,
 'execution_failed_measurements':0,
 'timed_out_measurements':0,
 'accuracy_passed':None,
 'accuracy_failed':None,
 'accuracy_unknown':None,
 'not_run':0,
 'phase_counts':{
  'screening':{'planned':216,'completed':216,'warmups':72},
  'confirmation_original':{'planned':147,'completed_processes':147,'warmups':42,'adapter_invalid_rotation_rows':21},
  'confirmation_rotation_corrected':{'planned':21,'completed':21,'warmups':6},
  'julia_confirmation':{'planned':49,'completed':49,'warmups':14},
  'throughput_main':{'planned':196,'completed_measurements':196,'warmups':56,'workers':28,'completed_workers':28,'adapter_invalid_python_rotation_measurements':14},
  'throughput_rotation_corrected':{'planned':21,'completed':21,'warmups':6,'workers':3},
  'diffsol_confirmation':{'planned':70,'completed':70,'warmups':20,'source_stream_serializer_recovery_required':True},
  'decay_work_precision':{'planned':140,'completed':140,'warmups':40},
  'rotation_work_precision':{'planned':140,'completed':140,'warmups':40,'original_parent_summary_undercount':True},
  'nonlinear_confirmation':{'planned':70,'completed':70,'warmups':20,'independent_accuracy_reference':'UNKNOWN'}
 },
 'unique_problem_cases_screened':24,
 'screening_accuracy_unknown_case_ids':['B-robertson-short','B-vdp-mu20'],
 'host_telemetry_samples':len(rows('HOST_TELEMETRY.jsonl')),
 'overall_status':'COMPLETED_WITH_SCOPED_NUMERICAL_FAILURES_AND_ADAPTER/AGGREGATOR_EVIDENCE_ISSUES'
}
# calculated counts (observations can overlap cohorts, no unique global accuracy denominator)
dump('COVERAGE.json',coverage)
# Rebuild dependency report with facts derived from receipts (initial preserved as-is too).
deps=J('DEPENDENCIES_INITIAL.json')
deps.update({'ncp_host':'s1a0e5abed09','rust':'1.94.1','python':'3.12.3','numpy':'1.26.4','scipy':'1.11.4','mpmath':'1.2.1','pytest':'7.4.4','julia':'1.12.7','ordinarydiffeq':'7.8.1','ordinarydiffeqbdf':'2.4.13','ordinarydiffeqfirk':'2.8.9','scimlbase':'3.58.0','diffsol':'0.17.1','native_libraries':'GCC/GFortran and BLAS available; SUNDIALS 6.4.1 installed but CVODE not used','status':'READY_FOR_EXECUTION_AND_COMPARATORS_COMPILED','scope_note':'No full VigilODE workspace test suite was run; builds/smokes are recorded separately.'})
dump('DEPENDENCIES.json',deps)
# Accuracy/latency/RSS groups from valid confirmation sources.
valid=conf+rotv2
julia=rows('JULIA_RESULTS.jsonl')
# Map exact medians, accuracy count, RSS for confirmation scopes.
def summary_group(rs,solver,case):
 a=[r for r in rs if not r.get('warmup') and r.get('solver')==solver and r.get('case_id')==case and r.get('status')=='OK']
 times=[]; passes=[]; rss=[]
 for r in a:
  t=(r.get('timing') or {}).get('elapsed_ns')
  if t is not None: times.append(float(t)/1e9)
  elif r.get('elapsed_seconds') is not None: times.append(float(r['elapsed_seconds']))
  elif r.get('process_wall_seconds') is not None: times.append(float(r['process_wall_seconds']))
  ep=errpass(r)
  if ep is None and isinstance(r.get('accuracy'),dict): ep=r['accuracy'].get('accuracy_pass_scaled_le_1')
  passes.append(ep)
  rr=r.get('worker_peak_rss_kb',r.get('child_peak_rss_kb',r.get('peak_rss_kb')))
  if rr is not None: rss.append(float(rr))
 return {'n':len(a),'median_s':med(times),'q1_s':pct(times,25),'q3_s':pct(times,75),'accuracy_pass':sum(x is True for x in passes),'accuracy_fail':sum(x is False for x in passes),'accuracy_unknown':sum(x is None for x in passes),'median_peak_rss_kib':med(rss),'times':times,'rss':rss,'rows':a}
case_specs=[
 ('A-decay-n8','A nonstiff smooth; n=8, rtol=1e-6'),('B-stiffdiag-n32-L1000','B diagonal stiff; n=32, spectral ratio about 1000, rtol=1e-6'),('C-rotation-w10','C rotation omega=10; rtol=1e-6, atol=1e-10'),('D-damping-z1','D critical damping zeta=1; rtol=1e-6'),('E-nonnormal-transient','E triangular nonnormal exact transient; rtol=1e-6'),('F-heat-n128','F periodic heat Fourier mode n=128; rtol=1e-6'),('G-mixed-nearzero','G mixed scale / near-zero; rtol=1e-6')]
solvers=['vigilode','scipy_vode_adams','scipy_vode_bdf','Julia OrdinaryDiffEq']
# diffsol confirmation cases separate
for cid,label in case_specs:
 for solver in solvers:
  ss=summary_group(julia if solver=='Julia OrdinaryDiffEq' else valid,solver,cid)
  print('CONF',cid,solver,ss['n'],ss['median_s'],ss['accuracy_pass'],ss['accuracy_fail'],ss['accuracy_unknown'],ss['median_peak_rss_kib'])
# Work precision group stats.
tol=rows('TOLERANCE_RESULTS.jsonl')
rot_tol=rt
wp_rows=tol+rot_tol
wp={}
for (case,solver,backend,rtol) in sorted({(r.get('case_id'),r.get('solver'),r.get('backend'),r.get('rtol')) for r in wp_rows},key=lambda z:(z[0] or '',z[1] or '',z[3] or 0)):
 rr=[r for r in wp_rows if r.get('case_id')==case and r.get('solver')==solver and r.get('backend')==backend and r.get('rtol')==rtol and not r.get('warmup')]
 times=[float(r.get('elapsed_seconds') if r.get('elapsed_seconds') is not None else r.get('process_wall_seconds')) for r in rr if (r.get('elapsed_seconds') is not None or r.get('process_wall_seconds') is not None)]
 aps=[errpass(r) for r in rr]
 errs=[r.get('accuracy',{}).get('max_scaled_error') for r in rr if isinstance(r.get('accuracy'),dict) and r['accuracy'].get('max_scaled_error') is not None]
 wp[f'{case}|{solver}|{backend}|{rtol:g}']={'n':len(rr),'median_s':med(times),'q1_s':pct(times,25),'q3_s':pct(times,75),'accuracy_pass':sum(x is True for x in aps),'accuracy_fail':sum(x is False for x in aps),'accuracy_unknown':sum(x is None for x in aps),'median_max_scaled_error':med([float(x) for x in errs]),'min_max_scaled_error':min(errs) if errs else None,'max_max_scaled_error':max(errs) if errs else None}
dump('WORK_PRECISION_SUMMARY.json',wp)
# compute exact accuracy classification totals separately by cohort
wp_acc={'decay_n128':J('TOLERANCE_SUMMARY.json'),'rotation_original_partial':J('ROTATION_TOLERANCE_SUMMARY.json'),'rotation_recovered':{'measurement_rows':sum(v['n'] for k,v in wp.items() if k.startswith('C-rotation-w10|')),'accuracy_pass':sum(v['accuracy_pass'] for k,v in wp.items() if k.startswith('C-rotation-w10|')),'accuracy_fail':sum(v['accuracy_fail'] for k,v in wp.items() if k.startswith('C-rotation-w10|'))}}
dump('WORK_PRECISION_COVERAGE.json',wp_acc)
# nonlinear invariants summarize across nonwarmup measurements
nl=rows('NONLINEAR_CONFIRMATION_RESULTS.jsonl')
nl_summary={}
for case in sorted({r['case_id'] for r in nl}):
 rs=[r for r in nl if r['case_id']==case and not r.get('warmup')]
 inv=[r.get('invariants') for r in rs if r.get('invariants')]
 nl_summary[case]={'measurement_rows':len(rs),'execution_ok':sum(r.get('status')=='OK' for r in rs),'accuracy_status':'UNKNOWN_NO_INDEPENDENT_REFERENCE','minimum_component_min':min((x.get('minimum_component') for x in inv if x.get('minimum_component') is not None),default=None),'max_abs_mass_residual_max':max((x.get('max_abs_mass_residual') for x in inv if x.get('max_abs_mass_residual') is not None),default=None),'positivity_all':all(x.get('positivity_pass_nonnegative') is True for x in inv) if inv else None,'max_state_abs_max':max((x.get('max_state_abs') for x in inv if x.get('max_state_abs') is not None),default=None),'peak_runtime_s':max((r.get('process_wall_seconds') or 0 for r in rs),default=None)}
dump('NONLINEAR_INVARIANTS_SUMMARY.json',nl_summary)
# Decision records. Evidence is empirical, scope-specific. No global promotion without paired confirmation uncertainty.
dec=[]
for cid,label in case_specs:
 for solver in ['vigilode','scipy_vode_adams','scipy_vode_bdf','Julia OrdinaryDiffEq','diffsol']:
  src=julia if solver=='Julia OrdinaryDiffEq' else (rows('DIFFSOL_RESULTS_RECOVERED.jsonl') if solver=='diffsol' else valid)
  backend={'vigilode':'native-rodas5p-fast','scipy_vode_adams':'VODE Adams','scipy_vode_bdf':'VODE BDF','Julia OrdinaryDiffEq':('Tsit5' if cid.startswith(('A-','C-')) else 'Rodas5P'),'diffsol':('Tsit45' if cid.startswith(('A-','C-')) else 'BDF')}.get(solver)
  ss=summary_group(src,solver,cid)
  if ss['n']==0:
   d={'solver':solver,'backend':backend,'scope':{'case_id':cid,'description':label,'rtol':1e-6},'verdict':'hold','evidence_level':'empirical','accuracy_pass':None,'speed_ratio_definition':'baseline_time / candidate_time','speed_ratio':None,'uncertainty_interval':None,'limitations':['No valid confirmation measurement for this solver/case combination.'],'evidence_paths':['RAW_RESULTS.jsonl','CONFIRMATION_RESULTS.jsonl'],'next_action':'Run a corrected adapter confirmation only if this scope remains decision-relevant.'}
  else:
   passed=ss['accuracy_pass']; failed=ss['accuracy_fail']; unknown=ss['accuracy_unknown']
   # diffsol rows use key accuracy schema too; timing uses elapsed_seconds.
   if solver=='diffsol':
    pass
   if unknown or passed+failed==0: verdict='hold'; acc=None
   elif failed>0 and passed==0: verdict='prune'; acc=False
   elif passed==ss['n'] and ss['n']>=7: verdict='hold'; acc=True
   else: verdict='hold'; acc=(passed>0 and failed==0)
   d={'solver':solver,'backend':backend,'scope':{'case_id':cid,'description':label,'rtol':1e-6,'dimension':next((x.get('dimension') for x in ss['rows']),None)},'verdict':verdict,'evidence_level':'empirical','accuracy_pass':acc,'measurements':ss['n'],'accuracy_pass_count':passed,'accuracy_fail_count':failed,'accuracy_unknown_count':unknown,'median_latency_seconds':ss['median_s'],'latency_iqr_seconds':[ss['q1_s'],ss['q3_s']],'median_peak_rss_kib':ss['median_peak_rss_kib'],'speed_ratio_definition':'baseline_time / candidate_time','speed_ratio':None,'uncertainty_interval':None,'limitations':['One exact case and tolerance only.','Confirmation arms were sequential randomized, not paired; no speed promotion.','Wall time is diagnostic and host exclusivity/frequency lock are unestablished.','No full workspace tests or broad certification.'],'evidence_paths':['CONFIRMATION_RESULTS_RECOVERED.jsonl','JULIA_RESULTS.jsonl','DIFFSOL_RESULTS_RECOVERED.jsonl'],'next_action':('Accuracy failure reproduced in this exact case/budget; retain scoped prune only, investigate method/tolerance behavior before broader claims.' if verdict=='prune' else 'Hold pending broader paired confirmation, adequate reference, and scope expansion.')}
  dec.append(d)
# Replace diffsol accuracy unavailable issue stats function is compatible.
dump('DECISIONS.json',{'protocol':'vigilode_ncp_intensive_120m_v1','broad_production':'HOLD','decision_count':len(dec),'verdict_counts':{k:sum(d['verdict']==k for d in dec) for k in ['promote','prune','hold','optimization_possible','specialized_for_specific_cases_only']},'decisions':dec,'policy_note':'Scoped empirical verdicts only. No promote or specialization based on unpaired/case-limited timing; scoped prune applies only to reproduced accuracy failure at stated exact case and tolerance.'})
# Pareto points, where exact accuracy is available; use latency and RSS, retain all measurements as points plus nondominated medians.
pareto={}
for cid,label in case_specs:
 pts=[]
 candidates=[('vigilode','native-rodas5p-fast',valid),('SciPy VODE Adams','VODE Adams',valid),('SciPy VODE BDF','VODE BDF',valid),('Julia OrdinaryDiffEq',None,julia),('diffsol','diffsol',rows('DIFFSOL_RESULTS_RECOVERED.jsonl'))]
 for solver,backend,src in candidates:
  if solver=='vigilode': key='vigilode'
  elif solver.startswith('SciPy'): key='scipy_vode_adams' if 'Adams' in solver else 'scipy_vode_bdf'
  else: key=solver
  ss=summary_group(src,key,cid)
  if ss['n'] and ss['accuracy_pass']==ss['n'] and ss['median_s'] is not None and ss['median_peak_rss_kib'] is not None:
   pts.append({'solver':solver,'backend':backend,'latency_s':ss['median_s'],'peak_rss_kib':ss['median_peak_rss_kib'],'accuracy_pass':True})
 frontier=[]
 for x in pts:
  dominated=any((y['latency_s']<=x['latency_s'] and y['peak_rss_kib']<=x['peak_rss_kib'] and (y['latency_s']<x['latency_s'] or y['peak_rss_kib']<x['peak_rss_kib'])) for y in pts if y is not x)
  if not dominated: frontier.append(x)
 pareto[cid]={'description':label,'points':pts,'frontier':frontier,'limitations':['Only matched exact-confirmation measurements with memory observed; missing RSS/corrected cases excluded.','No timing authority; descriptive empirical Pareto only.']}
dump('PARETO_FRONTIERS.json',pareto)
# optimization measured bottlenecks, no speed promises
opt={'protocol':'vigilode_ncp_intensive_120m_v1','items':[
 {'priority':'high','scope':'component adapter + periodic heat Fourier mode n=128, rtol=1e-6','evidence':{'vigilode_median_s':0.014555132,'vigilode_peak_rss_kib':13824,'julia_rodas5p_median_s':0.009195117,'julia_peak_rss_kib':1010336,'extra_rhs_calls':5376,'counted_rhs_calls':336,'accepted_steps':42,'jacobian_builds':42,'jacobian_reuses':0},'measured_bottleneck':'Benchmark adapter forms dense Jacobian by basis JVP; periodic heat JVP calls RHS, causing 5376 derivative RHS calls that are separately marked excluded from solver counters. VIG is slower than Julia in this case but uses far less measured RSS.','target':'component_benchmark_support::Case::jac_into and periodic_heat jvp adapter path','action':'Compare a structured/banded or analytic Jacobian route already supported by the production interface; separately account derivative RHS in adapter output.','verification':'same exact n=128, tolerance, confirmation repetitions, error and complete timing/RSS/work ledger; no predicted speedup.'},
 {'priority':'high','scope':'rapid oscillation omega=10, fixed componentwise rtol=1e-6 atol=1e-10','evidence':{'vigilode_max_scaled_error':5156.614120926653,'all_four_tolerance_levels_failed':True},'measured_bottleneck':'Accuracy fails strongly near component zeros for all four tested tolerances; larger rtol refinements were not used.','target':'accuracy controller/scaling policy investigation only; no change in this campaign','action':'Inspect scaled error allocation and phase/amplitude error separately under frozen tolerances; preserve refusal/accuracy failure until independently validated.','verification':'analytic phase+amplitude and componentwise shared-grid checks at existing protocol without loosening tolerance.'},
 {'priority':'medium','scope':'accuracy comparison evidence design','evidence':{'confirmation_repetitions':7,'arm_order':'randomized sequential; unpaired'},'measured_bottleneck':'Current repetitions do not support paired speed-ratio uncertainty.','target':'next measurement harness only','action':'Interleave deterministically seeded matched blocks and record block IDs, preserving the same solver options and CPU affinity.','verification':'predeclared paired bootstrap/interval and matched accuracy before any speed verdict.'}
 ],'no_implementation_performed':True}
dump('OPTIMIZATION_PLAN.json',opt)
# Figures
figdir=p/'figures'; figdir.mkdir(exist_ok=True)
# Work precision by case
fig,ax=plt.subplots(figsize=(9,6))
colors={'scipy_vode_adams':'tab:orange','scipy_vode_bdf':'tab:red','julia_ordinarydiffeq':'tab:green','diffsol':'tab:purple','vigilode':'tab:blue'}
for case in ['A-decay-n128','C-rotation-w10']:
 for solver in sorted(set(k.split('|')[1] for k in wp if k.startswith(case+'|'))):
  xs=[];ys=[];passes=[]
  for k,v in wp.items():
   c,s,b,t=k.split('|')
   if c==case and s==solver and v['median_s'] and v['median_max_scaled_error'] is not None:
    xs.append(v['median_s']);ys.append(v['median_max_scaled_error']);passes.append(v['accuracy_pass']>0)
  if xs: ax.plot(xs,ys,'o-',label=f'{case}: {solver}')
ax.axhline(1,color='black',linestyle='--',linewidth=1);ax.set_xscale('log');ax.set_yscale('log');ax.set_xlabel('median solver time (s; diagnostic)');ax.set_ylabel('median max scaled error');ax.set_title('Work precision: analytic decay and rotation');ax.legend(fontsize=7);ax.grid(True,which='both',alpha=.25);fig.tight_layout();fig.savefig(figdir/'work_precision.png',dpi=150);plt.close(fig)
# Scaling one actual 28-worker throughput point only
fig,ax=plt.subplots(figsize=(7,4));ax.bar(['independent throughput\n28 workers'],[28],color='steelblue');ax.axhline(64,color='gray',linestyle='--',label='64 logical CPUs reported');ax.set_ylabel('simultaneous single-thread workers');ax.set_title('Throughput block; internal scaling NOT RUN');ax.text(0,28.5,'18.41 s batch',ha='center');ax.legend();fig.tight_layout();fig.savefig(figdir/'scaling.png',dpi=150);plt.close(fig)
# Memory medians confirmed cohorts
fig,ax=plt.subplots(figsize=(10,5));
labels=[];vals=[]
for cid,_ in case_specs:
 for solver in ['vigilode','scipy_vode_adams','scipy_vode_bdf','Julia OrdinaryDiffEq']:
  ss=summary_group(julia if solver=='Julia OrdinaryDiffEq' else valid,solver,cid)
  if ss['median_peak_rss_kib'] is not None:
   labels.append(cid.replace('B-stiffdiag-n32-L1000','B-stiff').replace('F-heat-n128','F-heat')+'\n'+solver.replace('scipy_vode_','VODE ').replace('Julia OrdinaryDiffEq','Julia'))
   vals.append(ss['median_peak_rss_kib']/1024)
ax.bar(range(len(vals)),vals);ax.set_xticks(range(len(vals)),labels,rotation=90,fontsize=6);ax.set_ylabel('median reported peak RSS (MiB)');ax.set_title('Confirmation RSS; worker/process scope as recorded');ax.grid(axis='y',alpha=.25);fig.tight_layout();fig.savefig(figdir/'memory.png',dpi=150);plt.close(fig)
# failure coverage by family from screening VIG/external
scr=rows('RAW_RESULTS.jsonl'); fams=sorted(set(x.get('family','unknown') for x in scr if not x.get('warmup'))); names=['vigilode','scipy_vode_adams','scipy_vode_bdf'];mat=np.zeros((len(names),len(fams)),int)
for i,solver in enumerate(names):
 for j,fam in enumerate(fams):
  rr=[x for x in scr if not x.get('warmup') and x.get('solver')==solver and x.get('family')==fam]
  mat[i,j]=sum(errpass(x) is False for x in rr)
fig,ax=plt.subplots(figsize=(9,4));im=ax.imshow(mat,aspect='auto',cmap='Reds');ax.set_xticks(range(len(fams)),fams,rotation=30,ha='right');ax.set_yticks(range(len(names)),names);ax.set_title('Screening accuracy-failure counts (not execution failures)');fig.colorbar(im,ax=ax,label='failed measurements');fig.tight_layout();fig.savefig(figdir/'failure-coverage.png',dpi=150);plt.close(fig)
# Generate complete harness digest after creating analysis script copy; never self hash.
shutil.copy2('/tmp/build_vigilode_intensive_report.py',p/'harness'/'analyze_campaign.py')
entries=[]
for f in sorted((p/'harness').rglob('*')):
 if f.is_file() and f.name not in ['HARNESS_SHA256SUMS.json','HARNESS_SHA256SUMS.txt']:
  h=hashlib.sha256(f.read_bytes()).hexdigest(); entries.append({'path':str(f.relative_to(p)),'sha256':h,'bytes':f.stat().st_size})
for f in sorted(figdir.glob('*.png')):
 entries.append({'path':str(f.relative_to(p)),'sha256':hashlib.sha256(f.read_bytes()).hexdigest(),'bytes':f.stat().st_size})
dump('HARNESS_SHA256SUMS.json',{'algorithm':'SHA256','files':entries,'scope':'harness scripts/configs/requests/patches/binaries and generated figures; excludes manifest self-hash'})
# preserve telemetry malformed stop file and valid recovered copy
raw=(p/'HOST_TELEMETRY_STOP.json').read_text()
try: json.loads(raw)
except Exception:
 clean=raw.rstrip('\\n\n')
 obj=json.loads(clean); dump('HOST_TELEMETRY_STOP_RECOVERED.json',obj)
# Host block correction: create notes and don't alter original initial observation
# Report concise but detailed, cite paths relatively inside the artifact.
report='''# VigilODE NCP intensive benchmark — 결과 보고\n\n## 범위와 실행\n\n프로토콜 `vigilode_ncp_intensive_120m_v1`; source commit `de4c10a9f41321beef2313a3d7fb503bab35d714`, tree `42c3917d5303568ca9169c0465728b783af8f45c`. NCP `s1a0e5abed09` (Xeon Gold 5220, KVM): 1 socket, 64 reported cores/logical CPUs, 1 thread/core, 1 NUMA node, cpuset 0–63, cgroup CPU/memory quota 없음, RAM 135.05 GB, 시작 시 available 93.67 GB, swap 없음. exclusivity/clock lock는 미확인. 시작 `2026-10-21T06:21:09Z`, 종료 `2026-10-21T07:15:14Z`, 경과 3244.59 s (54m04.6s), 120분 제한 미도달.\n\nRust 1.94.1와 pinned Python/Julia/diffsol toolchain에서 native benchmark adapter와 외부 adapters를 빌드했다. Julia의 최초 smoke API 오류 및 diffsol version/API 빌드 실패, 결과 framing/aggregator 오류는 모두 원본 로그로 보존하고 수정된 isolated adapter/recovery 파생물을 별도 보관했다. Cargo.lock과 solver source는 바꾸지 않았다. full workspace test suite는 이번 campaign에서 실행하지 않았다. Package verifier PASS는 source/evidence identity·coverage 검사이며 solver 정확성 증거가 아니다.\n\n## 측정 구성과 제한\n\nScreening은 24 cases, 216 measurements/72 warmups, 7개 문제군을 포함한다. Confirmation·work-precision·throughput·nonlinear 단계를 포함해 `COVERAGE.json`의 phase별 계획/완료 수를 확인한다. 28-worker throughput block은 28개의 독립 1-thread worker를 CPUs 12–39에 pin하고 18.41 s 동안 실행했다. 확인/precision blocks는 주로 CPU 8 단일 affinity였다. 초기 screening은 unpinned였다. 따라서 latency는 diagnostic-only이며 screening/throughput/serial을 한 순위로 합치지 않는다. 실제 solver 내부 1/2/4/8/16/32 scaling은 NOT_RUN; 테스트된 경로에서 내부 병렬 backend를 확인하지 못했다.\n\n비교군: SciPy 1.11.4 VODE Adams/BDF (float64, process-isolated), diffsol 0.17.1 Tsit45/BDF + dense NalgebraLU, Julia 1.12.7 OrdinaryDiffEq 7.8.1 (Tsit5/Rodas5P, dense default linear path), VigilODE native `rodas5p-fast`. CVODE/RADAU5는 실행하지 않았다.\n\n## 정확도와 성능 관측\n\n정확한 해석적/검증된 제조해가 있는 scoped confirmation은 `CONFIRMATION_RESULTS_RECOVERED.jsonl`, `JULIA_RESULTS.jsonl`, `DIFFSOL_RESULTS_RECOVERED.jsonl` 및 tolerance 결과를 근거로 한다. 7회 confirmation의 정확도 통과는 해당 case/tolerance에만 해당한다. 반복 arm은 paired/interleaved가 아니어서 speed ratio confidence interval을 산출하지 않았고, speed 기반 promotion은 하지 않는다.\n\n- Nonstiff decay n=8: VigilODE 7/7 정확도 통과, median 0.161 ms; Julia Tsit5 7/7, 0.0311 ms; VODE Adams 7/7, 0.431 ms; BDF 0/7.\n- Stiff diagonal n=32, L=1000: VigilODE 7/7, 1.024 ms; Julia Rodas5P 7/7, 1.059 ms; VODE Adams/BDF 0/7. 이 비교군의 차이는 practical 10% 이내이며 paired uncertainty가 없어 승격 근거가 아니다.\n- Rotation omega=10: 수정 adapter로 모든 arm이 실행됐지만 tested componentwise rtol 1e-6 / atol 1e-10에서 모두 실패. VigilODE max scaled error 약 5156.6. Work precision rtol 1e-4…1e-10 네 점 모두 통과 0/140. 원래 adapter 회전 branch 누락은 환경/adapter 실패로 보존하며 수치 실패와 합치지 않았다.\n- Damping zeta=1: VigilODE/Julia 7/7 pass; VODE Adams/BDF fail.\n- Nonnormal transient: VigilODE/Julia 7/7 pass; VODE Adams/BDF fail.\n- Structured heat n=128: VigilODE, Julia, VODE BDF, diffsol BDF 모두 7/7 pass. VigilODE median 14.56 ms, ~13.5 MiB peak RSS; Julia Rodas5P 9.20 ms, ~987 MiB; VODE BDF 252.4 ms, ~61.9 MiB; diffsol BDF 약 4.09 ms. RSS 경계는 각 프로세스/worker가 보고한 peak이며 process-tree 또는 OS 상한으로 해석하지 않는다. VigilODE 측정은 이 케이스에서 Julia보다 느리고 메모리 footprint는 작다.\n- Mixed near-zero: VigilODE/Julia 7/7 pass, VODE Adams/BDF fail.\n- Robertson / Van der Pol mu=20: solver runs completed, positivity/mass or state descriptors recorded, 그러나 independent numerical reference 없음. 정확도 UNKNOWN/HOLD. Robertson sampled solutions nonnegative; max mass residual 8.9e-16. 이는 trajectory accuracy 증명이 아니다.\n- Screening-only families/parameters는 coverage와 accuracy pass/fail를 raw에 보존했다. Screening time은 unpinned이므로 성능 승격 근거로 사용하지 않는다.\n\nDecay n=128 work precision은 4 tolerance level, 5 arms, 140 measurements/40 warmups. aggregate 77 pass/63 fail. 회전 work-precision aggregator는 Python adapter 108행을 원본 summary에 빠뜨렸으며, 복구 집계는 140 measurements/40 warmups이고 0 pass/140이다. 원 summary/raw 모두 보존했다. diffsol 원본 JSONL은 literal `\\n` framing 때문에 파서가 실패했고, 원본 SHA와 recovered JSONL을 분리했다. 결과 실행은 70/70 측정행 모두 상태 OK다.\n\n## scoped 판정\n\n`DECISIONS.json`에는 solver/backend × exact case × rtol 1e-6의 판정이 있다. 반복된 exact-budget accuracy fail은 그 사례/설정에 한정해 `prune`로 기록했고, 그 밖의 정확도 통과 범위도 비교 폭·reference·paired uncertainty 부족으로 `hold`다. 어떠한 broad production 승격도 없다. `specialized_for_specific_cases_only`는 이 데이터만으로도 아직 부여하지 않았다. heat n=128은 VigilODE의 낮은 관측 RSS와 느린 runtime이 함께 보여 scoped resource tradeoff 후보이나, 한 구조/크기뿐이다.\n\n`PARETO_FRONTIERS.json`은 RSS와 latency가 모두 관측된 confirmation median만의 기술적 Pareto 요약이다. timeout/unknown accuracy를 제거해 성공률 분모를 감추는 용도가 아니다. Work precision, scaling, memory, failure-coverage 그림은 `figures/` 아래 있다.\n\n## HOLD 및 최적화 근거\n\n1. Heat adapter는 dense Jacobian을 basis JVP 반복으로 만들고, periodic heat JVP가 RHS를 호출한다. n=128에서 별도 미계상 derivative work 5,376 RHS 호출, 계상된 336 RHS calls, 42 step/Jacobian builds, Jacobian reuse 0회가 관측됐다. 이건 benchmark adapter 경로의 측정 사실이며 제품 solver 전반의 병목으로 확대하지 않는다. 이미 지원되는 structured/analytic Jacobian 경로와 adapter accounting을 비교할 수 있다.\n2. Rotation의 근영역 componentwise scaled error가 모든 solver에서 budget을 초과했다. phase/amplitude와 componentwise 지표를 함께 추적하되 tolerance를 완화하지 않고 원인을 조사해야 한다.\n3. confirmation arms는 paired block이 아니므로 다음 속도 판정은 deterministic interleaving과 사전등록된 paired interval이 필요하다.\n\n## 상태\n\nPackage: PASS (identity/coverage only). Builds/adapters/smokes: 성공 기록됨. 전체 workspace tests: NOT_RUN. Benchmarks: campaign 완료, 여러 exact scopes accuracy fail. Gate: NOT_RUN (해당 frozen promotion reader는 다른 프로토콜용). broad production: HOLD. 실패·unknown·adapter/aggregator 문제는 원본과 recovered 파일을 나누어 보존했다. 상세 counts는 `COVERAGE.json`, decision rows는 `DECISIONS.json`을 본다.\n'''
(p/'REPORT_KO.md').write_text(report)
repro='''# Reproduction and artifact map\n\nProtocol: `vigilode_ncp_intensive_120m_v1`. Source commit/tree are fixed in `IDENTITY.json`. The original NCP checkout remains at `/root/vigilode-reaudit-20261021T0549Z/checkout`; campaign directory was `/dev/shm/vigilode_ncp_intensive_120m_v1_20261021T062109Z`. The returned local copy is the current artifact directory. Do not re-run into this directory or overwrite raw streams.\n\n## Reproduce\n\nSee `CONFIG.json` and `harness/`. Tool versions and installed dependencies are in `DEPENDENCIES.json`; host topology and initial pinning caveats are in `HOST.json`, `HOST.lscpu.json`, `HOST.cgroup-limits.txt`, `HOST.memory.txt`, and disk snapshot files. Exact commands and worker receipts are preserved in `logs/` and `harness/`. The campaign was executed once, with screening, confirmation, throughput, tolerance, nonlinear, and recovery blocks. Raw files retain their initial bytes. Files ending `_RECOVERED` are derived parsing/aggregation products; original malformed/partial sources are preserved and their issue is documented in `DIFFSOL_RECOVERY.json` / `ROTATION_TOLERANCE_SUMMARY.json`.\n\n## Identity / validation\n\n`PACKAGE_VERIFY.json` status PASS means file identity and coverage checks only. Source commit/tree: `FINAL_COMMIT.txt`, `FINAL_TREE.txt`; clean status snapshot: `FINAL_SOURCE_STATUS.txt`. No source or Cargo.lock changes were made by this campaign. The experiment-specific diffsol scratch crate is isolated under `harness/diffsol-adapter/`.\n\n## Main evidence\n\n- Screening raw: `RAW_RESULTS.jsonl`; `SCREENING_COVERAGE.json`.\n- Confirmation raw plus corrected rotation branch: `CONFIRMATION_RESULTS.jsonl`, `CONFIRMATION_ROTATION_V2.jsonl`, derived concatenation `CONFIRMATION_RESULTS_RECOVERED.jsonl`.\n- Julia: `JULIA_RESULTS.jsonl`; diffsol original/recovered plus `DIFFSOL_RECOVERY.json`.\n- Throughput original and correction: `THROUGHPUT_*`; tolerance original and recovered rotation: `TOLERANCE_RESULTS.jsonl`, `ROTATION_TOLERANCE_*`.\n- Nonlinear runs/invariants: `NONLINEAR_CONFIRMATION_*`.\n- Host samples: `HOST_TELEMETRY.jsonl`; stop-file parse recovery kept separate.\n- Human/machine analysis: `REPORT_KO.md`, `SUMMARY.json`, `DECISIONS.json`, `OPTIMIZATION_PLAN.json`, `PARETO_FRONTIERS.json`, `COVERAGE.json`, `WORK_PRECISION_SUMMARY.json`, `figures/`.\n- `GATE.json` is intentionally absent: this campaign protocol is not input to the frozen production promotion gate. broad production remains HOLD.\n\nRaw hashes are in `RAW_SHA256SUMS.txt`; harness, adapter, request, patch, binary, and figure digests are in `HARNESS_SHA256SUMS.json`. Hash manifests do not hash themselves.\n'''
(p/'REPRODUCE.md').write_text(repro)
# Final summary machine readable.
summary={'protocol':'vigilode_ncp_intensive_120m_v1','source_commit':'de4c10a9f41321beef2313a3d7fb503bab35d714','source_tree':'42c3917d5303568ca9169c0465728b783af8f45c','host':'s1a0e5abed09','started_at_utc':'2026-10-21T06:21:09Z','finished_at_utc':'2026-10-21T07:15:14Z','elapsed_seconds':3244.59,'deadline_seconds':7200,'deadline_hit':False,'broad_production':'HOLD','package_verification':'PASS_IDENTITY_COVERAGE_ONLY','source_clean':True,'workspace_tests':'NOT_RUN','benchmarks':'COMPLETED_WITH_SCOPED_ACCURACY_FAILURES','internal_scaling':'NOT_RUN_NO_VERIFIED_INTERNAL_PARALLEL_BACKEND','coverage':coverage,'promotion_gate':'NOT_RUN_PROTOCOL_MISMATCH_BY_DESIGN','artifact_integrity_issues':['original diffsol JSONL newline framing malformed; recovered derived stream preserved separately','rotation work-precision parent summary omitted Python raw subset; recovered aggregation preserves original','initial rotation confirmation/throughput Python adapter omitted RHS; original attempts retained, corrected runs separate'], 'evidence_paths':{'raw':'RAW_RESULTS.jsonl','confirmation':'CONFIRMATION_RESULTS_RECOVERED.jsonl','work_precision':'WORK_PRECISION_SUMMARY.json','decisions':'DECISIONS.json','pareto':'PARETO_FRONTIERS.json','report':'REPORT_KO.md'},'limitations':['No full VigilODE workspace test suite.','Accuracy evidence is empirical and scope-limited; nonlinear exact reference unknown.','No paired timing interval; wall clock diagnostic only.','Host exclusivity, frequency lock, and physical isolation unestablished.','Broad production promotion remains HOLD.']}
dump('SUMMARY.json',summary)
# Hash all raw payloads (never mutate raw files).
rawnames=[]
for f in sorted(p.iterdir()):
 if f.is_file() and (f.name.endswith('.jsonl') or f.name.endswith('.log') or f.name.endswith('.txt') and ('SHA256' in f.name)):
  rawnames.append(f)
with (p/'RAW_SHA256SUMS.txt').open('w') as out:
 for f in rawnames: out.write(f"{hashlib.sha256(f.read_bytes()).hexdigest()}  {f.name}\n")
print('ARTIFACTS_WRITTEN',p)
print('decision_verdicts',J('DECISIONS.json')['verdict_counts'])
print('work_precision_groups',len(wp),'coverage',coverage['planned_measurements'])
