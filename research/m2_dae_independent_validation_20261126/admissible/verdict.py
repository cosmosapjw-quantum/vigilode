"""One-shot preregistered M2 challenge; no tuning and no protected holdout data."""
from pathlib import Path
from fractions import Fraction as Q
from itertools import product
from math import isqrt
import collections,hashlib,json,sys,traceback
import numpy as np
from scipy.integrate import solve_ivp
import scipy
from reference import rhs3,rhs2,evaluate
from structural import native,ev
from exact_native import exact_cells,counts as exact_counts
from exact_phase import query_check
P=Path(__file__).parent
S=json.load(open(P/'CASE_SPEC.json'))
specs={x['id']:x for x in S['hybrid']}
A=S['analytic'];ref=S['reference'];eps=ref['state_allowance'];teps=ref['clock_allowance']
C=collections.Counter();fail=[];inconclusive=[];rows=[];ref_stats=collections.Counter()
max_spread=0.;max_clock_spread=0.;max_ratio=0.
def check(b,label):
 if not b:raise AssertionError(label)
def report_failure(key,e):
 fail.append({'case':key,'kind':'assertion' if isinstance(e,AssertionError) else 'verdict_exception','message':str(e)[:1400],'location':traceback.format_exc(limit=3)[-2000:]})
def exact_encl(box,a,b):check(Q(box[0])<=a<=b<=Q(box[1]),('directed enclosure',box,str(a),str(b)))
def pair(v):
 check(0<=v['returned']<=v['started'],('count',v))
def native_work(n):
 for key in ['initialization','point_projection','scan_nodes','replay_nodes']:
  w=n[key]
  for k in ['calls','evaluations','frozen','roots','linear']:pair(w[k])
  check(w['rhs_returned']<=w['rhs_requested'],'projection returned>requested')
  check(w['rhs_requested']==w['successful_rhs_requested']+w['refused_rhs_requested'],'projection partition')
 for k in ['scan_reconstruction','replay_reconstruction','scan_certificate','replay_certificate','guard_build','warp_setup','difference_comparisons']:pair(n[k])
 s=n['stats']
 if s is not None:
  check(n['driver_invoked'] and s['attempts']==s['accepted_steps']+s['rejected_steps'],'native Stats partition')
  C['native_stats']+=1;C['native_attempts']+=s['attempts'];C['accepted']+=s['accepted_steps'];C['rejected']+=s['rejected_steps']
  return s['attempts']
 if n['driver_invoked']:C['native_unknown']+=1;return None
 C['pre_driver_zero']+=1;return 0
def check_audit(a,w,atlas):
 check(a['admitted'],'returned but not admitted')
 check(sum(n['charged_native_attempts']for n in a['native'])==w['charged_native_attempts'],'native debit')
 check(sum(n['charged_scan_slabs']for n in a['native'])==w['charged_scan_slabs'],'slab debit')
 for n in a['native']:
  m=native_work(n['native']);check(m==n['measured_attempts'],'native measured identity')
  if n['returned']:check(m==n['charged_native_attempts'],'returned debit mismatch')
  else:check(n['charged_native_attempts']==n['native_quota']and n['charged_scan_slabs']==n['slab_quota'],'refused quota mismatch')
 if atlas:
  check(a['committed']==w['committed'] and a['cache_hits']==w['cache_hits'],'atlas commits/cache')
 else:
  check(len(a['transitions'])==w['transitions_attempted'],'transition attempts')
  check(sum(t['charged_cells']for t in a['transitions'])==w['charged_reset_cells'],'reset debit')
  check(sum(t['work']['gap_checks']['started']for t in a['transitions'])==w['gap_checks'],'gap debit')
  check(sum(t['work']['committed']for t in a['transitions'])==w['transitions_committed'],'reset commits')
 C['audit_reconciliations']+=1
