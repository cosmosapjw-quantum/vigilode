"""Compare exact original dataset key domains and report observed, not qualified, performance.
No substitutions for absent outputs; no extrapolation or ranking disputed references.
"""
import json,math,statistics,hashlib,collections
from pathlib import Path
root=Path(__file__).resolve().parent
spec=[
 ('cross_rust','cross_solver_v1_rust.json',126),
 ('cross_julia','cross_solver_v1_julia.json',45),
 ('spatial_v3','spatial_battery_v3_rust.json',378),
 ('opt_battery','opt_battery_head.json',96),
 ('full_rust','full_scope_rust.json',242),
 ('full_ens','full_scope_ensemble_unpinned.json',3),
 ('full_julia','full_scope_julia.json',112),
 ('full_scipy','full_scope_scipy.json',30),
]
def rows(d):return d.get('rows',d.get('records',d.get('results',[])))
def key(r):return (r.get('problem',r.get('problem_id')),r.get('arm',r.get('algorithm',r.get('route_policy'))),r['rtol'])
def status(r):
 s=r.get('status')
 if isinstance(s,dict):return s.get('kind')
 if isinstance(s,str):return s
 if 'ok' in r:return 'success' if r['ok'] else 'failed'
 return 'not-recorded'
def time(r):
 v=r.get('wall_ms_median',r.get('time_median_ms'))
 if v is None and r.get('wall_ns_median_of_5') is not None:v=r['wall_ns_median_of_5']/1e6
 return v
def error(r):
 for k in ('error_common','err_wrms_at_T'):
    if r.get(k) is not None:return r[k]
 return None
def valid(x):return isinstance(x,(float,int)) and not isinstance(x,bool) and math.isfinite(x)
def geomean(vals):return math.exp(statistics.mean(map(math.log,vals))) if vals else None
def ref_summary(d):
 refs=d.get('references')
 if isinstance(refs,list):
    return {r['problem']:{'disputed':r.get('disputed'),'status':r.get('status'),'crosscheck':r.get('crosscheck') and str(r['crosscheck'])[:300]} for r in refs}
 return {}
def ref_arrays(d):
 refs=d.get('references')
 if isinstance(refs,list):return {r['problem']:r.get('states') for r in refs}
 if isinstance(refs,dict):return refs
 return {}
def flatten(x):
 if isinstance(x,list):
    for v in x:yield from flatten(v)
 elif isinstance(x,(int,float)):yield x
def compare_ref(a,b):
 x,y=ref_arrays(a),ref_arrays(b);out={}
 for k in sorted(x.keys()&y.keys()):
    u=list(flatten(x[k]));v=list(flatten(y[k]))
    out[k]={'old_values':len(u),'new_values':len(v),'exact_equal':x[k]==y[k],'max_abs_delta':max((abs(a-b) for a,b in zip(u,v)),default=0.) if len(u)==len(v) and u else None}
 return out
