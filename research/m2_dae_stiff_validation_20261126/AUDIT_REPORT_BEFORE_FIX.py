from pathlib import Path
from fractions import Fraction as Q
import json,hashlib,collections,math,mpmath as mp
from structural import proof,summary,full_approx,ev,ceil80
P=Path(__file__).parent;mp.mp.dps=85
def load(name,prefix):return [json.loads(l.split(prefix,1)[1])for l in (P/name).read_text().splitlines()if prefix+'{'in l]
def mq(x):
 if isinstance(x,Q):return mp.mpf(x.numerator)/x.denominator
 return mp.mpf(float(x))
def up(x):
 v=float(x)
 if Q(v)<x:v=math.nextafter(v,math.inf)
 return v
def analytic(case,t):
 t=mq(t);z=1+mp.exp(-1024*t)/16
 if case=='sqrt-stiff':return [z*z,z]
 return [z,z/(1+t/8)]
ref=json.load(open(P/'REFERENCE.json'))
def refs(key):
 if key=='robertson':
  ts=ref['times'];data=ref['values']
 else:ts=ref['hybrid_times'][key];data={k:v[key]for k,v in ref['hybrid_values'].items()}
 fine={t:v for t,v in zip(ts,data['Radau-fine'])}
 spread={t:max(max(abs(a-b)for a,b in zip(v,fine[t]))for v in vs)for t,vs in zip(ts,zip(*data.values()))}
 fine_spread=max(max(abs(a-b)for a,b in zip(x,y))for x,y in zip(data['Radau-fine'],data['BDF-fine']))
 return fine,spread,fine_spread
maps={k:refs(k)for k in ['robertson','pre','post']}
counts=collections.Counter()
def samples(case,run,cert=None,key=None):
 observed=mp.mpf(0);ratio=mp.mpf(0);violation=0;n=0;max_mass=mp.mpf(0);min_state=mp.inf
 if cert is not None:
  bd={(v['kind'],v['t']):v['bound']for v in cert['queries']}
 for k in ['accepted','queries']:
  for a in run[k]:
   state=list(map(mq,a['state']));t=a['t']
   if case=='robertson':
    rm,sp,_=maps[key or 'robertson'];true=list(map(mq,rm[t]));spread=mq(sp[t])
    max_mass=max(max_mass,abs(sum(state)-1));min_state=min(min_state,min(state))
   else:true=analytic(case,t);spread=mp.mpf('1e-78')
   err=max(abs(x-y)for x,y in zip(state,true));observed=max(observed,err);n+=1
   if cert is not None: b=mq(bd[k,t])
   elif run['global']:b=mq(a['physical_error_upper'])
   else:continue
   if case=='robertson':
    # Reference spread is a diagnostic allowance, NOT a certified reference enclosure.
    if err>b+spread+mp.mpf('2e-15'):violation+=1
   else:
    assert err<=b+spread,(case,t,err,b)
   if b>0:ratio=max(ratio,err/b)
 return {'observed_error':float(observed),'sample_checks':n,'diagnostic_violations':violation,'max_observed_ratio':float(ratio),'max_mass_error':float(max_mass),'min_state':None if min_state==mp.inf else float(min_state)}
def receipt(c):
 out=summary(c)
 if c['status']=='certified':
  out.update({'bound':up(c['bound']),'bound_exact':str(c['bound']),'e_out_exact':str(c['e_out']),'endpoint_exact':[str(v)for v in c['endpoint']],
  'cells':[{'times':[str(r['a']),str(r['b'])],'jump':str(r['jump']),'cells':[{k:([str(v)for v in x]if isinstance(x,list)else str(x))for k,x in c.items()}for c in r['cells']]}for r in c['records']],
  'query_bounds':[{'kind':q['kind'],'t':q['t'],'bound':str(q['bound']),'correction':str(q['correction'])}for q in c['queries']]})
 return out
def fail(run):
 if not run['driver_success']:return 'native-driver'
 if not run['pointwise']:return 'pointwise'
 if not run['branch']:return 'branch'
 if not run['global']:
  for s in run['slabs']:
   c=s['certificate']
   if not c or not c['global_qualified']:
    return (c or {}).get('global_reason')or s['reason']or 'global-unknown'
  return 'global-cap'
 return None
