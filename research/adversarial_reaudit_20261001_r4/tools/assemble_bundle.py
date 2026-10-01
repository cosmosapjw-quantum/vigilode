from pathlib import Path
import json
W=Path(__file__).resolve().parents[1]
SRC='1c54194123ee6abc6daa512e8574922f510b4e2c';PRE='b2914f3e3c03eda60a8547619db6284aac8250a2'
def read(rel):return json.loads((W/rel).read_text())
def write(rel,j):(W/rel).write_text(json.dumps(j,ensure_ascii=False,indent=2)+'\n')
findings=[]
for lane,name in [('time','FINDINGS.json'),('polynomial','FINDINGS.json'),('homotopy','FINDINGS.json'),('statistics','RESULT.json')]:
 for x in read(f'{lane}/{name}')['findings']:
  y={'id':x['id'],'severity':x.get('severity',x.get('priority')),'title':x['title'],'status':'CONFIRMED_OPEN','scope':x.get('scope',x.get('scope_limit','')),'source_locations':x.get('source_locations',x.get('source',x.get('source_refs',[]))),'owner_evidence':f'{lane}/{name}','details':x}
  if x['id']=='R4-TIME-01':y['status']='ACKNOWLEDGED_RESIDUAL_NEW_COUNTEREXAMPLE'
  if x['id']=='R4-TIME-02':y['details']['status']='CONFIRMED_UPDATED_PATH_MISMATCH';y['details']['novelty']='Newly reproduced mismatch between unequal represented halves and the existing equal-half estimator; prior-version correctness on the identical fixture was not executed, so an introduced regression is not established.'
  findings.append(y)
write('FINDINGS.json',{'schema':'vigilode.r4.findings.v1','source_commit':SRC,'findings':findings,'count':len(findings),'severity_counts':{'P2':8,'P3':1},'production_fix_applied':False,'default_rodas_failure_demonstrated':False})
nodes=[]
for lane in ['time','polynomial','homotopy','statistics']:
 j=read(f'{lane}/NEXT_STEPS.json')
 for x in j.get('nodes',j.get('tasks',[])):
  changes=x.get('implementation',x.get('changes',x.get('change',[])));changes=[changes] if isinstance(changes,str) else changes
  targets=x.get('targets',x.get('files',[]))
  if not targets:
   targets=['crates/rodas5p-integrators/src/outward_certificate.rs'] if lane=='homotopy' else ['crates/rodas5p-fair-ab/src/paired_receipt.rs','crates/rodas5p-fair-ab/src/paired_timing.rs'] if lane=='statistics' else ['crates/rodas5p-core/src/polynomial_action.rs']
   if x['id']=='R4-HOM-DEV-04':targets=['crates/rodas5p-integrators/src/transactional_q1_q2.rs','crates/rodas5p-integrators/src/outward_certificate.rs']
  nodes.append({'id':x['id'],'priority':x['priority'],'title':x['title'],'state':'PROPOSED_NOT_IMPLEMENTED','depends_on':x.get('depends_on',[]),'targets':targets,'implementation':changes,'acceptance':x['acceptance'],'kill_or_stop_criterion':x.get('kill_criterion',x.get('stop_condition',x.get('failure_policy','Stop after exact controls and source-bound regression pass; if authority or total-cost gate fails, retain negative result without tolerance relaxation.'))),'claim_ceiling':x.get('claim_ceiling',x.get('risk',x.get('evidence_status','Named supported domain only; no production/speed promotion from finite fixtures.'))),'owner_plan':f'{lane}/NEXT_STEPS.json','details':x})
lookup={x['id']:x for x in nodes}
for id in ['R4-HOM-DEV-07','R4-POLY-DEV-04','R4-STAT-DEV-05']:
 for dep in ['R4-STAT-DEV-02','R4-STAT-DEV-04']:
  if dep not in lookup[id]['depends_on']:lookup[id]['depends_on'].append(dep)
