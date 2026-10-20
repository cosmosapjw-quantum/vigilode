#!/usr/bin/env python3
"""Frozen fixed-step latency campaign; full states/time samples are retained.

The coupled worker-count comparison has bit-identical outputs. Comparisons to
native-K sequential are separately labelled reference-threshold-only, not exact
raw/K identity or optimal-algorithm ranking.
"""
import argparse,csv,gzip,hashlib,itertools,json,math,os,random,statistics,subprocess,sys
from collections import Counter
from pathlib import Path
import numpy as np
import scipy
from scipy.linalg import lu_factor,lu_solve
ARMS=['coupled-1','coupled-2','coupled-4','sequential-mf','transactional-4']
CELLS=[(name,n,e,h) for name,ns,es,hs in [
 ('few-mode',[96,384,12288],[1.0,10.0],[.01]),
 ('few-mode',[96,384,12288],[1.0],[.001]),
 ('broad',[64],[0.0],[.02]),('chain',[96,384],[1.0],[.01]),
 ('diffusion',[96,384],[1.0],[.001,.01])]
 for n,e,h in itertools.product(ns,es,hs)]

def dump(path,x):Path(path).write_text(json.dumps(x,indent=2,sort_keys=True,allow_nan=False)+'\n')
def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def name(cell):return f'{cell[0]}_n{cell[1]}_eps{cell[2]:g}_h{cell[3]:g}'
def call(binary,cell,arm,reps=1,warm=0,mode='run'):
 cmd=[str(binary),mode,cell[0],str(cell[1]),str(cell[2]),str(cell[3]),arm,str(reps),str(warm)]
 try:
  p=subprocess.run(cmd,capture_output=True,text=True,timeout=30)
  r=json.loads(p.stdout) if p.returncode==0 else {'root_completed':False,'error':p.stderr[-6000:],'exit_code':p.returncode}
 except subprocess.TimeoutExpired:r={'root_completed':False,'error':'30-second process timeout','exit_code':124}
 r.update(cell_id=name(cell),command=cmd);return r

def python_reference(cell,fixture):
 case,n,eps,h=cell;gamma=float(fixture['gamma']);A=np.array(fixture['A'],dtype=float);C=np.array(fixture['C'],dtype=float);b=np.array(fixture['b_code'],dtype=float)
 i=np.arange(n);rates=-10.**(4.*i/(n-1)) if case=='broad' else np.array([-1.,-20.,-100.])[i%3]
 y=np.ones(n) if case=='broad' else .1+(.01*np.sin(2*np.pi*i/n) if case=='diffusion' else np.zeros(n))
 d=.002*n*n if case=='diffusion' else 0.
 def rhs(x):
  f=rates*x+eps*x*x
  if case=='chain':f[1:]+=.3*x[:-1]
  if case=='diffusion':f+=d*(np.roll(x,1)-2*x+np.roll(x,-1))
  return f
 hg=h*gamma;diag=1-hg*(rates+2*eps*y-2*d)
 if case in ['chain','diffusion']:
  W=np.diag(diag)
  if case=='chain':W[np.arange(1,n),np.arange(n-1)]=-.3*hg
  else:
   W[i,(i-1)%n]=-d*hg;W[i,(i+1)%n]=-d*hg
  lu=lu_factor(W)
 else:lu=None
 u=np.zeros((8,n))
 for j in range(8):
  state=y.copy();mix=np.zeros(n)
  for k in range(j):state+=A[j,k]*u[k];mix+=C[j,k]*u[k]
  r=hg*rhs(state)+gamma*mix
  u[j]=lu_solve(lu,r) if lu is not None else r/diag
 output=y.copy()
 for j in range(8):output+=b[j]*u[j]
 return {'y0':y.tolist(),'y_new':output.tolist(),'error_vector':u[-1].tolist(),'stages_u':u.tolist()}

class Store:
 def __init__(self,out):self.out=out;self.path=out/'states';self.path.mkdir(exist_ok=True)
 def encode(self,row):
  r=dict(row);result=r.pop('result',None);obj={'y_new':r.pop('y_new',None),'error_vector':r.pop('error_vector',None),'result':result}
  b=json.dumps(obj,separators=(',',':'),sort_keys=True,allow_nan=False).encode();h=hashlib.sha256(b).hexdigest();p=self.path/(h+'.json.gz')
  if not p.exists():
   with gzip.open(p,'wb',compresslevel=9)as f:f.write(b)
  r['state_object_sha256']=h;return r