rows=load('training.log','CELL ');assert len(rows)==108
routes=['Mass','Reduced3','Reduced5','Prepared5','Scalar5','WarpFolded'];methods=['Native','StructMax','StructL1_4'];cases=['robertson','sqrt-stiff','pencil-stiff']
assert len({(r['case'],r['route'],r['rtol'],r['green'])for r in rows})==108
assert {(r['case'],r['route'],r['rtol'],r['green'])for r in rows}=={(c,r,t,g)for c in cases for r in routes for t in [1e-5,1e-7,1e-9]for g in [False,True]}
allrows=[];struct_counts=collections.Counter()
with (P/'STRUCTURAL_RECEIPTS.jsonl').open('w') as f:
 for row in rows:
  case=row['case'];rtol=row['rtol'];base={k:row[k]for k in ['case','route','rtol','green']}
  if 'error'in row:
   for meth in methods:allrows.append(dict(base,method=meth,qualified=False,reason=row['error']))
   counts['structural_admission_refusals']+=1;continue
  r=row['run'];counts['actual_returned']+=1
  if r['driver_success']:counts['native_success']+=1
  if r['pointwise']:counts['pointwise']+=1
  if r['branch']:counts['branch']+=1
  if r['global']:counts['native_global']+=1
  assert r['driver_end']==row['end']
  if r['slabs']:
   assert r['slabs'][0]['times'][0]==0 and r['slabs'][-1]['times'][1]==row['end']
   assert all(a['times'][1]==b['times'][0]for a,b in zip(r['slabs'],r['slabs'][1:]))
  obs=samples(case,r);counts['native_sample_checks']+=obs['sample_checks']
  assert obs['diagnostic_violations']==0
  b=r['bound'];cost=r['counters']['rhs_calls']+r['work']['rhs_count']
  valid=r['global']and obs['observed_error']<=2*rtol
  shared=dict(base,attempts=r['attempts'],rhs_proxy=cost,slabs=len(r['slabs']),warp_predictions=r['work']['warp_predictions'],prepared_owners=r['work']['prepared_owners'],dictionary_uses=r['work']['dictionary_uses'],**obs)
  allrows.append(dict(shared,method='Native',bound=b,qualified=valid,reason=fail(r)or(None if valid else 'empirical-cap'),proof_cells=len(r['slabs'])))
  initial=[1,0]if case=='robertson'else[17/16]
  for meth in methods[1:]:
   cert=proof(case,r,meth,initial)
   f.write(json.dumps(dict(base,method=meth,proof=receipt(cert)),allow_nan=False)+'\n')
   if cert['status']!='certified':
    allrows.append(dict(shared,method=meth,qualified=False,reason=cert['status']));continue
   assert cert['bound']>=0
   o=samples(case,r,cert);counts['structural_sample_checks']+=o['sample_checks'];counts['exact_coefficient_checks']+=cert['coefficient_checks'];assert o['diagnostic_violations']==0
   n=sum(len(x['cells'])for x in cert['records']);counts['exact_certificate_cells']+=n
   valid=cert['bound']<=Q(1,1000) and o['observed_error']<=2*rtol
   allrows.append(dict(shared,method=meth,bound=up(cert['bound']),bound_exact=str(cert['bound']),qualified=valid,reason=None if valid else ('structural-bound-cap'if cert['bound']>Q(1,1000)else'empirical-cap'),proof_cells=n,max_mu=up(cert['max_mu']),**{k:v for k,v in o.items() if k not in shared}))
   struct_counts[case]+=1
# Independent reference agreement diagnostic: not interval authority.
hy=load('hybrid.log','HYBRID ');assert len(hy)==12
hyrows=[]
with (P/'HYBRID_STRUCTURAL_RECEIPTS.jsonl').open('w')as f:
 for r in hy:
  base={k:r[k]for k in ['route','rtol','tau','end']};chain=r.get('chain',{})
  assert chain.get('stop')!='Horizon'
  for meth in methods[1:]:
   v=dict(base,method=meth,chain_stop=chain.get('stop'),chain_reason=chain.get('reason'),qualified=False)
   if 'pre'not in r or 'post'not in r:
    v['reason']=r.get('error')or r.get('post_error')or 'missing numeric segment';hyrows.append(v);continue
   pre,post=r['pre'],r['post']
   assert pre['driver_end']==r['tau'] and post['driver_end']==r['end']
   pc=proof('robertson',pre,meth,[1,0]);assert pc['status']=='certified'
   reset=[pc['endpoint'][0],pc['endpoint'][1]/2];cx=list(map(Q,r['reset_center']));assert cx==list(map(Q,post['initial']['x']))
   carry=ceil80(pc['e_out']+sum(abs(a-b)for a,b in zip(reset,cx)))
   qc=proof('robertson',post,meth,cx,60000000,carry);assert qc['status']=='certified'
   a,b=samples('robertson',pre,pc,'pre'),samples('robertson',post,qc,'post')
   counts['hybrid_sample_checks']+=a['sample_checks']+b['sample_checks']
   assert a['diagnostic_violations']==0 and b['diagnostic_violations']==0
   assert not post['global'] # unknown incoming authority in the frozen post solve
   v.update({'qualified':max(pc['bound'],qc['bound'])<=Q(1,1000)and max(a['observed_error'],b['observed_error'])<=2*r['rtol'],'pre_bound':up(pc['bound']),'post_bound':up(qc['bound']),'carry':up(carry),'carry_exact':str(carry),'observed_error':max(a['observed_error'],b['observed_error']),'attempts':pre['attempts']+post['attempts'],'pre_mass_error':a['max_mass_error'],'post_mass_error':b['max_mass_error'],'pre_min_state':a['min_state'],'post_min_state':b['min_state']})
   f.write(json.dumps(dict(v,pre_proof=receipt(pc),post_proof=receipt(qc)),allow_nan=False)+'\n')
   hyrows.append(v)
result={'logical_candidates':len(allrows),'native_calls':len(rows),'counts':dict(counts),'structural_certificates':dict(struct_counts),'fine_reference_discrepancy':{k:m[2]for k,m in maps.items()},'rows':allrows,'hybrid_rows':hyrows,'hybrid_actual_calls':len(hy),'hybrid_native_stops':dict(collections.Counter(r.get('chain',{}).get('stop','error')for r in hy))}
(P/'AUDIT.json').write_text(json.dumps(result,indent=2,allow_nan=False)+'\n')
print({k:v for k,v in result.items()if k not in ('rows','hybrid_rows')})
for case in cases:
 for method in methods:
  s=[r for r in allrows if r['case']==case and r['method']==method]
  print(case,method,sum(r['qualified']for r in s),'/',len(s),'certified',sum(r.get('bound')is not None for r in s))
print('hybrid offline',sum(r['qualified']for r in hyrows),'/',len(hyrows))