def samples(s):
 x=list(map(Q,s['initial']));r=Q(s['radius'])
 out=[]
 for du,dv in [(0,0),(r,0),(-r,0),(0,r),(0,-r)]:
  y=[x[0]+du,x[1]+dv,x[2]-du-dv]
  check(sum(y)==1 and min(y)>=0,'bad preregistered simplex vertex')
  out.append(tuple(map(float,y)))
 return out
def guard_value(g,t,y):
 return g['value']-y[1] if g['kind']=='v'else t-g['value']
def reference_trace(s,y0,method):
 # Independently integrate the continuous equations and locate first guard
 # crossings; never use the candidate's event interval/selected mode.
 end=s['end'];a=0.;mode=0;y=np.array(y0);out=[]
 for _ in range(64):
  m=s['modes'][mode];f,j=(rhs3(m['rate'])if method=='Radau'else rhs2(m['rate']))
  event=[]
  for g in m['guards']:
   def fun(t,y,g=g):return guard_value(g,t,y)
   fun.direction=1;fun.terminal=False;event.append(fun)
  check(all(guard_value(g,a,y)<0 for g in m['guards']),'reference starts with active guard')
  r=solve_ivp(f,(a,end),y if method=='Radau'else y[:2],method=method,jac=j,dense_output=True,events=event or None,rtol=ref['rtol'],atol=ref['atol'])
  check(r.success,('reference solver',s['id'],method,r.message))
  ref_stats['solves']+=1;ref_stats['steps']+=len(r.t)-1
  candidates=[(float(ts[0]),k)for k,ts in enumerate(r.t_events or [])if len(ts)]
  t,b=(min(candidates)if candidates else(end,None))
  tied=[k for tt,k in candidates if tt==t]
  out.append({'mode':mode,'start':a,'end':t,'sol':r,'guards':tied,'winner':b})
  if b is None or t>=end or s['id']=='exact_tie':return out
  ed=m['edges'][b];matrix=np.array(ed['matrix'])
  y=matrix@evaluate(r,t,method)
  if all(row[0]==row[1]==row[2]for row in ed['matrix']):y=matrix[:,0].copy()
  a=t;mode=ed['target']
 raise AssertionError('reference transition cap')
refs={}
for s in specs.values():
 for y in samples(s):
  for method in ['Radau','BDF']:
   try:refs[s['id'],y,method]=reference_trace(s,y,method)
   except Exception as e:report_failure(['reference',s['id'],method],e)
def traces(case,y,i):
 a=refs[case,y,'Radau'];b=refs[case,y,'BDF']
 check(i<len(a)and i<len(b),('reference epoch missing',case,i,len(a),len(b)))
 return a[i],b[i]
def check_event(s,i,mode,target,guard,root,committed):
 global max_clock_spread
 for y in samples(s):
  a,b=traces(s['id'],y,i);check(a['mode']==b['mode']==mode,'event source mode')
  check(a['winner'] is not None and b['winner'] is not None,'candidate event without reference crossing')
  if committed:
   check(s['id']!='exact_tie' and a['winner']==b['winner']==guard and s['modes'][mode]['edges'][guard]['target']==target,'incorrect committed guard/mode')
  d=abs(a['end']-b['end']);max_clock_spread=max(max_clock_spread,d)
  check(all(root[0]-d-teps<=x<=root[1]+d+teps for x in [a['end'],b['end']]),('reference event outside bracket',s['id'],i,root,a['end'],b['end']))
  C['event_reference_checks']+=1
