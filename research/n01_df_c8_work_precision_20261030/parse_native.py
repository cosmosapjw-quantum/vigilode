#!/usr/bin/env python3
"""Parse actual source-matched Rust C8 CI output into bounded achieved-error/work evidence.
Do NOT compare map calls to full RHS/dense solves as a universal cost unit.
"""
import csv,json,sys
from pathlib import Path
script=Path(__file__).resolve()
root=script.parents[1] if script.parent.name=='analysis' else script.parents[0]
ref=json.loads((root/'reference.json' if (root/'reference.json').exists() else root/'results/reference.json').read_text())
cases={x['id']:x for x in ref['cases']}
log=Path(sys.argv[1]) if len(sys.argv)>1 else (root/'native.stdout.log')
rows=[];fails=[];faults=[]
for line in log.read_text(errors='replace').splitlines():
 for tag,dest in [('C8_ROW ',rows),('C8_FAIL ',fails),('C8_NEGATIVE ',faults)]:
  if tag in line:
   dest.append(json.loads(line.split(tag,1)[1]));break
assert len(rows)>=30 and len(fails)>=8 and len(faults)>=1,(len(rows),len(fails),len(faults))
records=[]
for row in rows:
 refc=cases[row['case']]
 rad=refc['reference']['Radau']; bdf=refc['reference']['BDF']
 errors=[abs(x-y) for x,y in zip(row['state'],rad)]
 rec={**row,'status':'COMPLETED','achieved_l1':sum(errors),'achieved_linf':max(errors),
 'reference_cross_difference_linf':refc['reference_cross_difference_linf'],
 'local_algebraic_bound_not_global':row.get('algebraic_endpoint_bound_l1_max')}
 records.append(rec)
for f in fails:records.append({**f,'status':'REFUSED','achieved_l1':None,'achieved_linf':None,'reference_cross_difference_linf':cases[f['case']]['reference_cross_difference_linf']})
fields=['case','method','n','status','achieved_l1','achieved_linf','mass_error','min_state',
        'rhs','jac','linear_solves','direct_factorizations','map_attempted','map_completed',
        'speculative_trials','steps_completed','original_interval_residual_l1_max',
        'algebraic_endpoint_bound_l1_max','certificate_status','error','reference_cross_difference_linf']
outdir=root/'results' if (root/'results').is_dir() else root
with (outdir/'work_precision.csv').open('w',newline='') as f:
 w=csv.DictWriter(f,fieldnames=fields,extrasaction='ignore');w.writeheader();w.writerows(records)
summary=[]
for case in cases:
 for method in ['RODAS5P-direct-oracle','BE-Picard-value-only','BE-Homotopy-value-only']:
  arms=sorted([r for r in records if r['case']==case and r['method']==method],key=lambda r:r['n'])
  ok=[r for r in arms if r['status']=='COMPLETED']
  caps={}
  for cap in (1e-3,1e-4,1e-5,1e-6,1e-8):
   good=[r for r in ok if r['achieved_l1']<=cap]
   caps[f'{cap:.0e}']=None if not good else {k:good[0].get(k) for k in ['n','achieved_l1','rhs','jac','linear_solves','map_attempted']}
  summary.append({'case':case,'method':method,'completed':len(ok),'refused':len(arms)-len(ok),
   'coarsest_error_l1':ok[0]['achieved_l1'] if ok else None,
   'finest_error_l1':ok[-1]['achieved_l1'] if ok else None,
   'max_mass_error':max((r['mass_error'] for r in ok),default=None),
   'local_stage_algebraic_certified_steps':sum(r.get('certificate_status')=='PROVED_CONDITIONAL_TUBE' for r in ok),
   'caps':caps})
report={'schema':'vigilode.n01.c8.native_work_precision.v1','source_kind':'native GitHub-hosted Rust1.94.1',
        'native_completed_rows':len(rows),'native_refused_rows':len(fails),
        'native_negative_controls':faults,'reference':{'methods':['Radau','BDF'],'rtol':ref['rtol'],'atol':ref['atol'],
         'cross_diff_not_certified':{k:v['reference_cross_difference_linf'] for k,v in cases.items()}},
        'comparison':'Same physical source; discrete fixed forced steps and achieved endpoints. No total-work normalization across dense matrix and scalar map; no universal speedup or physical adaptive acceptance claim.',
        'summary':summary,'refusals':fails}
(outdir/'work_precision_summary.json').write_text(json.dumps(report,indent=2)+'\n')
print('C8_PARSED',len(rows),len(fails),'records, summary entries',len(summary))
for s in summary:print('C8_COMPARISON',s['case'],s['method'],s['completed'],s['refused'],s['caps'].get('1e-05'))
