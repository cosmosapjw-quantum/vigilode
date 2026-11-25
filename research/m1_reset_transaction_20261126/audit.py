"""Independent high precision exact-source scalar hybrid reference."""
import json,sys,math
from pathlib import Path
import mpmath as mp
mp.mp.dps=85
P=Path(__file__).resolve().parent
def m(x):return mp.mpf(float(x))
def read(p,tag):return [json.loads(l.split(tag,1)[1]) for l in p.read_text().splitlines() if tag+'{' in l]
def inside(v,b):return m(b[0])<=v<=m(b[1])
def flow(mode,y,h):
 a,b=m(mode['rate']),m(mode['forcing']);return mp.exp(-a*h)*y-b*mp.expm1(-a*h)/a
def root(mode,y,start):
 g=mode['guard']
 if g is None:return None
 q=m(g['threshold']);eq=m(mode['forcing'])/m(mode['rate'])
 if not min(y,eq)<q<max(y,eq):return None
 return start+mp.log((y-eq)/(q-eq))/m(mode['rate'])
def oracle(s):
 t=mp.mpf(0);y=m(s['y']);end=m(s['end']);i=0;events=[];modes=s['system']['modes']
 for _ in range(65):
  mode=modes[i];r=root(mode,y,t)
  if r is None or r>end:return events,i,flow(mode,y,end-t)
  g=mode['guard'];q=m(g['threshold']);reset=g['reset'];post=m(reset['state_scale'])*q+m(reset['offset'])+m(reset['time_scale'])*r
  events.append({'time':r,'pre':q,'post':post,'from':i,'to':g['next'],'kind':'Up' if y<q else 'Down'})
  t,y,i=r,post,g['next']
 raise ValueError('oracle event cap')
def counters_sum(rows,key):
 d={}
 for r in rows:
  for k,v in r[key].items():d[k]=d.get(k,0)+v
 return d
def audit_run(src,r,events,final_mode,final,ttol,stol):
 assert r['counters']==counters_sum(r['epochs'],'counters')
 assert r['work']['accepted_inspections']==sum(e['inspected_steps'] for e in r['epochs'])
 for k,e in enumerate(r['epochs']):
  assert e['epoch']==k and e['inspected_steps']==e['counters']['accepted_steps']
  assert e['start']<=e['committed_end']<=e['numerical_end']
  if k:
   ev=r['events'][k-1]
   assert e['start']==ev['sync_time'] and e['start_center']==ev['reset_center'] and e['mode']==ev['to']
   assert e['start_enclosure']==ev['post_enclosure']
  if k<len(r['events']):
   ev=r['events'][k]
   assert e['committed'] and ev['epoch']==k and e['committed_end']==ev['sync_time']==ev['bracket'][1]
   if e['numerical_end']<src['end']:assert e['stop_reason']=='ClientStop'
  if not e['committed']:assert e['committed_end']==e['start'] and k==len(r['epochs'])-1
 if r['route']=='Represented':
  assert not r['physical_qualified'] and r['enclosure'] is None
  complete=r['status']=='Complete'
  return {'physical':False,'complete':complete,'qualified':False,'roots_checked':0,'endpoint_checks':0,
   'represented_root_error':float(max(abs(m(a['sync_time'])-b['time']) for a,b in zip(r['events'],events))) if complete and len(r['events'])==len(events) and events else None,
   'represented_final_error':float(abs(m(r['center'])-final)) if complete else None}
 modes=src['system']['modes'];y=m(src['y']);sync=mp.mpf(0);checks=0;roots=0;ratio=0.;maxw=0.;maxrad=0.;gain_ratios=[]
 for k,e in enumerate(r['epochs']):
  assert e['mode']==(0 if k==0 else events[k-1]['to'])
  assert m(e['start'])==sync and inside(y,e['start_enclosure'])
  mode=modes[e['mode']];guard=mode['guard']
  for j,p in enumerate(e['endpoint_proofs']):
   assert p['times'][0]>=e['start'] and p['times'][1]<=e['numerical_end']
   if j:assert p['times'][0]==e['endpoint_proofs'][j-1]['times'][1]
   if p['incoming_radius'] is not None:
    assert p['outgoing_radius'] is not None
    if j:assert p['incoming_radius']==e['endpoint_proofs'][j-1]['outgoing_radius']
   v=flow(mode,y,m(p['times'][1])-sync)
   assert inside(v,p['enclosure']),(src['case'],r['route'],k,j,mp.nstr(v,25),p['enclosure'])
   if guard and p['sign']:assert p['sign']* (v-m(guard['threshold']))>0
   checks+=1
  if k<len(r['events']):
   ev=r['events'][k];z=events[k]
   assert ev['from']==z['from'] and ev['to']==z['to'] and ev['kind']==z['kind']
   assert inside(z['time'],ev['bracket']),(src['case'],r['route'],k,mp.nstr(z['time'],25),ev['bracket'])
   assert m(ev['width'])>=m(ev['bracket'][1])-m(ev['bracket'][0])
   assert ev['width']<=ttol
   assert sync<z['time']<=m(ev['sync_time'])
   sync=m(ev['sync_time']);next_mode=modes[z['to']]
   if k+1<len(events):assert sync<events[k+1]['time']
   y=flow(next_mode,z['post'],sync-z['time'])
   assert inside(y,ev['post_enclosure'])
   assert inside(z['post'],ev['gap_tube']) and inside(y,ev['gap_tube'])
   if next_mode['guard']:
    q=m(next_mode['guard']['threshold']);assert (m(ev['gap_tube'][0])-q)*(m(ev['gap_tube'][1])-q)>0
   rad=m(ev['post_radius']);err=abs(m(ev['reset_center'])-y)
   assert err<=rad and ev['post_radius']<=stol
   if rad:ratio=max(ratio,float(err/rad))
   maxw=max(maxw,ev['width']);maxrad=max(maxrad,ev['post_radius']);roots+=1
   if ev['transported_width'] is not None:
    assert ev['transported_width']<=ev['rectangle_width']
    if ev['transported_width']>0:gain_ratios.append(ev['rectangle_width']/ev['transported_width'])
  elif e['committed']:
   assert e['committed_end']==src['end'] and len(r['events'])==len(events)
 complete=r['status']=='Complete'
 if complete:
  assert r['physical_qualified'] and r['time']==src['end'] and r['mode']==final_mode and len(r['events'])==len(events)
  assert inside(final,r['enclosure']),(src['case'],r['route'],mp.nstr(final,25),r['enclosure'])
  rad=m(r['radius']);err=abs(m(r['center'])-final);assert err<=rad and r['radius']<=stol
  if rad:ratio=max(ratio,float(err/rad))
  maxrad=max(maxrad,r['radius'])
 else:
  assert not r['physical_qualified'] and r['time']==(r['events'][-1]['sync_time'] if r['events'] else 0)
  assert inside(y,r['enclosure'])
 return {'physical':True,'complete':complete,'qualified':complete,'roots_checked':roots,'endpoint_checks':checks,'max_radius_ratio':ratio,'max_width':maxw,'max_radius':maxrad,'max_sync_tightening':max(gain_ratios,default=1.)}
