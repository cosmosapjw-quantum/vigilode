#!/usr/bin/env python3
"""Exact-rational generated-model proof replay; references never approve proofs."""
from fractions import Fraction as Q
from math import comb
from pathlib import Path
import gzip,hashlib,json,math,statistics,struct,sys
from collections import Counter

def fq(x):return Q(float(x))
def elevate(p,n):
 d=len(p)-1
 return [sum((Q(comb(d,j)*comb(n-d,k-j),comb(n,k))*p[j]for j in range(max(0,k-(n-d)),min(k,d)+1)),Q(0))for k in range(n+1)]
def product(p,q):
 n=len(p)-1;m=len(q)-1;out=[Q(0)]*(n+m+1)
 for i in range(n+1):
  for j in range(m+1):out[i+j]+=Q(comb(n,i)*comb(m,j),comb(n+m,i+j))*p[i]*q[j]
 return out
def exp_bounds(x):
 assert x>=0;squares=0
 while x>Q(1,2):x/=2;squares+=1
 term=Q(1);s=term
 for k in range(1,41):term*=(-x)/k;s+=term
 hi=s;lo=s+term*(-x)/41
 for _ in range(squares):lo*=lo;hi*=hi
 return lo,hi
def flow(r0,delta,a,t):
 if a==0:return r0+delta*t
 lo,hi=exp_bounds(a*t);c=r0-delta/a
 return delta/a+c*(hi if c>=0 else lo)
def value(b,t):
 p=b[:]
 for degree in range(len(b)-1,0,-1):
  for k in range(degree):p[k]=(1-t)*p[k]+t*p[k+1]
 return p[0]
def fp(y):
 h=0xcbf29ce484222325
 for x in y:
  for b in struct.pack('<d',x):h=((h^b)*0x100000001b3)&((1<<64)-1)
 return h

def verify_curve(s,model):
 n=model['n'];d,la,al,eps=[fq(model[k])for k in ['d','lambda','alpha','epsilon']];curve=s['curve'];controls=[[fq(x)for x in b]for b in curve['controls']]
 assert len(controls)==n and all(0<=x<=1 for b in controls for x in b)
 identity=[n]+[struct.unpack('<Q',struct.pack('<d',float(model[k])))[0]for k in ['d','lambda','alpha','epsilon']];assert curve['owner']==identity
 p=len(controls[0])-1;degree=2*p;dt=fq(curve['t1'])-fq(curve['t0']);assert dt>0
 mean=[sum((b[k]for b in controls),Q(0))/n for k in range(p+1)];em=elevate(mean,degree);count=0
 for i,b in enumerate(controls):
  derivative=elevate([p*(b[k+1]-b[k])/dt for k in range(p)],degree)
  linear=[-la*b[k]+(d*(controls[i-1][k]-b[k])if i else 0)+(d*(controls[i+1][k]-b[k])if i+1<n else 0)for k in range(p+1)]
  linear=elevate(linear,degree);mq=product(mean,b);qq=product(b,b);exact=[derivative[k]-linear[k]-al*(em[k]-mq[k])+eps*qq[k]for k in range(degree+1)]
  intervals=s['residual_coefficients'][i];assert len(intervals)==len(exact)
  for z,r in zip(exact,intervals):
   assert fq(r['lo'])<=z<=fq(r['hi']),('NON_ENCLOSED_RESIDUAL',i,z,r)
   assert fq(s['defect_upper'])>=max(abs(fq(r['lo'])),abs(fq(r['hi'])));count+=1
 decay=fq(s['decay_lower']);assert 0<=decay<=la-al
 bound=flow(fq(s['initial_radius']),fq(s['defect_upper']),decay,dt)
 assert fq(s['endpoint_radius'])>=bound
 assert fq(s['window_radius'])>=max(fq(s['initial_radius']),bound)
 return count

