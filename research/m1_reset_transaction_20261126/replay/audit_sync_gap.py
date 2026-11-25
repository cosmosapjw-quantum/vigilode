"""Check complete sync interval formula at a grid of candidate event times.
Independent of the actual physical root oracle and existing native trajectory."""
import json
from pathlib import Path
from audit import m,flow,inside,read
import mpmath as mp
P=Path(__file__).resolve().parent
n=0;events=0;max_ratio=0.
for name in ['training','holdout']:
 src={r['case']:r for r in read(P/(name+'.log'),'SOURCE ')}
 for r in read(P/(name+'.log'),'CELL '):
  if r['result'] is None:continue
  z=r['result']
  runs=z['attempted_runs'] if r['candidate']=='Adaptive' else [z['run']]
  for run in runs:
   if run is None or run['route']=='Represented':continue
   for e in run['events']:
    modes=src[r['case']]['system']['modes'];g=modes[e['from']]['guard'];rmap=g['reset'];next=modes[e['to']]
    a,b=map(m,e['bracket']);events+=1
    for i in range(9):
     tau=a+(b-a)*i/8;reset=m(rmap['state_scale'])*m(g['threshold'])+m(rmap['offset'])+m(rmap['time_scale'])*tau
     sync=flow(next,reset,b-tau)
     assert inside(sync,e['post_enclosure']),(name,run['route'],e)
     rad=m(e['post_radius'])
     if rad:max_ratio=max(max_ratio,float(abs(sync-m(e['reset_center']))/rad))
     for j in range(9):
      y=flow(next,reset,(b-tau)*j/8)
      assert inside(y,e['gap_tube'])
      if next['guard'] is not None:
       q=m(next['guard']['threshold'])
       assert (m(e['gap_tube'][0])-q)*(m(e['gap_tube'][1])-q)>0
      n+=1
(P/'SYNC_GAP_AUDIT.json').write_text(json.dumps({'physical_reset_records':events,'synchronization_gap_grid_checks':n,'max_sync_state_radius_ratio':max_ratio,'precision_dps':mp.mp.dps,'scope':'finite independent checks over the entire candidate-time bracket and gap; not replacing interval proof'},indent=2)+'\n')
print(events,'reset records',n,'gap checks',max_ratio)
