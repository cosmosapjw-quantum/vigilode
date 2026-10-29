#!/usr/bin/env python3
"""Read saved evidence only; never starts an integrator or edits its checkout."""
import pathlib,json,hashlib,tarfile,collections,math,re,subprocess
R=pathlib.Path(__file__).resolve().parent
W=pathlib.Path('/home/cosmosapjw/vigilode-nonstiff-source-WjxDXM')
def load(p):return json.loads(p.read_text())
def digest(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  for b in iter(lambda:f.read(1024*1024),b''):h.update(b)
 return h.hexdigest()
def archive_check(name):
 p=R/name;m=load(p/'RAW_MANIFEST.json');assert digest(p/m['archive'])==m['sha256'];count=0
 with tarfile.open(p/m['archive'],'r:gz') as t:
  members={x.name:x for x in t.getmembers()}
  assert len(members)==len(m['files'])
  for e in m['files']:
   assert members[e['path']].size==e['bytes'];assert hashlib.sha256(t.extractfile(e['path']).read()).hexdigest()==e['sha256'];assert digest(p/e['path'])==e['sha256'];count+=1
 receipts=load(p/'RECEIPTS.json')
 for e in receipts:
  for k in ['stdout','stderr']:
   assert digest(p/e[k])==e[k+'_sha256']
 return {'archive_sha256':m['sha256'],'archive_entries_verified':count,'raw_receipts_verified':len(receipts),'all_hashes_match':True}
source=load(R/'SOURCE_IDENTITY.json');execution=load(R/'EXECUTION_RECEIPT.json');main=load(R/'full-comparison/SUMMARY.json');ensemble=load(R/'ensemble-comparison/SUMMARY.json');plan=load(R/'full-comparison/PLAN.json');campaign=load(R/'full-comparison/CAMPAIGN.json')
assert main['source_commit']==ensemble['source_commit']==source['source_commit']==execution['source_commit']
assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=W,text=True).strip()==source['source_commit']
assert subprocess.check_output(['git','rev-parse','HEAD^{tree}'],cwd=W,text=True).strip()==source['source_tree']
assert not subprocess.check_output(['git','status','--porcelain=v1'],cwd=W,text=True)
input_count=0
for line in (R/'source-input-sha256.txt').read_text().splitlines():
 h,path=line.split(None,1);assert digest(W/path.strip())==h;input_count+=1
for name,h in execution['binary_sha256'].items():assert digest(R/'build/measurement/examples'/name)==h
checks={name:archive_check(name) for name in ['full-comparison','ensemble-comparison']}
counts={}
for filename in ['tests-kernel.log','tests-public-ensemble.log']:
 values=re.findall(r'test result: .*?\. (\d+) passed; (\d+) failed;', (R/filename).read_text());counts[filename]={'passed':sum(int(a) for a,b in values),'failed':sum(int(b) for a,b in values)}
assert sum(x['passed'] for x in counts.values())==25 and not sum(x['failed'] for x in counts.values())
rows={r['id']:r for r in main['rows']}
for c in main['common_error_comparisons']:
 if c['status']=='COMPARABLE':
  a,b=rows[c['explicit_config']],rows[c['comparator_config']]
  assert a['all_runs_successful'] and b['all_runs_successful']
  assert a['max_normalized_error']<=c['normalized_error_threshold'] and b['max_normalized_error']<=c['normalized_error_threshold']
  assert math.isclose(b['median_seconds']/a['median_seconds'],c['comparator_over_explicit_time'],rel_tol=1e-14)
 else:assert c['comparator_over_explicit_time'] is None
forbidden=['jacobian_builds','jvp_calls','jvp_vectors','ft_calls','mass_matvecs','linear_solves','direct_factorizations','direct_solve_calls','nonlinear_solves','nonlinear_iterations']
explicit_records=0;work_sets=collections.defaultdict(set)
for rec in load(R/'full-comparison/RECEIPTS.json'):
 data=load(R/'full-comparison'/rec['stdout']);work_sets[rec['id']].add(json.dumps(data.get('work'),sort_keys=True))
 if rec['mode']=='dopri5':
  assert all(data['work']['counters'].get(k,0)==0 for k in forbidden);explicit_records+=1
ensemble_digests=collections.defaultdict(set);ensemble_work=collections.defaultdict(set)
for rec in load(R/'ensemble-comparison/RECEIPTS.json'):
 d=load(R/'ensemble-comparison'/rec['stdout']);assert all(d['work']['counters'].get(k,0)==0 for k in forbidden)
 ensemble_digests[rec['rtol']].add(hashlib.sha256(json.dumps(d['states'],separators=(',',':')).encode()).hexdigest());ensemble_work[rec['rtol']].add(json.dumps(d['work'],sort_keys=True))