def main(out):
 manifest=json.loads((out/'MANIFEST.json').read_text());refs=json.loads((out/'REFERENCES.json').read_text());objects={};verified_proofs=set();checks=Counter();tim={};cal={}
 def load_obj(h):
  if h not in objects:
   data=gzip.decompress((out/'objects'/(h+'.json.gz')).read_bytes());assert hashlib.sha256(data).hexdigest()==h;objects[h]=json.loads(data)
  return objects[h]
 for filename,phase in [('CALIBRATION.jsonl.gz','cal'),('TIMING.jsonl.gz','time')]:
  for line in gzip.open(out/filename,'rt'):
   r=json.loads(line);row=r['row'];obj=load_obj(row['object_sha256']);met=r['metrics'];checks[phase]+=1
   if row.get('success'):
    ref=refs[f"{row['n']}:{row['epsilon']}"];y=obj['states'];v=ref['states'];assert len(y)==len(v)==21 and row['times']==ref['times'];assert all(len(q)==row['n']for q in y)
    error=max(abs(a-b)for q,p in zip(y,v)for a,b in zip(q,p))+max(x for q in ref['radius']for x in q)
    assert math.isclose(error,met['absolute_error_upper'],rel_tol=1e-12,abs_tol=1e-20);assert met['valid']==(error<=1e-6 and row['deterministic']);checks['reference_metrics']+=1
   else:assert not met['valid'];checks['failed_observations_preserved']+=1
   integ=(obj.get('details')or{}).get('integration')
   if integ and row['object_sha256']not in verified_proofs:
    verified_proofs.add(row['object_sha256']);model=row['model'];assert model['lambda']>=model['alpha']>=0 and model['epsilon']>=0 and model['d']>=0
    assert all(0<=x<=1 for x in row['initial']);rprev=fq(integ['radii'][0]);yprev=row['initial'];tprev=fq(row['times'][0]);segments=integ['segments']
    for s in segments:
     checks['exact_residual_coefficients']+=verify_curve(s,model);checks['continuous_segments']+=1
     assert fq(s['curve']['t0'])==tprev and fq(s['initial_radius'])==rprev
     assert [b[0]for b in s['curve']['controls']]==yprev
     yprev=[b[-1]for b in s['curve']['controls']];tprev=fq(s['curve']['t1']);rprev=fq(s['endpoint_radius'])
    assert [float(x)for x in yprev]==integ['final_state'] and tprev==fq(integ['final_t'])and rprev==fq(integ['final_radius'])
    state=row['initial'];radius=fq(integ['radii'][0]);clock=fq(row['times'][0]);si=0
    for trial in integ['trials']:
     assert fq(trial['t0'])==clock and fq(trial['radius_before'])==radius
     if trial['accepted']:
      seg=segments[si];si+=1;clock=fq(seg['curve']['t1']);state=[b[-1]for b in seg['curve']['controls']];radius=fq(seg['endpoint_radius'])
     else:checks['rollback_trials']+=1
     assert fq(trial['radius_after'])==radius and trial['accepted_state_fingerprint']==fp(state)
    assert si==len(segments)
    for ti,y,radius in zip(integ['times'],integ['states'],integ['radii']):
     if ti==row['times'][0]:continue
     seg=next(s for s in segments if s['curve']['t0']<=ti<=s['curve']['t1']);c=seg['curve'];t=fq(ti)-fq(c['t0']);theta=t/(fq(c['t1'])-fq(c['t0']))
     error=max(abs(value([fq(x)for x in b],theta)-fq(v))for b,v in zip(c['controls'],y));theory=flow(fq(seg['initial_radius']),fq(seg['defect_upper']),fq(seg['decay_lower']),t)
     assert fq(radius)>=theory+error,('RETURNED_OUTPUT_NOT_ENCLOSED',radius,float(theory+error))
     assert radius<=integ['absolute_budget'];checks['returned_outputs_rational']+=1
    if integ['success']:assert integ['times']==row['times']and integ['final_t']==row['times'][-1]
   if phase=='cal':cal[(row['n'],row['epsilon'],row['arm'],row['tol'],row['workers'])]=r
   else:
    key=(row['n'],row['epsilon'],row['arm'],r['block']);tim[key]=r
    if met['valid']:
     assert len(row['elapsed_ns'])==3 and all(x>0 for x in row['elapsed_ns']);checks['raw_timing_arrays']+=1
 units=json.loads((out/'UNITS.json').read_text())
 for u in units:
  if u['valid']:
   r=tim[(u['n'],u['epsilon'],u['arm'],u['block'])];assert statistics.median(r['row']['elapsed_ns'])==u['median_ns']
 for p in json.loads((out/'PAIRS.json').read_text()):
  ratios=[]
  for block in range(4):
   group={u['arm']:u for u in units if u['n']==p['n']and u['epsilon']==p['epsilon']and u['block']==block};a=group[p['baseline']];b=group[p['candidate']]
   if a['valid']and b['valid']:ratios.append(a['median_ns']/b['median_ns'])
  assert ratios==p['ratios']
  if len(ratios)==4:assert statistics.median(ratios)==p['median_ratio'];checks['paired_medians']+=1
 audit={'status':'PASS_SCOPED_RATIONAL_CERTIFICATE_AND_RAW_READBACK','checks':dict(checks),'verified_objects':len(objects),'distinct_native_proof_records':len(verified_proofs),'proof_scope':'generated reflecting-grid semidiscrete model; polynomial residual coefficients and exponential comparison exactly replayed with Fractions','conditional_assumptions':['IEEE arithmetic without fast-math/FTZ','initial true state in invariant box','declared immutable generated model'],'not_claimed':['arbitrary-model certification','PDE spatial/remap error','independent-personnel/formal proof','universal speed/optimality']}
 (out/'AUDIT.json').write_text(json.dumps(audit,indent=2)+'\n');print(json.dumps(audit))
if __name__=='__main__':main(Path(sys.argv[1]))
