from pathlib import Path
from fractions import Fraction as Q
from itertools import product
import collections,json,hashlib,math
import numpy as np
from scipy.optimize import brentq
from reference import solve,evaluate
from structural import native,br,mag,sub,add,scale,deriv,ev
from exact_native import audit_run,exact_cells,counts
P=Path(__file__).parent
def load(n,prefix):return [json.loads(l.split(prefix,1)[1])for l in (P/n).read_text().splitlines()if prefix+'{'in l]
rows=load('training.log','CELL ');assert len(rows)==108
fields=['solver','sync','l1_route','order'];cases=['separated','overlap','mixed']
axes=[['Mass','Prepared5','Scalar5'],['BoxPicard','Centered','GreenMoment'],['BoxLift','JointTransport'],['Shared','Correlated']]
assert {(r['case'],*(r[f]for f in fields))for r in rows}=={(c,*k)for c in cases for k in product(*axes)}
q=1/256;end=1/32;alpha=1/128
canonical=[[1.,0.,0.],[0.,0.5,0.],[0.,0.5,1.]]
mixed=[[1023/1024,0.,0.25],[0.,0.5,0.],[1/1024,0.5,0.75]]
def spec(case):
 return (2**-20 if case=='separated'else 2**-17, mixed if case=='mixed'else canonical,q+2**-28 if case=='overlap'else 2*q)
def seeds(rad):
 r=[]
 for i in range(5):
  for j in range(5-i):
   a=rad*i/4;b=rad*j/8;u=1-a-b
   assert Q(u)+Q(a)+Q(b)==1 and Q(1)-Q(u)+Q(b)<=Q(rad)
   r.append([u,b,a])
 return r
refs={}
for case in cases:
 rad,M,q2=spec(case);M=np.array(M)
 for seed in seeds(rad):
  for method in ['Radau','BDF']:
   y=seed if method=='Radau'else seed[:2]
   pre=solve(3e7,(0,end),y,method,3e-13,1e-16)
   def h(t,threshold):return t+alpha*evaluate(pre,t,method)[2]-threshold
   r1=brentq(h,0,end,args=(q,),xtol=5e-324,rtol=4*np.finfo(float).eps)
   r0=brentq(h,0,end,args=(q2,),xtol=5e-324,rtol=4*np.finfo(float).eps)
   assert r1<r0
   x=evaluate(pre,r1,method);rst=M@x
   assert min(rst)>=-1e-15 and abs(sum(rst)-1)<2e-14
   post=solve(6e7,(r1,end),rst if method=='Radau'else rst[:2],method,3e-13,1e-16)
   refs[case,tuple(seed),method]=(pre,post,r0,r1,rst)
def band(a,b):return max(abs(a-b))+2e-13
def in_box(v,box,tol):
 return box[0]-tol<=v<=box[1]+tol
def vector_inside(v,boxes,tol):return len(v)==len(boxes)and all(in_box(x,b,tol)for x,b in zip(v,boxes))
def l1norm(M):
 M=[[Q(x)for x in row]for row in M]
 assert all(sum(M[i][j]for i in range(3))==1 for j in range(3))and min(min(r)for r in M)>=0
 return max(sum(abs(M[i][j]-M[i][2])for i in range(2))for j in range(2))