lookup['R4-HOM-DEV-03']['targets']=['crates/rodas5p-integrators/src/stage_target.rs','crates/rodas5p-integrators/src/transactional_q1_q2.rs','crates/rodas5p-integrators/src/outward_certificate.rs']
lookup['R4-HOM-DEV-06']['targets']=['crates/rodas5p-integrators/src/transactional_q1_q2.rs']
lookup['R4-HOM-DEV-07']['targets']=['crates/rodas5p-cli/src/r3_campaigns.rs','crates/rodas5p-integrators/src/transactional_q1_q2.rs']
lookup['R4-STAT-DEV-01']['targets'].append('crates/rodas5p-cli/src/main.rs')
lookup['R4-STAT-DEV-03']['targets'].append('docs/TIMING_DESIGN_CONTRACT.md')
lookup['R4-STAT-DEV-04']['targets'].append('crates/rodas5p-fair-ab/src/timing_design.rs')
lookup['R4-STAT-DEV-05']['targets']=['crates/rodas5p-cli/src/r3_campaigns.rs']
nodes.extend([
 {'id':'R4-POLY-DEV-06','priority':'P3','title':'Prototype a separately versioned Leja or scaled-Taylor block-phi backend','state':'PROPOSED_NOT_IMPLEMENTED','depends_on':['R4-POLY-DEV-02'],'targets':['crates/rodas5p-core/src/polynomial_action.rs','crates/rodas5p-krylov/src'],'implementation':['Define block action/operator epoch/domain-witness interfaces and keep Arnoldi as reference/fallback.','Choose one initial non-Arnoldi backend; record scaling, degree, all matrix/vector products, coefficient and domain costs.','Derive a valid truncation/backward-to-forward bound for the declared operator class; do not label a two-small-terms heuristic Certified.'],'acceptance':['Exact-input diagonal and nilpotent cases; independently computed small nonnormal cases with explicit conditioning.','Capability rejection outside validated domain; no silent symmetry inference from eigenvalues.','Separate cold/warm/certified timings only after the statistics gates pass.'],'kill_or_stop_criterion':'Reject certification or stop candidate if domain bound cannot be established or total-cost improvement disappears; retain typed EstimateOnly research status.','claim_ceiling':'Literature-supported design proposal; neither implemented nor timed in R4.','owner_plan':'literature/RESEARCH_DIRECTION_KO.md'},
 {'id':'R4-VERIFY-DEV-01','priority':'P2','title':'Resolve the named native tests omitted by the bounded audit','state':'PROPOSED_NOT_IMPLEMENTED','depends_on':[],'targets':['evidence/runtime/LOCAL_REMAINDER_PLAN.json','crates'],'implementation':['Use the exact source/toolchain and retain each first failed/timed-out record.','Run only the 20 unresolved and 40 budget-skipped named tests on a suitable host with explicit per-target budgets; handle ignored tests/doctests as separate intended gates.'],'acceptance':['Publish per-test status and stderr plus source/dependency identity.','Never convert timeout or absence of output to PASS; classify any assertion failure and repair only under a separate authorized source task.'],'kill_or_stop_criterion':'Stop once each named remainder has a clear result or explicit environmental blocker; no fresh broad audit loop.','claim_ceiling':'Complete previously bounded regression evidence only; no speed claim.','owner_plan':'evidence/runtime/LOCAL_REMAINDER_PLAN.json'}])
ids={x['id'] for x in nodes}; assert len(ids)==len(nodes)
order=[];left=set(ids)
while left:
 ready=sorted(i for i in left if set(next(x['depends_on'] for x in nodes if x['id']==i))<=set(order))
 assert ready,('cycle or missing deps',left);order.extend(ready);left-=set(ready)