assert all(len(x)==1 for x in ensemble_digests.values()) and all(len(x)==1 for x in ensemble_work.values())
notes={
 'decay':('smooth_nonstiff_sampled_scope','No sampled uniform-time certificate.',['Application-specific error/long-time validation.']),
 'driven':('nonautonomous_rhs_sampled_scope','No dfdt calls; arbitrary forcing discontinuities not tested.',['Add client forcing/domain validation before application admission.']),
 'logistic':('nonlinear_analytic_sampled_scope','Observed logistic accuracy does not establish a general positivity theorem.',['Application invariant/positivity validation.']),
 'harmonic':('oscillatory_sampled_scope','VODE best observed error1.28484e-8 misses1e-8.',['Denser phase/interior output assessment as separately registered followup.']),
 'rapid':('precision_limited_at_tight_threshold','DOPRI best1.97151e-8 and VODE8.23170e-8 miss1e-8; native comparator faster at1e-6.',['Separate phase/global-error investigation or tighter registered controls; no retuning this run.']),
 'damped':('damped_oscillation_sampled_scope','Tight native ratio~1.07 is a small local timing difference.',['Controlled repeated performance measurements before speed promotion.']),
 'mixed-scales':('mixed_component_scales_sampled_scope','Normalized error and absolute error differ; no global tolerance guarantee.',['Application-specific component scaling and vector-atol validation.']),
 'nonnormal':('analytic_transient_sampled_scope','Five observations may miss between-output transient maxima.',['Interior/peak observable verification before broader transient claims.']),
 'large-vector':('large_diagonal_nonstiff_sampled_scope','VODE16384 timing cliff observed; cause NOT_MEASURED.',['Controlled BLAS-thread/host-load followup; retain current timing identity.']),
 'moderate-stiff':('limited_negative_control_observation','A favorable finite control does not make explicit RK a general stiff solver.',['Separately validate application stability restrictions.']),
 'stiff-control':('stiff_performance_negative_control','DOPRI~117x slower than native at1e-8; this VODE BDF setting misses1e-6 and1e-8.',['Use an explicitly chosen stiff method when warranted; review this comparator setting separately.'])}
families=[]
for case,n in dict.fromkeys((r['case'],r['dimension']) for r in main['rows']):
 rs=[r for r in main['rows'] if r['case']==case and r['dimension']==n];mode_info={}
 for mode in dict.fromkeys(r['mode'] for r in rs):
  z=[r for r in rs if r['mode']==mode];mode_info[mode]={'successful_configurations':sum(r['all_runs_successful'] for r in z),'configurations':len(z),'best_normalized_error':min(r['max_normalized_error'] for r in z),'min_observed_normalized_error':min(r['max_normalized_error'] for r in z),'max_observed_normalized_error':max(r['max_normalized_error'] for r in z),'eligibility':{str(t):[r['id'] for r in z if r['all_runs_successful'] and r['max_normalized_error']<=t] for t in [1e-6,1e-8]}}
 status,note,steps=notes[case];families.append({'case':case,'dimension':n,'scope':status,'limitations':note,'next_improvements':steps,'methods':mode_info,'comparisons':[c for c in main['common_error_comparisons'] if c['case']==case and c['dimension']==n]})
result={'schema':'vigilode.nonstiff.local-results.v1','source_identity':source,'source_modified':False,'postprocessing_script_sha256':digest(pathlib.Path(__file__)),'tests':counts,'targeted_test_total':25,'campaign':campaign,'configuration_count':len(main['rows']),'replay':main['replay'],'unparsed':main['unparsed'],'ratio_counts':dict(collections.Counter(c['status'] for c in main['common_error_comparisons'])),'accuracy_threshold_exclusions':[c for c in main['common_error_comparisons'] if c['status']!='COMPARABLE'],'problem_families':families,'all_rows_with_requested_tolerance_and_achieved_error':main['rows'],'ensemble':ensemble,'all_warmup_and_timed_ensemble_states_and_work_identical_across_workers':True,'integrity':checks,'source_input_files_verified':input_count,'explicit_records_checked_for_forbidden_operations':explicit_records,'forbidden_operations_checked':forbidden,'all_config_work_identical_across_warmup_and_repeats':all(len(s)==1 for s in work_sets.values()),'external_adapters':load(R/'EXTERNAL_ADAPTERS.json'),'timing_authority':'diagnostic_only','statistical_authority':'HOLD','general_production':'HOLD','claim_ceiling':'Finite registered analytic cases and25targetedcontracts; common sampled-error requirements, not uniform-time/global certification or universal speedup.','independent_review':'PENDING','ncp_used':False,'ssh_used':False}
(R/'RESULTS.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'source_input_files_verified':input_count,'archives':checks,'explicit_records_checked':explicit_records,'work_deterministic':result['all_config_work_identical_across_warmup_and_repeats'],'ensemble_state_and_work_identity':True,'results':'RESULTS.json'},indent=2))
