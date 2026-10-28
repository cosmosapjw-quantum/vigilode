#!/usr/bin/env python3
"""Analyze the measured native declared-flow probe at common sampled-error caps."""
import argparse,json,statistics
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('input',type=Path);p.add_argument('output',type=Path);args=p.parse_args()
data=json.loads(args.input.read_text());rows=[r for r in data['rows'] if not r['warmup']]
aggregates=[]
for key in sorted({(r['case'],r['method'],r['tol']) for r in rows}):
 r=[r for r in rows if (r['case'],r['method'],r['tol'])==key]
 aggregates.append(dict(case=key[0],method=key[1],tol=key[2],success=all(v['success'] for v in r),median_seconds=statistics.median(v['seconds'] for v in r),min_seconds=min(v['seconds'] for v in r),max_seconds=max(v['seconds'] for v in r),error=max(v['maximum_sampled_absolute_error'] for v in r),rhs_calls=r[0]['rhs_calls'],accepted=r[0]['accepted'],dimension=r[0]['dimension'],workspace=r[0].get('quotient_work')))
comparisons=[]
for case in sorted({r['case'] for r in rows}):
 for cap in [1e-2,1e-4,1e-6,1e-8]:
  chosen={}
  for method in ['full-rhs-dopri5','quotient-lawson-dopri5']:
   candidates=[a for a in aggregates if a['case']==case and a['method']==method and a['success'] and a['error']<=cap]
   chosen[method]=min(candidates,key=lambda x:x['median_seconds']) if candidates else None
  a,b=chosen.values()
  comparisons.append(dict(case=case,sampled_error_cap=cap,baseline=a,candidate=b,baseline_over_candidate=(a['median_seconds']/b['median_seconds'] if a and b else None),status='comparable' if a and b else 'unmatched'))
result=dict(schema='vigilode.quotient-analysis.v1',scope='minimum median among measured tolerances under the same sampled absolute-error cap; not a global continuous-error bound',rows=len(data['rows']),timed_rows=len(rows),aggregates=aggregates,comparisons=comparisons)
args.output.write_text(json.dumps(result,indent=2,allow_nan=False)+'\n')