def check_state(s,i,mode,t,box=None,state=None,bound=None):
 global max_spread,max_ratio
 for y in samples(s):
  a,b=traces(s['id'],y,i);check(a['mode']==b['mode']==mode,'query reference mode')
  dt=teps+abs(a['start']-b['start'])+abs(a['end']-b['end'])
  check(all(v['start']-dt<=t<=v['end']+dt for v in [a,b]),('query outside reference physical epoch',s['id'],i,t,a['start'],a['end'],b['start'],b['end']))
  x=evaluate(a['sol'],min(a['end'],max(a['start'],t)),'Radau')
  z=evaluate(b['sol'],min(b['end'],max(b['start'],t)),'BDF')
  d=float(np.max(np.abs(x-z)));max_spread=max(max_spread,d)
  if box is not None:
   check(all(l-d-eps<=v<=h+d+eps for vv in [x,z] for v,(l,h)in zip(vv,box)),('reference phase state outside',s['id'],i,t,x.tolist(),box))
   C['atlas_reference_components']+=6
  else:
   check(bound is not None,'missing physical error')
   err=float(max(np.max(np.abs(x-state)),np.max(np.abs(z-state))))
   check(err<=bound+d+eps,('reference state outside error',s['id'],i,t,err,bound))
   if bound>0:max_ratio=max(max_ratio,err/bound)
   C['joint_reference_components']+=6
def native_prefix(slabs,cert,radius,center,rate):
 out={};prev=Q(radius);last=None
 for s in slabs:
  c=s['certificate']
  if not c or not c['global_qualified'] or not c.get('conservation'):C['unqualified_slab_prefix_ends']+=1;break
  pp=native(s)[:2];C['exact_native_coefficients']+=5*len(s['native']['y0'])
  a,b=map(Q,s['times']);check(a<b,'native times')
  check(Q(c['incoming'])==prev,'native incoming radius')
  if last is None:check([ev(x,Q(0))for x in pp]==list(map(Q,center)),'native center')
  else:check(a==last[0] and [ev(x,Q(0))for x in pp]==last[1],'native continuity')
  prev=exact_cells(s,pp,rate,cert);last=(b,[ev(x,Q(1))for x in pp]);out[tuple(s['times'])]=(pp,c)
 return out