write('NEXT_DEVELOPMENT_DAG.json',{'schema':'vigilode.r4.development_dag.v1','source_commit':SRC,'nodes':nodes,'topological_order':order,'execution_policy':'Proposal only: each future scientific experiment requires its own committed preregistration.','authority_gate':'No timing promotion until R4-STAT-DEV-04 passes its own independent domain review.'})
rows=[
('TIME-01','PARTIAL_REOPENED','Original represented-clock fixtures pass; unequal-half estimator remains mismatched.',['R4-TIME-02']),
('TIME-02','CLOSED_FOR_ORIGINAL_REPRODUCERS','Invalid singleton/uniform spans rejected; explicit endpoint controls pass.',[]),
('TIME-03','PARTIAL_REOPENED','Indexed1000-step contract passes; acknowledged BDF step equality floor has new wrong-value witness.',['R4-TIME-01']),
('ARITH-01','CLOSED_FOR_ORIGINAL_REPRODUCERS','Positive/negative-h lost weights no longer authoritative/converged.',[]),
('ARITH-02','REOPENED_RESEARCH_BOUND','Valid low-order generated weights and high orders violate bounds; malformed public inputs also fail open.',['R4-ARITH-01','R4-ARITH-02','R4-ARITH-03']),
('ARITH-03','CLOSED_FOR_REGISTERED_CONTRACTS','Dense reference flags are propagated; current targeted native tests pass.',[]),
('ARITH-04','CLOSED_FOR_REGISTERED_CONTRACTS','Directed budget fixture contracts pass; no universal formal proof inferred.',[]),
('STAT-DEV-01','CLOSED_FOR_ORIGINAL_REPRODUCERS','All five malformed raw case mutations reject.',[]),
('STAT-DEV-02','PARTIAL_REOPENED','Process receipts exist; individual later-session cells are incompletely validated.',['R4-STAT-02']),
('STAT-DEV-03','CLOSED_WITH_CLAIM_CEILING','MC gate controls simulation error/conservatism; it does not establish population coverage.',[]),
('STAT-DEV-04','MEASURED_FAIL_PRESERVED','Both coverage FAIL studies retained; actual hold propagation absent.',['R4-STAT-01']),
('HOM-01','CLOSED_WITH_CLAIM_CEILING','Explicit sequential strict-lower target exists; original structural contracts pass.',[]),
('HOM-02','CONDITIONAL_CONSTRUCTOR_DOMAIN','Valid witness majorant structure and existing native fixtures supported; public malformed witnesses excluded from this claim.',['R4-HOM-01']),
('HOM-03','PARTIAL_REOPENED','Constructor controls pass; public/serde witness payload not sealed or fully checked.',['R4-HOM-01']),
('HOM-04','NATIVE_EIGHT_SUPPORTED_GENERIC_REOPENED','Three levels suffice for s8 but not generic s9/s16.',['R4-HOM-02']),
('HOM-05','CLOSED_WITH_CLAIM_CEILING','q2 seven-batch replacement contracts pass; stage certificate only, q1 operational, serial consumer.',[]),
('HOM-06','MEASURED_FAIL_PRESERVED','All four published arms replay Block; no new timing experiment.',[]),
('POLY-01','CLOSED_WITH_CLAIM_CEILING','Joint dense symmetric action/cache implemented and bounded new oracle corpus passes.',[]),
('POLY-02','CLOSED_WITH_CLAIM_CEILING','32newCertified enclose numerical errors;8EstimateOnly honest; no full-domain proof from finite tests.',[]),
('POLY-03','MEASURED_NUMERIC_PASS_UNDER_HOLD','Warm unbounded-action coefficient cache result only; certified whole-integrator speed not established.',['R4-STAT-01']),
('PROCESS-01','R4_PREREGISTRATION_SATISFIED','R4 inputs remotely committed before native/oracle execution; first failures and finite scope retained.',[])]
write('CLOSURE_MATRIX.json',{'schema':'vigilode.r4.closure.v1','source_commit':SRC,'prior_node_count':21,'rows':[{'node':a,'status':b,'evidence_summary':c,'findings':d,'scope':'Re-audit conclusion, not unqualified production closure.'}for a,b,c,d in rows]})
claims=[]
for lane,filename in [('polynomial','CLAIMS.json'),('homotopy','CLAIMS.json'),('statistics','RESULT.json')]:
 for x in read(f'{lane}/{filename}')['claims']:
  claims.append({'id':x['id'],'statement':x.get('claim',x.get('statement')),'evidence_level':x.get('evidence_status',x.get('status',[])),'claim_ceiling':x.get('ceiling',x.get('limits',x.get('scope_limits',[]))),'owner_record':f'{lane}/{filename}','details':x})
