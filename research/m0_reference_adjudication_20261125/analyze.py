"""Independent reduction of actual raw M0 receipts. Never executes a solver."""
from pathlib import Path
from collections import defaultdict
import hashlib,json,math,sys,re
import mpmath as mp
import numpy as np
mp.mp.dps=85
D=Path(__file__).resolve().parent
threshold=1e-10
def read(p):return json.loads(p.read_bytes())
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def dec(s):
 return mp.mpf(re.sub(r'\x60{1,2}[0-9.+-]*','',s).replace('*^','e'))
def shape(r,s,old):
 if not r.get('ok',False):return False
 a=np.array(r.get('states',[]),dtype=float)
 return a.shape==(101,s['n']) and np.all(np.isfinite(a)) and np.array_equal(a[0],np.array(old['states'][0]))
def high_states(r):
 if 'states_decimal' not in r:return [[mp.mpf(float(x)) for x in v] for v in r['states']]
 a=[[dec(x) for x in v] for v in r['states_decimal']]
 assert len(a)==101 and all(len(v)==len(w) for v,w in zip(a,r['states']))
 for v,w in zip(a,r['states']):
  for x,y in zip(v,w):
   assert mp.isfinite(x)
   f=float(x)
   assert f==y or abs(f-y)<=math.ulp(y),("decimal representation mismatch",f,y)
 return a
def directions(a,b,s,high=False):
 if high:
  a=high_states(a);b=high_states(b);ss=[mp.mpf(float(x)) for x in s['atol_scale']];n=len(ss)
  def calc(a,b):return float(max(mp.sqrt(sum(((x-y)/(sc+abs(y)))**2 for x,y,sc in zip(u,v,ss))/n) for u,v in zip(a,b)))
  return [calc(a,b),calc(b,a)]
 a=np.array(a['states'],dtype=float);b=np.array(b['states'],dtype=float);ss=np.array(s['atol_scale'],dtype=float)
 def calc(a,b):return float(np.max(np.sqrt(np.mean(((a-b)/(ss+abs(b)))**2,axis=1))))
 return [calc(a,b),calc(b,a)]
files=sorted((D/'independent').glob('*.jsonl'))
wl_files=sorted((D/'independent').glob('*.wl.*.json'))
families=defaultdict(list);raw=[]
for f in files:
 for i,l in enumerate(f.read_text().splitlines(),1):
  if not l.startswith('{'):continue
  r=json.loads(l);r['_source']=str(f.relative_to(D));r['_row']=i;r['_sha256']=sha(f);raw.append(r)
  if f.name=='julia_big.jsonl':
   r['_api_attempt']='vector-atol-incompatible'
   continue
  families[(r['problem'],r['family'])].append(r)
for f in wl_files:
 r=read(f);r['_source']=str(f.relative_to(D));r['_row']=1;r['_sha256']=sha(f);raw.append(r);families[(r['problem'],r['family'])].append(r)
for key,rows in families.items():
 if all('rtol' in r for r in rows):rows.sort(key=lambda r:-r['rtol'])
 elif all('goal' in r for r in rows):rows.sort(key=lambda r:r['goal'])
def desc(r):
 return {k:v for k,v in r.items() if k not in ('states','states_decimal')}
def ladder(s,old,tag,fa,fb,high=False,rtol_a=None,rtol_b=None):
 a=families[(s['id'],fa)];b=families[(s['id'],fb)]
 if len(a)!=3 or len(b)!=3:return {'protocol':tag,'families':[fa,fb],'status':'INCOMPLETE','level_counts':[len(a),len(b)],'records':[list(map(desc,a)),list(map(desc,b))]}
 if rtol_a is not None:assert [x['rtol'] for x in a]==rtol_a
 if rtol_b is not None:assert [x['rtol'] for x in b]==rtol_b
 valid=[all(shape(r,s,old) for r in q) for q in (a,b)]
 result={'protocol':tag,'families':[fa,fb],'level_counts':[3,3],'records':[list(map(desc,a)),list(map(desc,b))],'precision_metric':'mpmath85' if high else 'binary64'}
 if not all(valid):
  result['status']='INCOMPLETE_OR_INVALID';return result
 cross=[directions(x,y,s,high) for x,y in zip(a,b)]
 ref=[[directions(q[i],q[i+1],s,high) for i in (0,1)] for q in (a,b)]
 mx=max([v for p in cross for v in p]+[v for k in ref for p in k for v in p])
 result.update(cross=cross,successive=ref,max_discrepancy=mx,finest_discrepancy=max(cross[-1]),status='PASS_EMPIRICAL' if mx<=threshold else 'FAILED_METRIC')
 return result