fields=['case','solver','cert','order','method']
def key(r):return tuple(r[k]for k in fields)
def check_hybrid(r):
 s=specs[r['case']];atlas=r['method'].startswith('Atlas');a=r['audit'];out={k:r[k]for k in fields}
 out.update({'qualified':False,'error':r.get('error'),'commits':0,'alloc_requests':sum(r['allocation'][k]for k in ['alloc','zeroed','realloc']),'query_requests':0,'query_returns':0,'last_width':None})
 if 'error'in r:
  check(not a['admitted'] or s['id']!='clock_mix' or not atlas,'unsupported atlas admitted')
  if atlas and s['id']=='clock_mix':
   check(not a['admitted']and not a['native'] and 'phase atlas requires state-only guards' in r['error'],'unsupported atlas specific admission');C['unsupported_atlas_refusals']+=1
  out['stop']='AdmissionErr';return out
 check(not(atlas and s['id']=='clock_mix'),'unsupported atlas returned')
 v=r['atlas'if atlas else'joint'];w=v['work']['phase'if atlas else'common'];check_audit(a,w,atlas)
 out['stop']=v['stop'];out['reason']=v['reason'];out['attempts']=w['native_attempts']
 if atlas:
  ph=v['phases'];es=v['epochs'];events=v['events']
  certs=[native_prefix(p['slabs'],r['cert'],p['initial']['l1_radius'],p['initial']['center'],s['modes'][p['mode']]['rate'])for p in ph]
  check(w['committed']==sum(e['event']['committed']for e in events),'atlas event commits')
  check(w['cache_hits']==sum(e['phase']['cached']for e in es),'cache hits')
  out['commits']=w['committed'];seen={}
  for i,e in enumerate(es):
   p=ph[e['phase']['phase_index']];z=e['phase'];check(e['mode']==p['mode'],'phase mode identity')
   check(z['clock']==([0.,0.]if i==0 else events[i-1]['event']['absolute']),'phase clock chain')
   check(z['relative_safe']==p['safe'] and Q(z['absolute_safe'])<=Q(z['clock'][0])+Q(z['relative_safe']),'phase safe time')
   k=(p['mode'],tuple(p['initial']['center']))
   if z['cached']:check(k in seen and seen[k]==z['phase_index'],'cache key');C['cache_identity']+=1
   elif i and r['method']=='AtlasReuse' and p['winner'] is not None:check(k not in seen,'cache miss repeated key');seen[k]=z['phase_index']
   if i:check(events[i-1]['event']['committed'] and e['mode']==events[i-1]['target'] and p['initial']['center']==events[i-1]['reset_center'],'phase reset continuity')
  for i,e in enumerate(events):
   z=e['event'];p=ph[z['phase_index']];c=es[i]['phase']['clock'];ed=s['modes'][e['from']]['edges'][z['guard']]
   check(e['target']==ed['target'] and e['reset_center']==[row[0]for row in ed['matrix'][:2]],'reset map')
   exact_encl(z['absolute'],Q(c[0])+Q(z['relative'][0]),Q(c[1])+Q(z['relative'][1]))
   check(Q(z['width_upper'])>=Q(z['absolute'][1])-Q(z['absolute'][0]),'event width')
   C['clock_sums']+=1
   if z['committed']:
    check_event(s,i,e['from'],e['target'],z['guard'],z['absolute'],True)
    for j,g in enumerate(e['reset_guards']):
     val=Q(s['modes'][e['target']]['guards'][j]['value'])-Q(e['reset_center'][1])
     exact_encl(g['enclosure'],val,val);check(g['enclosure'][1]<0,'reset guard negative');C['reset_guard_checks']+=1
  if s['id']=='exact_tie':check(not events and all(p['winner']is None for p in ph),'tied atlas event authority')
  groups={}
  for q in v['queries']:
   i=q['epoch'];e=es[i];p=ph[e['phase']['phase_index']];out['query_requests']+=1
   if 'query'not in q['result']:C['atlas_query_refused']+=1;continue
   x=q['result']['query'];out['query_returns']+=1
   query_check(x,p,e['phase'],certs[e['phase']['phase_index']])
   check_state(s,i,e['mode'],q['t'],box=x['enclosure'])
   groups.setdefault((i,q['t']),{})[q['route']]=x
   if q['route']=='Centered' and q['t']==s['end']:out['last_width']=max(h-l for l,h in x['enclosure'])
  for g in groups.values():
   if len(g)==2:
    check(all(a[0]<=b[0]<=b[1]<=a[1] for a,b in zip(g['Horner']['enclosure'],g['Centered']['enclosure'])),'centered vs Horner enclosure');C['centered_pairs']+=1
 else:
  es=v['epochs'];events=v['events'];out['commits']=w['transitions_committed']
  check(w['transitions_committed']==sum(e['committed']for e in events),'chain event commits')
  if s['id']=='exact_tie':check(not events,'tied chain event authority')
  for i,e in enumerate(events):
   if e['committed']:
    check_event(s,i,e['from'],e['target'],e['guard'],e['event_time'],True)
    check(e['transport'] is not None and e['reset'] is not None,'committed without reset/transport')
    check(all(g['enclosure'][1]<0 for g in e['gap']),'committed nonnegative gap')
  for i,e in enumerate(es):
   native_prefix(e['slabs'],r['cert'],e['radius'],e['center'],s['modes'][e['mode']]['rate'])
   if i:check(events[i-1]['committed'] and e['start']==events[i-1]['event_time'][1]and e['radius']==events[i-1]['transport']['selected_l1'],'joint restart continuity')
   for q in e['queries']:
    out['query_requests']+=1
    if 'query'not in q['result']:C['joint_query_refused']+=1;continue
    x=q['result']['query'];out['query_returns']+=1;check_state(s,i,e['mode'],q['t'],state=x['state'],bound=x['physical_error_upper'])
    if q['t']==s['end']:out['last_width']=2*x['physical_error_upper']
 if v['stop']=='Horizon':
  check(s['id']!='exact_tie','tie falsely horizon')
  check(out['last_width'] is not None,'Horizon without final query')
  out['qualified']=out['query_returns']==out['query_requests']
  if not out['qualified']:inconclusive.append({'case':key(r),'reason':'Horizon with refused advertised query'})
 C['hybrid_query_requests']+=out['query_requests'];C['hybrid_query_returns']+=out['query_returns']
 return out