def assess(row,ref):
 bad={'valid':False,'output_wrms_up':None,'embedded_difference_wrms':None}
 if not row.get('root_completed') or row.get('deterministic')is not True:return bad
 v=np.asarray(ref['python']['y_new']);y=np.asarray(row.get('y_new'));e=np.asarray(row.get('error_vector'));er=np.asarray(ref['python']['error_vector'])
 if y.shape!=v.shape or e.shape!=v.shape or not np.isfinite(y).all() or not np.isfinite(e).all():return bad
 y0=np.asarray(ref['python']['y0']);scale=1e-10+1e-6*np.maximum(np.abs(y0),np.abs(v));rho=np.asarray(ref['radius'])
 value=float(np.linalg.norm((y-v)/scale)/math.sqrt(len(y))+np.linalg.norm(rho/scale)/math.sqrt(len(y)))
 embedded=float(np.linalg.norm((e-er)/scale)/math.sqrt(len(y)))
 return {'valid':bool(value<=.1 and embedded<=.1),'output_wrms_up':value,'embedded_difference_wrms':embedded}

def main(a):
 out=a.out;out.mkdir(parents=True,exist_ok=True);store=Store(out)
 fixture=json.loads(a.fixture.read_text());refs={};cal={};compact=[]
 manifest={'source_commit':a.source,'binary_sha256':sha(a.binary),'coefficient_sha256':sha(a.fixture),'numpy':np.__version__,'scipy':scipy.__version__,
  'cells':CELLS,'arms':ARMS,'blocks':10,'fixed_input_rtol':1e-6,'fixed_input_atol':1e-10,'accuracy_gate':{'output_wrms_up':.1,'embedded_difference_wrms':.1},
  'affinity':sorted(os.sched_getaffinity(0)),'clock_authority':'diagnostic_shared_runner','cost_scope':'fixed-h complete root attempt, not adaptive trajectory',
  'preconditioning_baseline':'existing sequential GMRES, not claimed strongest U-form/other library implementation',
  'reference_kind':'independent Python and Rust raw-U stage recurrence; empirical discrepancy, not continuous exact solution',
  'multiplicity':len(CELLS)*4,'resampling_unit':'one fresh process median per arm per paired block'}
 dump(out/'MANIFEST.json',manifest)
 for cell in CELLS:
  r=call(a.binary,cell,'coupled-1',mode='reference');p=python_reference(cell,fixture)
  if 'y_new' not in r:raise RuntimeError('reference process failed:'+name(cell))
  v=np.asarray(p['y_new']);v2=np.asarray(r['y_new']);rho=2*np.abs(v-v2)+8*np.finfo(float).eps*np.abs(v)
  ref={'python':p,'rust':r,'radius':rho.tolist(),'relative_crosscheck_max':float(np.max(np.abs(v-v2)/(1+np.abs(v))))}
  if ref['relative_crosscheck_max']>1e-10:raise RuntimeError('reference cross-check failure:'+name(cell))
  refs[name(cell)]=ref
 dump(out/'REFERENCES.json',refs)
 with gzip.open(out/'accuracy.jsonl.gz','wt')as stream:
  for cell,arm in itertools.product(CELLS,ARMS):
   r=call(a.binary,cell,arm);m=assess(r,refs[name(cell)]);cal[(name(cell),arm)]=(r,m)
   entry={'row':store.encode(r),'metrics':m};stream.write(json.dumps(entry,separators=(',',':'),allow_nan=False)+'\n');stream.flush()
   c={'cell':name(cell),'arm':arm,'root_completed':r.get('root_completed',False),**m}
   if arm.startswith('coupled') and 'result'in r:
    attempt=r['result']['attempt'];c.update(failure_kind=(attempt.get('failure')or{}).get('kind'),basis=(attempt.get('candidate')or{}).get('basis_dimension'),
      rhs=attempt['work']['rhs_attempted'],jvp=attempt['work']['jvp_attempted'],batches=attempt['work']['stage_batches'],validation_batches=attempt['work']['validation_batches'],
      newton_updates=attempt['work']['newton_updates'],peak_parallel_tasks=attempt['work']['peak_parallel_tasks'])
   compact.append(c)
 print('accuracy rows',len(compact),flush=True)
 # Fix eligible comparisons before reading any timed sample. Rejected attempts
 # remain timed and saved as failure cost, but never receive a speedup verdict.
 frozen=[{'cell':name(cell),'eligible':{arm:cal[(name(cell),arm)][1]['valid'] for arm in ARMS}}for cell in CELLS]
 dump(out/'FROZEN_COMPARISONS.json',frozen);manifest['frozen_sha256']=sha(out/'FROZEN_COMPARISONS.json');dump(out/'MANIFEST.json',manifest)
 rng=random.Random(20261020);units=[]
 with gzip.open(out/'timing.jsonl.gz','wt')as stream:
  for block in range(10):
   cells=CELLS.copy();rng.shuffle(cells);pattern=[0,1,4,2,3];rotation=block%5
   order=[ARMS[(i+rotation)%5]for i in pattern];order=order if block<5 else list(reversed(order))
   for cell in cells:
    for arm in order:
     reps=3 if cell[1]>=12288 else 5
     r=call(a.binary,cell,arm,reps,1);m=assess(r,refs[name(cell)]);old=cal[(name(cell),arm)][0]
     complete=m['valid'] and cal[(name(cell),arm)][1]['valid']
     if complete:
      complete=r.get('y_new')==old.get('y_new') and r.get('error_vector')==old.get('error_vector')
     ns=r.get('elapsed_ns',[]);complete=complete and len(ns)==reps and all(isinstance(v,int)and v>0 for v in ns)
     u={'cell':name(cell),'arm':arm,'block':block,'admitted':bool(complete),'root_completed':r.get('root_completed',False),
        'median_ns':statistics.median(ns) if len(ns)==reps else None,'output_wrms_up':m['output_wrms_up'],'repetitions':reps}
     units.append(u);stream.write(json.dumps({'row':store.encode(r),'metrics':m,'unit':u},separators=(',',':'),allow_nan=False)+'\n');stream.flush()
   print('timing block',block+1,flush=True)
 dump(out/'TIMING_UNITS.json',units)
 pairs=[];brng=np.random.default_rng(20261020)
 for cell in CELLS:
  for base,candidate in [('coupled-1','coupled-2'),('coupled-1','coupled-4'),('sequential-mf','coupled-4'),('sequential-mf','transactional-4')]:
   ratios=[]
   for block in range(10):
    pair={u['arm']:u for u in units if u['cell']==name(cell)and u['block']==block}
    if pair[base]['admitted']and pair[candidate]['admitted']:ratios.append(pair[base]['median_ns']/pair[candidate]['median_ns'])
   entry={'cell':name(cell),'baseline':base,'candidate':candidate,'blocks':len(ratios),'ratios':ratios,
     'status':'DIAGNOSTIC_COMPLETE'if len(ratios)==10 else'HOLD_ACCURACY_OR_EXECUTION',
     'accuracy_relation':'same-coupled-output-bits'if base=='coupled-1'else'raw-reference-threshold-only-native-K-differs'}
   if len(ratios)==10:
    v=np.asarray(ratios);samples=np.median(v[brng.integers(0,10,(100000,10))],axis=1);alpha=.05/manifest['multiplicity']
    entry.update(ratio_median=float(np.median(v)),ci95=np.quantile(samples,[.025,.975]).tolist(),ci95_family=np.quantile(samples,[alpha/2,1-alpha/2]).tolist())
   pairs.append(entry)
 dump(out/'PAIRED.json',pairs)
 cost=[]
 for row in compact:
  u=[q for q in units if q['cell']==row['cell']and q['arm']==row['arm']]
  row.update(median_ms=statistics.median(q['median_ns']for q in u)/1e6 if all(q['median_ns']is not None for q in u)else None,admitted_processes=sum(q['admitted']for q in u));cost.append(row)
 fields=[]
 for row in cost:
  for k in row:
   if k not in fields:fields.append(k)
 with(out/'COST.csv').open('w',newline='')as f:w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows(cost)
 summary={'status':'COMPLETE_BOUNDED','accuracy_rows':len(compact),'accuracy_admitted':sum(r['valid']for r in compact),
  'by_arm':{arm:{'root_completed':sum(r['root_completed']for r in compact if r['arm']==arm),'reference_admitted':sum(r['valid']for r in compact if r['arm']==arm)}for arm in ARMS},
  'timing_processes':len(units),'admitted_timing_processes':sum(u['admitted']for u in units),'timed_integrations':sum(u['repetitions']for u in units),
  'pairs':len(pairs),'complete_pairs':sum(p['status']=='DIAGNOSTIC_COMPLETE'for p in pairs),'state_objects':len(list(store.path.glob('*.gz'))),
  'state_compressed_bytes':sum(p.stat().st_size for p in store.path.glob('*.gz')),'no_trajectory_speed_claim':True,'no_certified_error_claim':True}
 dump(out/'SUMMARY.json',summary);print(json.dumps(summary),flush=True)

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--fixture',type=Path,required=True);p.add_argument('--source',required=True);p.add_argument('--out',type=Path,required=True);a=p.parse_args()
 try:main(a)
 except Exception as e:a.out.mkdir(parents=True,exist_ok=True);dump(a.out/'BLOCKER.json',{'error_type':type(e).__name__,'error':str(e)});raise