reports=[];newrefs=[];historic_count=0
oldroot=Path('/home/cosmosapjw/vigilode-preserved/20261125-adversarial-rebaseline/run')
for name,oldfile in [('canonical','cross_solver_v1_rust.json'),('spatial','spatial_battery_v3_rust.json')]:
 old=read(oldroot/oldfile);replay=read(D/'replay'/f'{name}.json')
 assert replay['provenance']['source_identity']['status']=='VERIFIED_BUILD_SOURCE'
 assert replay['plan']['corpus']==old['corpus']
 oldmap={r['problem']:r for r in old['references']}
 remap={r['problem']:r for r in replay['references']}
 grids={r['problem']:r['grid'] for r in replay['plan']['cases']}
 for s in replay['plan']['corpus']['problems']:
  id=s['id'];o=oldmap[id];r=remap[id];historic_count+=int(o['disputed'])
  assert r['disputed']==o['disputed'] and r['states']==o['states'],id
  assert r['independent_protocol']==o['independent_protocol'],id
  assert r['status']['kind']=='success' and len(r['states'])==101
  if name=='spatial':
   ls=[ladder(s,r,'spatial-shared-rhs-v1','scipy-dop853','julia-vern9',rtol_a=[1e-12,1e-13,3e-14],rtol_b=[1e-12,1e-13,3e-14])]
   chosen=ls[0];family='julia-vern9'
  elif id=='brusselator-1d-n100':
   ls=[ladder(s,r,'uncapped-v1','scipy-dop853','julia-vern9'),ladder(s,r,'capped-v2','scipy-dop853-capped','julia-vern9-capped'),ladder(s,r,'capped-orthogonal-radau','scipy-radau-capped','julia-vern9-capped')]
   chosen=ls[1];family='julia-vern9-capped'
  else:
   ls=[ladder(s,r,'extended-state-v1','julia-radau9-bigfloat','wolfram-stiffness-extrapolation',True),ladder(s,r,'canonical-endpoint-v2','julia-radau9-bigfloat','scipy-radau-grid-endpoints',True,rtol_a=[1e-15,1e-17,1e-19],rtol_b=[1e-13,5e-14,2.5e-14])]
   if id=='oregonator':
    ls.append(ladder(s,r,'direct-high-precision-v3','direct-radau7-mp','scipy-radau-grid-endpoints',True,rtol_a=[1e-15,1e-17,1e-19],rtol_b=[1e-13,5e-14,2.5e-14]));chosen=ls[-1];family='direct-radau7-mp'
   else:chosen=ls[1];family='julia-radau9-bigfloat'
  record={'problem':id,'cohort':name,'n':s['n'],'historical_disputed':o['disputed'],'historical_replay_equal':True,'historical_levels':r['independent_protocol']['levels'],'protocols':ls,'selected_protocol':chosen['protocol'],'new_status':'REFERENCE_DISPUTED'}
  if chosen['status']=='PASS_EMPIRICAL':
   f=families[(id,family)][-1]
   record.update(new_status='EMPIRICAL_QUALIFIED_NEW_PROTOCOL',selected_family=family,selected_max_discrepancy=chosen['max_discrepancy'],finest_discrepancy=chosen['finest_discrepancy'],old_reference_difference=directions(f,r,s,'states_decimal' in f))
   newrefs.append({'problem':id,'cohort':name,'grid':grids[id],'states':f['states'],'states_decimal':f.get('states_decimal'),'family':family,'source':desc(f),'protocol':chosen['protocol'],'historical_disputed':o['disputed'],'status':record['new_status'],'evidence':'empirical grid agreement, not rigorous error enclosure'})
   if id=='robertson':record['invariant_max_abs']=max(abs(sum(v)-1) for v in f['states'])
   if id=='hires':record['invariant_max_abs']=max(abs(v[6]+v[7]-0.0057) for v in f['states'])
   ex=families.get((id,'scipy-expm-exact-grid'),[])
   if ex:
    assert len(ex)==1 and shape(ex[0],s,r)
    record['linear_action_exact_grid_diagnostic']=directions(f,ex[0],s)
  reports.append(record)
assert len(reports)==14 and historic_count==12
summary={'schema':'vigilode.m0-adjudication.v1','historical_original_rows':1032,'historical_expanded_fresh_rows':2076,'historical_disputed':12,'original_replay_exact':14,'threshold':threshold,'new_qualified':len(newrefs),'new_unresolved':14-len(newrefs),'cases':reports,'raw_execution_rows':len(raw),'raw_ok_flags':sum(r.get('ok',False) for r in raw),'api_failures':sum(r.get('_api_attempt') is not None for r in raw),'source_sha256':{str(f.relative_to(D)):sha(f) for f in files+wl_files},'qualification':'empirical finite-grid only; old disputes not modified; no matched-error reclassification'}
for f,doc in [('SUMMARY.json',summary),('ADJUDICATED_REFERENCES.json',{'schema':'vigilode.empirical-reference.v1','references':newrefs,'threshold':threshold,'no_historical_mutation':True})]:
 (D/f).write_text(json.dumps(doc,indent=2,sort_keys=True,allow_nan=False)+'\n')
for r in reports:
 print(r['problem'],r['new_status'],r.get('selected_max_discrepancy'),r.get('finest_discrepancy'),r.get('old_reference_difference'),[(x['protocol'],x['status'],x.get('max_discrepancy')) for x in r['protocols']])
print("SUMMARY",len(newrefs),14-len(newrefs),len(raw),sum(r.get('ok',False) for r in raw))