def exp_neg(s):
 check(0<=s<=4,'exp range')
 n=ref['taylor_order'];term=Q(1);z=term
 for j in range(1,n+1):term=term*s/j;z+=term
 tail=term*s/(n+1)/(1-s/Q(n+2))
 return Q(1)/(z+tail),Q(1)/z
def sqrt_bound(v):
 check(v>=0,'sqrt domain')
 b=1<<ref['sqrt_bits'];n=isqrt((v*b*b).__floor__());l=Q(n,b)
 return l,l if l*l==v else Q(n+1,b)
def interval_for(t,lam,m,delta):
 c,v,e,a=map(Q,[A['c'],A['v'],A['e'],A['alpha']]);lo,hi=exp_neg(lam*t)
 x=(c+v*t+(e+delta)*lo,c+v*t+(e+delta)*hi);check(x[0]>0,'analytic x sign')
 z=(x[0]**2+a*t,x[1]**2+a*t)
 if m==2:z=(sqrt_bound(z[0])[0],sqrt_bound(z[1])[1])
 return x,z
def check_analytic(r):
 m=r['degree'];lam=Q(r['lambda']);solver=r['solver'];k=(m,int(lam),solver);a=r['audit']
 native_work(a)
 out={'degree':m,'lambda':int(lam),'solver':solver,'qualified':False,'error':r.get('error'),'query_checks':0,'uncertain':0}
 if m==2 and solver=='Affine5':
  check('error'in r and not a['driver_invoked'],'non-affine admitted by Affine5');C['nonaffine_refusals']+=1;return out
 if 'error'in r:return out
 q=r['run'];out['reason']=q['reason'];out['last']=q['last'];out['bound']=q['bound']
 for s in q['slabs']:
  if s['power'] and s['certificate'] is not None:
   native(s);C['analytic_native_coefficients']+=5*len(s['native']['y0'])
 for z in q['accepted']+q['queries']:
  t=Q(z['t']);x,y=map(Q,z['state'])
  check(abs(y**m-x*x-Q(A['alpha'])*t)<=Q(z['projected_constraint_upper']),('exact g residual',k,z['t']))
  C['analytic_residual_checks']+=1
  if z['physical_error_upper'] is None:continue
  bound=Q(z['physical_error_upper'])
  for delta in [-Q(A['radius']),Q(A['radius'])]:
   for val,box in zip([x,y],interval_for(t,lam,m,delta)):
    a,b=box;low=max(Q(0),a-val,val-b);high=max(abs(val-a),abs(val-b))
    check(low<=bound,('analytic reference counterexample',k,z['t'],float(low),float(bound)))
    if high>bound:out['uncertain']+=1;C['analytic_reference_inconclusive']+=1
    else:C['analytic_reference_components']+=1
  out['query_checks']+=1
 out['qualified']=q['global'] and q['pointwise']and q['branch'] and q['last']==A['end'] and out['uncertain']==0
 if out['qualified']:
  check(len(q['queries'])==A['grid_divisions']+1 and all(z['physical_error_upper'] is not None for z in q['accepted']+q['queries']),'qualified analytic missing grid bounds')
 C['analytic_query_checks']+=out['query_checks']
 return out
