#!/usr/bin/env python3
"""Same-binary residual-first/candidate-budget study. No historical times pooled."""
import argparse,csv,gzip,hashlib,importlib.util,itertools,json,math,os,random,statistics,struct,sys
from pathlib import Path
import numpy as np
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('frozen_reference_helpers',ROOT/'research/coupled_native_20261020/benchmark.py')
legacy=importlib.util.module_from_spec(spec);spec.loader.exec_module(legacy)
ARMS=['eager-1','eager-4','lazy-1','lazy-4','sequential-mf','txn','cap64','cap256']
CELLS=[('few-mode',96,1.0,.001),('few-mode',384,1.0,.001),('few-mode',12288,1.0,.001),
 ('few-mode',12288,10.0,.01),('chain',96,1.0,.001),('chain',384,1.0,.001),
 ('diffusion',96,1.0,.001),('diffusion',384,1.0,.001),('diffusion',96,1.0,.01),('broad',64,0.0,.02)]
PAIRS=[('eager-1','lazy-1'),('eager-4','lazy-4'),('lazy-1','lazy-4'),('txn','cap64'),('txn','cap256')]
def dump(p,x):Path(p).write_text(json.dumps(x,indent=2,sort_keys=True,allow_nan=False)+'\n')
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def bits(xs):return b''.join(struct.pack('>d',x)for x in xs)
def same(a,b):
 return a.get('y_new')is not None and b.get('y_new')is not None and bits(a['y_new'])==bits(b['y_new']) and bits(a['error_vector'])==bits(b['error_vector'])
def details(row):
 result=row.get('result')or{};tx=result.get('result')or{};attempt=result.get('attempt')or tx.get('candidate_attempt')or{}
 c=attempt.get('candidate')or{};w=attempt.get('work')or{};fb=tx.get('fallback')or{}
 if result.get('kind')=='sequential-native-k':embedded=result.get('embedded_test_passes');lte=result.get('error_norm')
 elif c:embedded=c.get('embedded_test_passes');lte=c.get('embedded_wrms')
 else:embedded=fb.get('accepted_by_existing_driver');lte=fb.get('embedded_wrms')
 return {'basis':c.get('basis_dimension'),'rhs':w.get('rhs_attempted'),'jvp':w.get('jvp_attempted'),'stage_jvp':w.get('stage_jvp'),
  'validation_jvp':w.get('validation_jvp'),'newton':w.get('newton_updates'),'rhs_batches':w.get('stage_batches'),
  'derivative_batches':w.get('derivative_batches'),'avoided_jvp':w.get('unused_stage_jvp_avoided'),
  'fallback_used':tx.get('fallback_attempted',False),'failure_kind':(attempt.get('failure')or{}).get('kind'),
  'embedded_test_passes':embedded,'embedded_wrms':lte,'full_residual_wrms':c.get('full_residual_wrms')}

