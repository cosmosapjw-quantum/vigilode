#!/usr/bin/env python3
"""Independent exact real constant-flow oracle for the represented f64 inputs.

No solver coefficients or solver interpolation are imported. Fraction.from_float
captures binary64 values exactly; f64 code results are compared with v*(t-t0).
This is an oracle for the constant-flow fixture only, not a solver certificate.
"""
import argparse
import json
from fractions import Fraction as F
from pathlib import Path

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('--raw',type=Path,default=Path(__file__).with_name('native_raw.jsonl'))
    ap.add_argument('--out',type=Path,default=Path(__file__).with_name('exact_oracle.json'))
    a=ap.parse_args()
    rows=[json.loads(line) for line in a.raw.read_text().splitlines() if line.strip()]
    analyzed=[]
    for r in rows:
        if 'velocity' not in r or 'times' not in r or 'values' not in r:
            continue
        t0=F.from_float(r['origin']); v=F.from_float(r['velocity'])
        err=[]
        for t,y in zip(r['times'],r['values']):
            expected=v*(F.from_float(t)-t0)
            actual=F.from_float(y[0])
            err.append({'time':t,'expected_exact':str(expected),'actual':y[0],
                        'absolute_error':float(abs(actual-expected))})
        entry={'case':r['case'],'method':r['method'],'origin':r['origin'],
               'success':r['success'],'samples':len(err),
               'max_absolute_error':max(e['absolute_error'] for e in err),
               'last':err[-1]}
        if 'step_sizes' in r:
            # Exact sum of represented input h need not match displacement.
            effective=sum((F.from_float(h) for h in r['step_sizes']),F(0))
            displacement=F.from_float(r['times'][-1])-t0
            entry.update({'sum_h_exact':str(effective),'represented_displacement_exact':str(displacement),
                          'sum_h_over_displacement':float(effective/displacement),
                          'max_step_clock_defect':max(float(abs(F.from_float(h)-(F.from_float(b)-F.from_float(t)))) for h,t,b in zip(r['step_sizes'],r['times'],r['times'][1:]))})
        analyzed.append(entry)
    closed=[r for r in analyzed if r['case'].startswith('r2_original')]
    candidates=[r for r in analyzed if r['case']=='represented_h_candidate']
    aliases=[r for r in analyzed if r['case'] in ('single_point_uniform','single_point_explicit','endpoint_alias_labels')]
    controls=[r for r in analyzed if r['case'] in ('valid_two_point_control','local_clock_control')]
    strict=[r for r in rows if r['case']=='strict_schedule_candidate']
    checks={
      'original_closures':all(r['success'] and r['max_absolute_error']<1e-9 for r in closed) and len(closed)==5,
      'candidate_constant_flow':all(r['success'] and r['max_absolute_error']<1e-12 for r in candidates) and len(candidates)==3,
      'controls':all(r['success'] and r['max_absolute_error']<1e-12 for r in controls) and len(controls)==3,
      'alias_counterexamples':all(r['success'] and r['max_absolute_error']>=0.99 for r in aliases) and len(aliases)==5,
      'strict_schedule_candidate':len(strict)==4 and all(r['admitted']==(r['input_case']=='valid_two_point_control') for r in strict),
      'nonadvancing_typed_failure':any(r['case']=='r2_nonadvancing_step' and 'does not advance' in r.get('error','') for r in rows),
    }
    obj={'schema':'vigilode.r3.time.exact_oracle.v1','oracle':'Fraction.from_float; dy/dt=v, y(t0)=0',
         'raw_rows':len(rows),'checks':checks,'results':analyzed,
         'claim_ceiling':'Exact reference for represented scalar constant velocity fixtures; candidate only local clock and endpoint admission checks, not production repair.'}
    a.out.write_text(json.dumps(obj,indent=2)+'\n')
    print(json.dumps({'raw_rows':len(rows),'checks':checks}))
    if not all(checks.values()): raise SystemExit(1)

if __name__=='__main__':main()
