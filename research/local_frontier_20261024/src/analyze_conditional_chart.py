#!/usr/bin/env python3
"""Summarize conditional chart samples without suppressing refusals or HOLD rows."""
import argparse,collections,hashlib,json,statistics
from pathlib import Path

def main():
 p=argparse.ArgumentParser();p.add_argument('input',type=Path);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
 rows=[json.loads(line) for line in a.input.read_text().splitlines()]
 rows=[r for r in rows if r.get('kind')=='result']
 assert rows and all(r['actual_error_lower'] <= r['actual_error_upper'] for r in rows)
 for r in rows:
  if r['scientific_status']=='PASS_SCOPED':
   assert r['final_accepted'] and r['actual_error_upper'] <= r['error_limit'] and r['certified_error_upper']<=r['error_limit']
  assert r['chart_work']['chart_actions_attempted']+r['chart_work']['original_actions_attempted']<= (r['theoretical_operator_action_cap'] or 0)
 groups=collections.defaultdict(list)
 for r in rows:groups[(r['case'],r['action'],r['error_limit'],r['method'])].append(r)
 summaries=[]
 for key,rr in sorted(groups.items()):
  case,action,tol,method=key
  direct=groups[(case,action,tol,'direct')]
  direct_valid=all(r['scientific_status']=='PASS_SCOPED' for r in direct)
  valid=all(r['scientific_status']=='PASS_SCOPED' for r in rr)
  median=statistics.median(r['elapsed_seconds'] for r in rr)
  summaries.append(dict(case=case,action=action,tolerance=tol,method=method,samples=len(rr),all_scoped_pass=valid,
   median_seconds=median,median_ratio_to_direct=median/statistics.median(r['elapsed_seconds'] for r in direct) if valid and direct_valid else None,
   chart_accepts=sum(r['chart_accepted'] for r in rr),fallback_solves=sum(r['fallback_solves'] for r in rr),
   maximum_corrections=max(r['corrections'] for r in rr),actual_error_upper=max(r['actual_error_upper'] for r in rr),
   last_chart_error_upper=max([r['last_chart_error_upper'] for r in rr if r['last_chart_error_upper'] is not None],default=None),
   refusal_reasons=dict(collections.Counter(r['refusal_reason'] for r in rr))))
 out=dict(schema_version=1,study='conditional_chart_accuracy_cost',input=str(a.input),input_sha256=hashlib.sha256(a.input.read_bytes()).hexdigest(),
  rows=len(rows),status_counts=dict(collections.Counter(r['scientific_status'] for r in rows)),
  refusal_counts=dict(collections.Counter(r['refusal_reason'] for r in rows)),groups=summaries,
  conclusions={'selected_for_these_known_blocks':'direct triangular solve with outward original residual check',
   'conditional_candidate':'bounded polynomial correction improves the polynomial candidate only, not the strongest final direct comparator',
   'cost_cap':'empirical routing estimate; callback count bound is hard but elapsed-time bound requires verified event-cost bounds',
   'scope':'same requested infinity-error tolerance, not identical attained errors; not whole ODE or production promotion',
   'holds':'retained and excluded from speed ranking; an upper error bound above tolerance does not prove an actual error failure'})
 out['accuracy_contract']={'norm':'absolute infinity','residual_threshold':'round_down(tau/K)',
  'acceptance':'K*(norm(computed_original_residual)+mu)<=tau',
  'floor_meaning':'certificate uncertainty; not a lower bound on actual solution error',
  'finite_precision_recurrence':'r_next <= eta*r + A0*(P0*mu+nu)',
  'original_reference':'outward interval block-triangular solve for exact binary B and RHS',
  'lower_upper_separation':True}
 out['cost_contract']={'hard_bound':'degree*(max_corrections+1) callback attempts, plus separately counted fallback',
  'conditional_wall_bound':'passes*U+F_upper <= rho*F_lower requires verified cost bounds',
  'implemented_cost_proxy':'2 times measured first polynomial pass, excluding common final certificate',
  'implemented_wall_bound_verified':False,'setup_and_calibration_amortized_in_route_timing':False}
 out['next_steps']=[{'id':'CC1','action':'Select structured direct when the exact block provider exists'},
  {'id':'CC2','action':'Retain bounded refinement only where a useful relation exists without a cheaper direct solve'},
  {'id':'CC3','action':'Validate weighted/time-integration error allocation separately before ODE integration'},
  {'id':'CC4','action':'Measure provider-regime cost stability and setup/calibration amortization on local independent runs'},
  {'id':'CC5','action':'Tighten callback error allowance or residual precision when certificate uncertainty blocks accuracy'}]
 a.output.mkdir(parents=True,exist_ok=True);(a.output/'CONDITIONAL_CHART.json').write_text(json.dumps(out,indent=2)+'\n')
 try:
  import matplotlib
  matplotlib.use('Agg')
  import matplotlib.pyplot as plt
  fig,axes=plt.subplots(1,2,figsize=(10,3.9),layout='constrained')
  for case,marker in [('leakage','o'),('nonnormal','s')]:
   ss=[r for r in summaries if r['case']==case and r['action']=='structured' and r['method']=='refinement' and r['chart_accepts']]
   ss=sorted(ss,key=lambda r:r['tolerance'],reverse=True)
   axes[0].loglog([r['tolerance'] for r in ss],[r['last_chart_error_upper'] for r in ss],marker+'-',label=case)
   axes[1].semilogx([r['tolerance'] for r in ss],[r['median_ratio_to_direct'] for r in ss],marker+'-',label=case)
  axes[0].loglog([1e-13,1e-5],[1e-13,1e-5],'k--',alpha=.5,label='requested tolerance')
  axes[0].set(xlabel='Requested absolute infinity-error tolerance',ylabel='Polynomial candidate error upper bound',title='Accepted structured refinement candidates')
  axes[1].axhline(1,color='black',linestyle='--',alpha=.5);axes[1].axhline(1.5,color='gray',linestyle=':',alpha=.6)
  axes[1].set(xlabel='Requested absolute infinity-error tolerance',ylabel='Median complete-route time / direct time',title='Same tolerance; direct attains smaller error')
  for ax in axes:ax.grid(True,alpha=.2);ax.legend(fontsize=8)
  fig.savefig(a.output/'conditional_chart_tradeoff.png',dpi=160);fig.savefig(a.output/'conditional_chart_tradeoff.svg')
 except ImportError:pass
 print(json.dumps({'rows':len(rows),'status_counts':out['status_counts'],'groups':len(summaries)}))
if __name__=='__main__':main()
