#!/usr/bin/env python3
"""C9 non-author independent numerical oracle via arbitrary-precision Taylor ODE.
Cross-check C8 Radau/BDF and actual native CSV/stdout with a different solver.
This is empirical high precision, not interval-verified exact global IVP error.
"""
import mpmath as mp
import csv, json
from pathlib import Path
ROOT=Path(__file__).resolve().parent
C8=ROOT/'work/VIGILODE_N01_DF_C8_20261030'
reference=json.loads((C8/'results/reference.json').read_text())
lines=(C8/'results/actions/native.stdout.log').read_text().splitlines()
rows=[json.loads(l.split('C8_ROW ',1)[1]) for l in lines if 'C8_ROW ' in l]
assert len(rows)==38
out=[]
for case in reference['cases'][:2]:
    kstr=[format(x,'.17g') for x in case['rates']]
    estimates=[]
    for dps in (40,75):
        mp.mp.dps=dps
        k=list(map(mp.mpf,kstr))
        def f(t,z):
            a,b,c=z;k1,k2,k3=k
            return [-k1*a+k2*b*c,k1*a-k2*b*c-k3*b*b,k3*b*b]
        sol=mp.odefun(f,mp.mpf(0),[mp.mpf('.75'),mp.mpf('.125'),mp.mpf('.125')],tol=mp.mpf(10)**(-min(dps-12,48)),degree=20)
        estimates.append([mp.nstr(z,70) for z in sol(mp.mpf(format(case['end'],'.17g')))])
    mp.mp.dps=75
    exact=[mp.mpf(v) for v in estimates[1]]
    dps_delta=max(abs(mp.mpf(a)-b) for a,b in zip(estimates[0],exact))
    dmethod={name:[mp.mpf(repr(v)) for v in case['reference'][name]] for name in ['Radau','BDF']}
    method_gap={name:max(abs(a-b) for a,b in zip(vals,exact)) for name,vals in dmethod.items()}
    numeric=[]
    for row in rows:
        if row['case']!=case['id']:continue
        native=[mp.mpf(repr(z)) for z in row['state']]
        error=sum(abs(a-b) for a,b in zip(native,exact))
        n=int(row['n'])
        numeric.append({'method':row['method'],'n':n,'mp_achieved_l1':float(error), 'native_reported_l1':float(sum(abs(a-b) for a,b in zip(native,dmethod['Radau']))),
                        'map_attempted':row.get('map_attempted'),'rhs_calls':row.get('rhs'),'linear_solves':row.get('linear_solves'), 'local_bound_not_global':row.get('algebraic_endpoint_bound_l1_max'),
                        'IVP_error_over_max_single_step_algebraic_bound':float(error/mp.mpf(repr(row['algebraic_endpoint_bound_l1_max']))) if row.get('algebraic_endpoint_bound_l1_max') else None})
    # Compare source-matched achieved-error ranges: RODAS values << BE values at every tested grid.
    ro=[r['mp_achieved_l1'] for r in numeric if r['method']=='RODAS5P-direct-oracle']
    be=[r['mp_achieved_l1'] for r in numeric if r['method']!='RODAS5P-direct-oracle']
    overlap=not (max(ro)<min(be) or max(be)<min(ro))
    out.append({'case':case['id'],'mp_dps':[40,75],'mp_reference':estimates[1],
       'mp_internal_cross_max':mp.nstr(dps_delta,12),
       'source_reference_max_difference':{name:mp.nstr(v,14) for name,v in method_gap.items()},
       'reference_Radau_BDF_cross_not_bound':case['reference_cross_difference_linf'],
       'native_rows':numeric,'Rodas_BE_achieved_accuracy_overlap_in_tested_grids':overlap,
       'methodological_comment':'No achieved-accuracy overlap across arms in these grids: cost-per-accuracy cannot be compared at equal achieved errors. The within-BE Picard vs acceleration comparison is meaningful.'})
res={'schema':'vigilode.n01.c9.mpmath_cross_reference.v1',
     'method':'mpmath arbitrary-precision Taylor odefun, dps 40 vs 75, independent numerical implementation',
     'meaning':'numerically stable high-precision reference, NOT a rigorous verified IVP error enclosure',
     'cases':out}
(ROOT/'mp_results.json').write_text(json.dumps(res,indent=2)+'\n')
for q in out:
    print(q['case'],'max mpmath40-vs75',q['mp_internal_cross_max'],'Radau',q['source_reference_max_difference']['Radau'],'BDF',q['source_reference_max_difference']['BDF'],'achieved overlap?',q['Rodas_BE_achieved_accuracy_overlap_in_tested_grids'])
print('rows',sum(len(v['native_rows']) for v in out))