claims.extend([
 {'id':'R4-TIME-C01','statement':'Existing public variable-BDF kernel on the same recorded time list repairs the registered unit-covariance fixtures.','evidence_level':['derived','implementation-verified','numerically checked'],'claim_ceiling':'Four fixed time-unit cases; no full BDF promotion.','owner_record':'time/FINDINGS.json'},
 {'id':'R4-TIME-C02','statement':'Geometry factor eta/(1-eta) gives exact5/4 correction on the registered Radau1 quadratic primitive.','evidence_level':['derived','implementation-verified','numerically checked'],'claim_ceiling':'Leading common LTE model generally; exact only on declared fixture family.','owner_record':'time/DERIVATIONS_KO.md'}])
for x in claims:x['independent_disposition']='SEE_DECISION_FINAL_REVIEW'
write('RESEARCH_CLAIMS.json',{'schema':'vigilode.r4.claims.v1','claims':claims,'production_promotion':False,'performance_promotion':False})
runtime=read('evidence/runtime/FINAL_RUNTIME_SUMMARY.json')
numeric={'schema':'vigilode.r4.numeric_results.v1','source_commit':SRC,'preregistration_commit':PRE,'native_rows':{'time':36,'polynomial':78,'homotopy':9,'statistics':21,'total':144},'findings':{'P2':8,'P3':1,'total':9},'source_invariant_gate':'FAIL','bounded_candidate_checks_gate':'PASS','native_regression':runtime['test_counts'],'complete_workspace_pass':False,'polynomial_actions':{'count':40,'certified':32,'estimate_only':8,'max_observed_absolute_error':4.73850883134441e-14,'false_certified_bounds_observed':0},'transform_false_bound_cases':4,'homotopy_unsound_observations':4,'statistics_later_malformed_records_admitted':5,'exact_candidate_counts':{'scaled_norm':5,'inverse_factorial':6,'adaptive_doubling':6,'diagonal_factorization':3,'session_interval_designs':32,'session_interval_sign_enumerations':6,'session_interval_decision_controls':12,'session_interval_invalid_inputs':4},'production_promotion':False,'speed_promotion':False,'new_timing_campaign':False}
write('NUMERIC_RESULTS.json',numeric)
bundle={'schema_version':'vigilode.r4.audit_bundle.v1','source_commit':SRC,'preregistration_commit':PRE,'branch':'claude/jolly-wozniak-7wl15h-wu23-reaudit-r3','verdict':'REWORK','production_modified':False,'new_branch_created':False,'findings':findings,'closure':read('CLOSURE_MATRIX.json')['rows'],'claims':claims,'development':nodes,'topological_order':order,'numeric_results':numeric,'runtime':runtime,'independent_review':'decision/FINAL_REVIEW.json','report':'REVIEW_KO.md','reproduce':'REPRODUCE.md'}
write('AUDIT_BUNDLE.json',bundle)
p=W/'REVIEW_KO.md';t=p.read_text().replace('<!-- RUNTIME_SUMMARY_INSERT -->','전체 workspace / all-targets / all-features offline 빌드는 **230.67초, exit 0**으로 완료됐다. 발견한 149개 harness의 735개 named test 중 **671 PASS, 4 ignored, 20 timeout 미확정, 40 총예산상 미실행**이다. Assertion failure는 관측하지 않았지만 **전체 suite PASS는 아니다**. 변경된 15개 harness는 모두 정상 종료했고 그 안의 68개 test가 통과, 1개가 ignored였다. Doctest는 실행하지 않았다. `evidence/runtime/TEST_COVERAGE.json`은 모든 이름과 상태를 보존하고 `LOCAL_REMAINDER_PLAN.json`은 남은 검사만 지정한다.')
t=t.replace('새 `split_clock`와 기존 Richardson 공식 사이의 새 결합 결함','업데이트된 경로에서 새로 재현한 기하/공식 불일치; 이전 버전의 동일 입력 정상 여부는 미검증')
p.write_text(t)
print({'findings':len(findings),'tasks':len(nodes),'claims':len(claims),'closure':len(rows)})