def audit_transport(r,M):
 tr=r['transport'];prep=r['preparation'];center=list(map(Q,tr['center']))
 box=Q(prep['incoming_radius'])*2
 comp=sum(max(abs(Q(b[0])-c),abs(Q(b[1])-c))for b,c in zip(prep['synchronized_x'],center))
 assert Q(tr['box_l1'])>=box and Q(tr['component_l1'])>=comp and Q(tr['operator_l1'])>=l1norm(M)
 assert tr['selected_l1']==r['incoming_l1']
 if r['l1_route']=='BoxLift':
  assert tr['joint_l1']is None and not tr['cells'] and tr['selected_l1']==tr['box_l1']
  return
 assert tr['joint_l1']==max(c['joint_l1']for c in tr['cells'])
 assert tr['selected_l1']==min(tr['joint_l1'],tr['box_l1'],tr['component_l1'])
 assert len(tr['cells'])==len(prep['cells'])
 for c,tc in zip(tr['cells'],prep['cells']):
  assert c['times']==tc['times'] and c['field_box']==tc['tube']['field']
  s=next(s for s in r['pre_slabs']if s['times']==c['native_times'])
  pp=native(s)[:2];a,b=map(Q,c['native_times']);h=b-a
  l,hi=map(Q,c['times']);u0,u1=(l-a)/h,(hi-a)/h
  sync=Q(r['event'][1]);k=list(map(Q,c['reference_field']))
  assert c['old_l1']==s['certificate']['whole_x_error']
  assert Q(c['mapped_l1'])>=Q(c['old_l1'])*Q(tr['operator_l1'])
  rem=(sync-l)*sum(max(abs(Q(v[0])-k[i]),abs(Q(v[1])-k[i]))for i,v in enumerate(c['field_box']))
  assert Q(c['remainder_l1'])>=rem
  assert Q(c['predictor_l1'])>=max(Q(v[1])for v in c['signed_predictor'])
  assert Q(c['joint_l1'])>=Q(c['predictor_l1'])+Q(c['mapped_l1'])+Q(c['remainder_l1'])
  lift=sub([Q(1)],add(*pp));pp=[*pp,lift];v=[]
  for i in range(2):
   x=[Q(0)]
   for j in range(3):x=add(x,scale(pp[j],Q(M[i][j])))
   x=add(x,[(sync-a)*k[i]-center[i],-h*k[i]])
   v.append(x)
  for idx,(i,j) in enumerate([(1,1),(1,-1),(-1,1),(-1,-1)]):
   poly=add(scale(v[0],Q(i)),scale(v[1],Q(j)));bound=list(map(Q,c['signed_predictor'][idx]))
   # Exact Bernstein derivative sign gives exact extrema at endpoints.
   db=br(deriv(poly),u0,u1-u0)
   if min(db)>=0 or max(db)<=0:
    ends=[ev(poly,u0),ev(poly,u1)]
    assert bound[0]<=min(ends)<=max(ends)<=bound[1]
    counts['signed_exact_monotone']+=1
   else:
    # Sufficient whole polynomial enclosure via independent subdivision.
    vv=[v for j0 in range(16)for v in br(poly,u0+(u1-u0)*Q(j0,16),(u1-u0)/16)]
    if bound[0]<=min(vv)<=max(vv)<=bound[1]:counts['signed_exact_bernstein']+=1
    else:counts['signed_sample_only']+=1
   for n in range(17):
    val=ev(poly,u0+(u1-u0)*Q(n,16));assert bound[0]<=val<=bound[1]
    counts['signed_predictor_samples']+=1
  counts['norm_transfer_exact']+=1