def main(a):
 out=a.out;out.mkdir(parents=True,exist_ok=True);store=legacy.Store(out);refs={};cal={};rows=[]
 fixture=json.loads(a.fixture.read_text())
 manifest={'source_commit':a.source,'binary_sha256':sha(a.binary),'comparison':'same binary eager vs opt-in residual-first and bounded transactions',
  'parent_companion':'15c43ce76734d0a560fcec5909ac49f1e6f2d347','coefficient_sha256':sha(a.fixture),'reference_helpers_sha256':sha(ROOT/'research/coupled_native_20261020/benchmark.py'),
  'cells':CELLS,'arms':ARMS,'pairs':PAIRS,'blocks':8,'repetitions':3,'warmups':1,'multiplicity':50,
  'accuracy_gate':{'output_wrms_up':.1,'embedded_difference_wrms':.1},'thread_affinity':sorted(os.sched_getaffinity(0)),
  'timing_authority':'shared_runner_diagnostic','new_full_trajectory_claim':False,'budgets_calibrated_optimal':False,
  'numpy':np.__version__,'parent_wall_times_reused':False}
 dump(out/'MANIFEST.json',manifest)
 for cell in CELLS:
  r=legacy.call(a.binary,cell,'eager-1',mode='reference');p=legacy.python_reference(cell,fixture)
  if 'y_new'not in r:raise RuntimeError('reference process failed:'+legacy.name(cell))
  x=np.asarray(p['y_new']);z=np.asarray(r['y_new']);rho=2*np.abs(x-z)+8*np.finfo(float).eps*np.abs(x)
  mismatch=float(np.max(np.abs(x-z)/(1+np.abs(x))))
  if mismatch>1e-10:raise RuntimeError('independent raw-reference mismatch')
  refs[legacy.name(cell)]={'python':p,'rust':r,'radius':rho.tolist(),'relative_crosscheck_max':mismatch}
 dump(out/'REFERENCES.json',refs)
 with gzip.open(out/'accuracy.jsonl.gz','wt')as f:
  for cell,arm in itertools.product(CELLS,ARMS):
   r=legacy.call(a.binary,cell,arm);m=legacy.assess(r,refs[legacy.name(cell)]);cal[(legacy.name(cell),arm)]=(r,m)
   f.write(json.dumps({'row':store.encode(r),'metrics':m},separators=(',',':'),allow_nan=False)+'\n');f.flush()
   rows.append({'cell':legacy.name(cell),'arm':arm,'root_completed':r.get('root_completed',False),'reference_admitted':m['valid'],**m,**details(r)})
 parity=[]
 for cell in CELLS:
  key=legacy.name(cell)
  for eager,lazy in [('eager-1','lazy-1'),('eager-4','lazy-4'),('lazy-1','lazy-4')]:
   x,xm=cal[(key,eager)];y,ym=cal[(key,lazy)]
   complete=x.get('root_completed')and y.get('root_completed')
   if complete:
    equal=same(x,y);dx=details(x);dy=details(y)
    if eager.startswith('eager'):
     equal=equal and dx['full_residual_wrms']==dy['full_residual_wrms']
    if not equal:raise RuntimeError('changed output/residual:'+key+':'+lazy)
   else:equal=None
   parity.append({'cell':key,'left':eager,'right':lazy,'completed_both':bool(complete),'output_and_embedded_bits_equal':equal})
 dump(out/'PARITY.json',parity)
 frozen=[{'cell':legacy.name(c),'arms':{arm:cal[(legacy.name(c),arm)][1]['valid']for arm in ARMS}}for c in CELLS]
 dump(out/'FROZEN_TIMING.json',frozen);manifest['frozen_sha256']=sha(out/'FROZEN_TIMING.json');dump(out/'MANIFEST.json',manifest)
 print('accuracy complete',len(rows),flush=True)
 units=[];rng=random.Random(20261020)
 # Even-order Williams design: each arm appears in each position once and
 # each directed carry-over pair occurs once over the eight rows.
 base_order=[0,1,7,2,6,3,5,4]
 with gzip.open(out/'timing.jsonl.gz','wt')as f:
  for block in range(8):
   cells=CELLS.copy();rng.shuffle(cells);order=[ARMS[(i+block)%8]for i in base_order]
   for cell in cells:
    for arm in order:
     r=legacy.call(a.binary,cell,arm,3,1);m=legacy.assess(r,refs[legacy.name(cell)]);old=cal[(legacy.name(cell),arm)][0]
     ns=r.get('elapsed_ns',[]);time_valid=len(ns)==3 and all(isinstance(v,int)and v>0 for v in ns)
     admitted=m['valid']and cal[(legacy.name(cell),arm)][1]['valid']and same(r,old)and time_valid
     u={'cell':legacy.name(cell),'arm':arm,'block':block,'admitted':bool(admitted),'time_valid':time_valid,
       'root_completed':r.get('root_completed',False),'median_ns':statistics.median(ns)if time_valid else None,'repetitions':3,
       'failure_kind':details(r)['failure_kind']}
     units.append(u);f.write(json.dumps({'row':store.encode(r),'metrics':m,'unit':u},separators=(',',':'),allow_nan=False)+'\n');f.flush()
   print('paired block',block+1,flush=True)
 dump(out/'TIMING_UNITS.json',units)
 comparisons=[];brng=np.random.default_rng(20261020)
 for cell in CELLS:
  key=legacy.name(cell)
  for baseline,candidate in PAIRS:
   a0,am=cal[(key,baseline)];b0,bm=cal[(key,candidate)]
   failed=not a0.get('root_completed')and not b0.get('root_completed')
   ratios=[]
   for block in range(8):
    q={u['arm']:u for u in units if u['cell']==key and u['block']==block};x=q[baseline];y=q[candidate]
    if (x['admitted']and y['admitted'])or(failed and x['time_valid']and y['time_valid']and not x['root_completed']and not y['root_completed']):
     ratios.append(x['median_ns']/y['median_ns'])
   relation='failed_attempt_cost_not_solver_speed'if failed else'exact_output_bits'if same(a0,b0)else'reference_threshold_only_different_targets'
   p={'cell':key,'baseline':baseline,'candidate':candidate,'ratios':ratios,'blocks':len(ratios),'relation':relation,
    'status':'COMPLETE_DIAGNOSTIC'if len(ratios)==8 else'HOLD_INCOMPLETE_OR_ACCURACY'}
   if len(ratios)==8:
    v=np.asarray(ratios);bs=np.median(v[brng.integers(0,8,(100000,8))],axis=1);alpha=.05/50
    p.update(ratio_median=float(np.median(v)),ci95_pointwise=np.quantile(bs,[.025,.975]).tolist(),ci95_family=np.quantile(bs,[alpha/2,1-alpha/2]).tolist())
   comparisons.append(p)
 dump(out/'PAIRED.json',comparisons)
 for r in rows:
  q=[u for u in units if u['cell']==r['cell']and u['arm']==r['arm']]
  r['median_ms']=statistics.median(u['median_ns']for u in q)/1e6 if all(u['time_valid']for u in q)else None
  r['admitted_processes']=sum(u['admitted']for u in q)
 with(out/'COST.csv').open('w',newline='')as f:w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
 summary={'status':'COMPLETE_BOUNDED','accuracy_rows':len(rows),'reference_admitted':sum(r['valid']for r in rows),
  'root_and_embedded_admitted':sum(r['valid']and r['embedded_test_passes']is True for r in rows),
  'timing_processes':len(units),'valid_timed_processes':sum(u['time_valid']for u in units),'admitted_timing_processes':sum(u['admitted']for u in units),
  'timed_attempts':len(units)*3,'comparison_count':len(comparisons),'complete_comparisons':sum(p['status']=='COMPLETE_DIAGNOSTIC'for p in comparisons),
  'exact_output_parity_pairs':sum(p['output_and_embedded_bits_equal']is True for p in parity),
  'state_objects':len(list(store.path.glob('*.gz'))),'state_compressed_bytes':sum(p.stat().st_size for p in store.path.glob('*.gz')),
  'unchanged_default_eager_api':True,'new_adaptive_integrator':False,'continuous_error_certificate':False}
 dump(out/'SUMMARY.json',summary);print(json.dumps(summary),flush=True)

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--fixture',type=Path,required=True);p.add_argument('--source',required=True);p.add_argument('--out',type=Path,required=True);a=p.parse_args()
 try:main(a)
 except Exception as e:a.out.mkdir(parents=True,exist_ok=True);dump(a.out/'BLOCKER.json',{'type':type(e).__name__,'message':str(e)});raise