def run(name):
 p=P/(name+'.log');srcs=read(p,'SOURCE ');rows=read(p,'CELL ')
 assert len(srcs)==(7 if name=='training' else 4)
 expected=112 if name=='training' else sum(v['selected'] is not None for g in json.loads((P/'SELECTION.json').read_text())['groups'].values() for v in g.values())
 assert len(rows)==expected and len({(r['case'],r['candidate'],r['time_tol']) for r in rows})==len(rows),(len(rows),expected)
 sources={s['case']:s for s in srcs};oracles={n:oracle(s) for n,s in sources.items()}
 results=[];totalchecks=totalroots=0
 for r in rows:
  s=sources[r['case']];events,mode,final=oracles[r['case']]
  summary={k:r[k] for k in ['case','group','candidate','time_tol','state_tol','median_ns']}
  summary.update(expected_events=len(events),qualified=False)
  z=r['result']
  if z is None:summary.update(status='Refused',reason=r.get('reason'));results.append(summary);continue
  if r['candidate']=='Adaptive':
   assert z['charged_native']==counters_sum(z['attempted_runs'],'counters')
   assert z['charged_work']==counters_sum(z['attempted_runs'],'work')
   for q in z['attempted_runs']:
    a=audit_run(s,q,events,mode,final,r['time_tol'],r['state_tol'])
    totalchecks+=a['endpoint_checks'];totalroots+=a['roots_checked']
  q=z['run']
  if q is None:summary.update(status='Unqualified',reason='no qualified route',charged_rhs=z['charged_native']['rhs_calls']);results.append(summary);continue
  a=audit_run(s,q,events,mode,final,r['time_tol'],r['state_tol'])
  if r['candidate']!='Adaptive':totalchecks+=a['endpoint_checks'];totalroots+=a['roots_checked']
  summary.update(a,status=q['status'],reason=q['reason'],selected_route=q['route'],charged_rhs=z['charged_native']['rhs_calls'],work=z['charged_work'],events=q['events'])
  results.append(summary)
 report={'cases':len(rows),'physical_qualified':sum(r['qualified'] for r in results),'represented_controls':sum(r['candidate']=='Represented' for r in results),'endpoint_checks_including_adaptive_attempts':totalchecks,'root_checks_including_adaptive_attempts':totalroots,'max_radius_ratio':max((r.get('max_radius_ratio',0.) for r in results),default=0.),'precision_dps':mp.mp.dps,'rows':results}
 (P/(name.upper()+'_AUDIT.json')).write_text(json.dumps(report,indent=2,sort_keys=True,allow_nan=False)+'\n')
 print(name,{k:v for k,v in report.items() if k!='rows'})
 for r in results:
  if not r['qualified'] and r['candidate']!='Represented':print('NOT QUALIFIED',r['case'],r['time_tol'],r['candidate'],r['status'],r.get('reason'))
if __name__=='__main__':run(sys.argv[1])