report={}
all_old=0;all_new=0;all_joined=0
for label,name,n in spec:
 old=json.loads((root/'original'/name).read_bytes()); aa=rows(old);assert len(aa)==n
 a={key(r):r for r in aa};assert len(a)==n,(name,'duplicate old keys')
 all_old+=n
 path=root/'run'/name
 if not path.exists():
    report[label]={'original_rows':n,'new_rows':0,'status':'NOT_COMPLETED','file':name}
    continue
 new=json.loads(path.read_bytes());bb=rows(new);b={key(r):r for r in bb};assert len(b)==len(bb),(name,'duplicate new keys')
 shared=a.keys()&b.keys();all_new+=len(bb);all_joined+=len(shared)
 changed=[{'key':k,'old':status(a[k]),'new':status(b[k])} for k in sorted(shared) if status(a[k])!=status(b[k])]
 cmp=[]
 for k in sorted(shared):
    r,s=a[k],b[k];t0,t1=time(r),time(s);e0,e1=error(r),error(s)
    cmp.append(dict(problem=k[0],arm=k[1],rtol=k[2],old_status=status(r),new_status=status(s),
                    old_ms=t0,new_ms=t1,old_error=e0,new_error=e1,
                    old_extra=r.get('extra',{}),new_extra=s.get('extra',{}),
                    old_callbacks=r.get('callbacks',{}),new_callbacks=s.get('callbacks',{}),
                    old_nf=r.get('nf',r.get('f_evals')),new_nf=s.get('nf',s.get('f_evals')),
                    old_njvp=r.get('njvp',r.get('jvp_calls')),new_njvp=s.get('njvp',s.get('jvp_calls')),
                    old_accepted=r.get('accepted',r.get('naccept')),new_accepted=s.get('accepted',s.get('naccept')),
                    old_rejected=r.get('rejected',r.get('nreject')),new_rejected=s.get('rejected',s.get('nreject')),
                    old_error_dense=r.get('err_max_dense_100pts'),new_error_dense=s.get('err_max_dense_100pts'),
                    old_work=r.get('work',{}),new_work=s.get('work',{})))
 er=[c for c in cmp if valid(c['old_error']) and valid(c['new_error'])]
 vig=[c for c in cmp if c['arm'].startswith('vigilode') and c['rtol']==1e-6 and c['old_status']==c['new_status']=='success' and valid(c['old_ms']) and valid(c['new_ms']) and c['old_ms']>0 and c['new_ms']>0]
 matched=collections.Counter(str(r.get('reason')) for r in new.get('matched',[]))
 report[label]=dict(original_rows=n,new_rows=len(bb),common_keys=len(shared),key_match=a.keys()==b.keys(),file=name,
   original_status=dict(collections.Counter(status(r) for r in aa)),new_status=dict(collections.Counter(status(r) for r in bb)),
   changed_status=changed,missing_keys=sorted(a.keys()-b.keys()),extra_keys=sorted(b.keys()-a.keys()),
   equal_error_rows=sum(c['old_error']==c['new_error'] for c in er),finite_error_rows=len(er),
   reference_comparison=compare_ref(old,new),references=ref_summary(new),
   protocol_violation_rows=new.get('protocol_violation_rows'),matched_reasons=dict(matched),
   geometric_old_over_new_vigilode_1e6=geomean([c['old_ms']/c['new_ms'] for c in vig]),
   source_identity={k:v for k,v in new.get('provenance',{}).get('source_identity',{}).items() if k!='manifest'},
   file_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),comparisons=cmp)
 if label=='spatial_v3':
    ratios={}
    for arm in sorted({k[1] for k in b}):
       rr=[]
       for k,r in b.items():
           if k[1]!=arm or k[2]!=1e-6:continue
           q=b.get((k[0],'vigilode-banded-dense',k[2]))
           if q and status(r)==status(q)=='success':rr.append(time(r)/time(q))
       if rr:ratios[arm]={'cells':len(rr),'comparator_over_banded_dense':geomean(rr)}
    report[label]['nominal_spatial_ratios']=ratios
 if label=='full_rust':
    report[label]['capability_refusals']=new.get('capability_refusals')
    report[label]['vg1_allocation_audit']=new.get('vg1_allocation_audit')
summary=dict(schema='vigilode.adversarial-rebaseline.v1',measured_commit=(root/'receipts/MEASURED_COMMIT.txt').read_text().strip(),original_rows=all_old,new_rows=all_new,common_keys=all_joined,timing_authority='diagnostic_only',datasets=report)
(root/'COMPARISON.json').write_text(json.dumps(summary,indent=2,sort_keys=True,allow_nan=False)+'\n')
for label,d in report.items():
 print(label,{k:d[k] for k in ('original_rows','new_rows','common_keys','key_match','original_status','new_status','equal_error_rows','finite_error_rows','geometric_old_over_new_vigilode_1e6') if k in d})
 if d.get('changed_status'):print('  status changes',d['changed_status'])
 if d.get('reference_comparison'):print('  reference',d['reference_comparison'])
 if d.get('references'):print('  disputed',sum(r['disputed'] is True for r in d['references'].values()),'/',len(d['references']))
 if d.get('matched_reasons'):print('  matched reasons',d['matched_reasons'])
print('TOTAL',all_old,all_new,all_joined)