# The script alone reads new output rows before first verdict.
raw=P/'challenge.log'
def load(tag):return [json.loads(l.split(tag+' ',1)[1])for l in raw.read_text().splitlines()if tag+' {'in l]
hy=load('CHALLENGE');an=load('ANALYTIC')
expect=set(product(specs,*[S['axes'][k]for k in fields[1:]]))
if len(hy)!=192 or {key(r)for r in hy}!=expect:fail.append({'kind':'coverage','message':'hybrid row identity/count'})
expect_an=set(product(A['degrees'],A['lambdas'],S['axes']['solver']))
if len(an)!=32 or {(r['degree'],r['lambda'],r['solver'])for r in an}!=expect_an:fail.append({'kind':'coverage','message':'analytic row identity/count'})
for r in hy:
 try:rows.append(check_hybrid(r))
 except Exception as e:report_failure(key(r),e)
analytic=[]
for r in an:
 try:analytic.append(check_analytic(r))
 except Exception as e:report_failure([r['degree'],r['lambda'],r['solver']],e)
pairs=collections.defaultdict(dict)
for r in hy:
 if 'atlas'in r:pairs[key(r)[:4]][r['method']]=r['atlas']
for k,d in pairs.items():
 if len(d)!=2:continue
 a,b=d['AtlasFresh'],d['AtlasReuse']
 def norm(es):return [{**e,'event':{k:v for k,v in e['event'].items()if k!='phase_index'}}for e in es]
 try:
  check(a['stop']==b['stop'] and a['final_mode']==b['final_mode'] and norm(a['events'])==norm(b['events']) and a['queries']==b['queries'],'fresh/reuse physical identity')
  check(b['work']['phase']['native_attempts']<=a['work']['phase']['native_attempts'],'reuse attempt increase')
  C['fresh_reuse_pairs']+=1
 except Exception as e:report_failure(k,e)
cover={'tri_atlas_horizon':sum(r['qualified']for r in rows if r['case']=='tri_cycle'and r['method'].startswith('Atlas')),'clock_joint_horizon':sum(r['qualified']for r in rows if r['case']=='clock_mix'and r['method'].startswith('Joint')),'analytic_m1':sum(r['qualified']for r in analytic if r['degree']==1),'analytic_m2':sum(r['qualified']for r in analytic if r['degree']==2),'tie_native_coverage':sum(r.get('attempts',0)>0 for r in rows if r['case']=='exact_tie')}
status='FALSIFIER_OR_VERDICT_ERROR'if fail else 'INCONCLUSIVE_COVERAGE'if not all(cover.values())or inconclusive else 'SCOPED_ONE_SHOT_CHALLENGE_PASS'
summary={'status':status,'hybrid_rows':len(hy),'analytic_rows':len(an),'coverage':cover,'failures':fail,'inconclusive':inconclusive,'counts':dict(C),'exact_counts':dict(exact_counts),'reference':{'scipy':scipy.__version__,'solves':ref_stats['solves'],'steps':ref_stats['steps'],'max_state_method_spread':max_spread,'max_clock_method_spread':max_clock_spread,'max_joint_error_bound_ratio':max_ratio,'empirical_not_rigorous':True},'hybrid_outcomes':rows,'analytic_outcomes':analytic,'raw_sha256':hashlib.sha256(raw.read_bytes()).hexdigest(),'scope':'one-shot preregistered fresh challenge; finite exact analytic checks and exact native residual checks; Robertson references empirical, no statistical safety holdout or general closure'}
(P/'VERDICT.json').write_text(json.dumps(summary,indent=2,allow_nan=False)+'\n')
print('verdict',status,'coverage',cover,'hybrid',len(hy),'analytic',len(an),'failures',len(fail),'inconclusive',len(inconclusive))
print('counts',dict(C));print('exact counts',dict(exact_counts));print('reference',summary['reference'])
print('hybrid stops',dict(collections.Counter((r['case'],r['stop'])for r in rows)))
print('analytic qualified',dict(collections.Counter((r['degree'],r['solver'],r['qualified'])for r in analytic)))
for f in fail[:8]:print('failure',f['case']if 'case'in f else '',f['kind'],f['message'][:300])