max_ratio=0.;max_spread=0.;max_err=0.;out=[];stops=collections.Counter()
for r in rows:
 case=r['case'];rad,M,q2=spec(case)
 v={k:r[k]for k in ['case',*fields]};v['qualified']=False;v['reason']=r['reason']
 assert r['attempts']<=r['charged_attempts']<=4096
 assert r['winner'] in (None,1)
 assert r['committed']==('post'in r and r['post']['global'] and r['reason'] is None)
 ss=seeds(rad)
 # Exact residual certificate on authentic pre-mode numerical extension.
 last=None;prev=Q(rad)
 for s in r['pre_slabs']:
  c=s['certificate'];assert c and c['global_qualified'] and c['conservation']
  pp=native(s)[:2];counts['exact_coeff_checks']+=5*len(s['native']['y0'])
  assert Q(c['incoming'])==prev
  a,b=map(Q,s['times'])
  if last is not None:assert a==last[0]and [ev(x,Q(0))for x in pp]==last[1]
  else:assert [ev(x,Q(0))for x in pp]==[Q(1),Q(0)]
  prev=exact_cells(s,pp,30000000,'Integral4');last=(b,[ev(x,Q(1))for x in pp])
 for g in r['pre_guards']:
  for slab in g['slabs']:
   assert slab['reduced_lip']<=alpha and slab['algebraic_guard_lip']==0.
 if r['event']:
  assert r['winner']==1 and r['event'][0]<r['event'][1] and r['event'][1]-r['event'][0]<=1e-6
 else:assert not r['committed']
 if r['committed']:
  assert r['preparation']['reason']is None and r['transport']and r['post']['driver_end']==end
  audit_transport(r,M)
  assert r['post']['initial']['x']==r['transport']['center']
  assert r['post']['slabs'][0]['times'][0]==r['event'][1]
  audit_run(r['post'],60000000,'Integral4',r['incoming_l1'])
  v.update({'qualified':True,'incoming_l1':r['incoming_l1'],'bound':r['post']['bound'],'event_width':r['event'][1]-r['event'][0],'attempts':r['attempts'],'charged_attempts':r['charged_attempts'],'operator_l1':r['transport']['operator_l1'],'box_l1':r['transport']['box_l1'],'component_l1':r['transport']['component_l1'],'joint_l1':r['transport']['joint_l1'],'selected_l1':r['transport']['selected_l1'],'cells':len(r['preparation']['cells']),'picard_proofs':sum(len(c['tube']['attempts'])for c in r['preparation']['cells']),'green_calls':sum(c['sync']['green_calls']for c in r['preparation']['cells']),'rhs_proxy':r['pre_work']['driver_rhs']+r['pre_work']['shared_projection']['rhs_count']+r['pre_work']['replay_projection_rhs']+r['post']['counters']['rhs_calls']+r['post']['work']['rhs_count']})
  counts['committed']+=1
 else:
  stops[r['reason']]+=1
 for seed in ss:
  a=refs[case,tuple(seed),'Radau'];b=refs[case,tuple(seed),'BDF']
  tau=a[3];tb=b[3];terr=abs(tau-tb)+2e-14
  assert r['safe']<=tau+terr;counts['safe_checks']+=1
  if r['event']:
   assert in_box(tau,r['event'],terr);counts['event_checks']+=1
  if r['committed']:
   s=r['event'][1];rst=a[4];tol=band(rst,b[4]);cover=False
   for cell in r['preparation']['cells']:
    if not in_box(tau,cell['times'],terr):continue
    cover=True
    assert vector_inside(rst[:2],[v['enclosure']for v in cell['reset']],tol)
    counts['reset_checks']+=2
    for k in range(5):
     t=tau+(s-tau)*k/4
     xa=evaluate(a[1],t,'Radau');xb=evaluate(b[1],t,'BDF');tol=band(xa,xb)
     assert vector_inside(xa[:2],cell['tube']['tube'],tol)
     assert in_box(xa[2],cell['tube']['attempts'][-1]['root_box'][0],tol)
     counts['tube_checks']+=3
    xa=evaluate(a[1],s,'Radau');xb=evaluate(b[1],s,'BDF');tol=band(xa,xb)
    assert vector_inside(xa[:2],cell['sync']['synchronized'],tol);counts['sync_checks']+=2
   assert cover
   xa=evaluate(a[1],s,'Radau');xb=evaluate(b[1],s,'BDF');err=sum(abs(xa[:2]-r['transport']['center']))
   assert err<=r['incoming_l1']+2*band(xa,xb);counts['carry_checks']+=1
  for k,qs in [(0,r['pre_queries'])]+([(1,r['post']['queries']+r['post']['accepted'])]if r['committed']else[]):
   if not qs:continue
   tt=np.array([x['t']for x in qs]);yy=np.array([x['state']for x in qs]);bd=np.array([x['physical_error_upper']for x in qs])
   aa=evaluate(a[k],tt,'Radau').T;bb=evaluate(b[k],tt,'BDF').T
   err=np.max(abs(aa-yy),axis=1);spread=np.max(abs(aa-bb),axis=1)
   assert np.all(err<=bd+spread+2e-13),(v,k,seed,float(max(err-bd-spread)))
   max_ratio=max(max_ratio,float(max(err/np.maximum(bd,1e-300))));max_spread=max(max_spread,float(max(spread)));max_err=max(max_err,float(max(err)))
   counts['reference_queries']+=len(qs)
 out.append(v)
summary={'rows':out,'counts':dict(counts),'stops':dict(stops),'max_reference_ratio':max_ratio,'max_reference_spread':max_spread,'max_reference_error':max_err,'initial_samples':15,'raw_sha256':hashlib.sha256((P/'training.log').read_bytes()).hexdigest()}
(P/'AUDIT.json').write_text(json.dumps(summary,indent=2,allow_nan=False)+'\n')
print({k:v for k,v in summary.items()if k!='rows'})
for c in cases:
 print(c,[(k,sum(r['qualified']for r in out if r['case']==c and r['order']==k))for k in axes[-1]])
