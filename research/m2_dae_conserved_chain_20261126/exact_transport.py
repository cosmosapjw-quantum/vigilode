from fractions import Fraction as Q
from structural import native,br,mag,sub,add,scale,deriv,ev
from exact_native import counts
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
