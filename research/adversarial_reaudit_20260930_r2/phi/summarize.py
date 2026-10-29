"""Summarize raw native observations; assertions close only bounded fixtures."""
from pathlib import Path
import hashlib, json, re, subprocess

base = Path(__file__).resolve().parent
repo = base.parents[1] / 'vigilode'
rows = [json.loads(line) for line in (base / 'native_probe.jsonl').read_text().splitlines()]
groups = {key: [row for row in rows if row['id'] == key] for key in {row['id'] for row in rows}}
closure = {}
for key in ['old_p1_closure', 'old_p2_closure', 'prefix_closure']:
    values = groups[key]
    maximum = max(row['relative_error'] for row in values)
    assert maximum <= 1e-12
    assert all(row.get('converged', True) for row in values)
    closure[key] = {'observations': len(values), 'max_relative_error': maximum, 'status': 'PASS_BOUNDED_FIXTURES'}
old_p3 = groups['old_p3_closure'][0]
assert old_p3['report']['converged'] and old_p3['report']['krylov_dimension'] == 1
assert abs(old_p3['report']['value'][0] - old_p3['expected']) <= 1e-15
closure['old_p3_closure'] = {'observations': 1, 'status': 'PASS_BOUNDED_FIXTURE', 'absolute_error': abs(old_p3['report']['value'][0] - old_p3['expected'])}
candidates = [row for row in rows if row['id'] in ['scaled_range_candidate', 'amplitude_candidate']]
assert len(candidates) == 13 and all(row['status'] == 'Ok' and row['relative_error'] <= 1e-12 for row in candidates)
assert all(row['report']['value'] == [0.] and row['report']['converged'] for row in groups['scaled_range_fused'][:2])
assert all(row['value'] == 0. for row in groups['amplitude_dense'] if row['h'] >= 1e60)
counts = re.findall(r'test result: ok\. (\d+) passed', (base / 'existing_contracts.log').read_text())
assert (base / 'native_probe.exit').read_text().strip() == '0'
assert (base / 'existing_contracts.exit').read_text().strip() == '0'
result = {
 'schema':'vigilode.reaudit.phi.v1',
 'source_commit':subprocess.check_output(['git','-C',str(repo),'rev-parse','HEAD'],text=True).strip(),
 'source_tree':subprocess.check_output(['git','-C',str(repo),'rev-parse','HEAD^{tree}'],text=True).strip(),
 'claim_ceiling':'bounded native closure; no general forward certificate; no production patch applied',
 'native_execution': {'exit':0,'raw_rows':len(rows),'raw_rows_are_not_test_count':True,'command':'source runtime_r2/env.sh; CARGO_TARGET_DIR=runtime_r2/target-phi CARGO_BUILD_JOBS=1 cargo run --manifest-path reaudit/phi/probe/Cargo.toml --offline'},
 'existing_contracts':{'exit':0,'passed':sum(map(int,counts)),'failed':0,'suites':5,'source_mutation':False},
 'old_closure':closure,
 'findings':[
  {'id':'PHI-R1','severity':'P2','status':'implementation-verified','failure_class':['NUMERICAL','IMPLEMENTATION'],
   'title_ko':'가중치 변환이 표현 가능한 h^k b를 중간 powi 언더플로로 0으로 소실',
   'scope':'public scaled fused/dense APIs; prefix shares helper by source trace; ordinary-step integrator failure not established',
   'source_locations':['crates/rodas5p-integrators/src/exponential.rs:1346-1364','crates/rodas5p-integrators/src/exponential.rs:1774-1777','crates/rodas5p-core/src/matrix_functions.rs:252-263'],
   'input':{'A':0,'h':[1e-100,-1e-100],'b4':1e300,'other_b':0,'rtol':1e-12,'atol':0},
   'expected':1e-100/24,'actual':0,'fused_converged':True,
   'overflow_variant':{'h':1e100,'b4':1e-300,'finite_expected':1e100/24,'actual':'Err(NonFinite)'},
   'historical_class':'newly identified failure in changed weighting lines; old implementation performance at these inputs NOT_EVALUATED'},
  {'id':'PHI-R2','severity':'P2','status':'implementation-verified','failure_class':['NUMERICAL'],
   'title_ko':'Dense φ oracle의 입력 진폭 비불변성: 유한한 큰 결과가 Ok(0)',
   'scope':'dense_phi_combination and its oracle role; fused direct combination passes same amplitude sweep',
   'source_locations':['crates/rodas5p-core/src/matrix_functions.rs:310-328','crates/rodas5p-core/src/matrix_functions.rs:85-130'],
   'input':{'A':-1,'h':0.1,'w1':1e60,'other_w':0},
   'expected':9.516258196404042e59,'actual':0,
   'intermediate_amplitude_errors':{'1e20':6.801411656098821e-10,'1e40':0.043049753133909685},
   'historical_class':'remaining amplitude-conditioning failure after time-normalization; not reopening the fixed small-step fixture'}],
 'candidate':{'status':['derived','numerically checked','implementation-verified'],'production_applied':False,
  'functions':['scaled_weights','balanced_dense'],'observations':len(candidates),
  'max_relative_error':max(row['relative_error'] for row in candidates),
  'independent_decision':'PENDING_ROOT_REVIEW','general_nonnormal_stability':'unresolved',
  'mixed_extreme_dynamic_range':'NOT_EVALUATED; common normalization may underflow small components'},
 'data_semantics':{'amplitude_dense/amplitude_candidate':'legacy helper field h contains amplitude, while actual step is fixed h=0.1; amplitude_fused has explicit amplitude key','null_nested_difference':'serde_json encodes initial Infinity diagnostic as null; not an observed finite zero'},
 'sha256':{name:hashlib.sha256((base/name).read_bytes()).hexdigest() for name in ['probe/src/main.rs','probe/Cargo.toml','probe/Cargo.lock','native_probe.jsonl','native_probe.stderr','existing_contracts.log']}
}
(base/'PHI_STATUS.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({'closure':closure,'existing_passed':result['existing_contracts']['passed'],'candidate_max_error':result['candidate']['max_relative_error'],'status':'ASSERTIONS_PASS'},ensure_ascii=False))
