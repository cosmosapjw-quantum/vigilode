"""Aggregate the native and independent work-precision runs at common error gates."""
import csv
import json
from pathlib import Path
from statistics import median

here = Path(__file__).resolve().parent
rows=[]
for name in ("adaptive_native.csv","adaptive_scipy.csv"):
    with (here/name).open() as f:
        rows.extend(csv.DictReader(f))
summary=[]
for key in sorted({(int(r['dimension']),r['method'],float(r['rtol'])) for r in rows}):
    n, method, tol=key
    cells=[r for r in rows if int(r['dimension'])==n and r['method']==method and float(r['rtol'])==tol]
    assert len(cells)==5
    summary.append(dict(n=n,method=method,rtol=tol,
                        median_us=median(int(r['elapsed_ns']) for r in cells)/1e3,
                        worst_max_abs=max(float(r['max_abs_error']) for r in cells),
                        steps=[int(cells[0][v]) for v in ('accepted','rejected')] if 'accepted' in cells[0] else None,
                        nfev=int(cells[0]['nfev']) if 'nfev' in cells[0] else None))
gates=[]
for n in [64,512]:
    for gate in [1e-6,1e-8,1e-10]:
        result={}
        for meth in ['modal','protected_band0','BDF','Radau','closed_exp']:
            eligible=[r for r in summary if r['n']==n and r['method']==meth and r['worst_max_abs']<=gate]
            if eligible:
                best=min(eligible,key=lambda r:r['median_us'])
                result[meth]=[best['rtol'],best['median_us'],best['worst_max_abs']]
            else:
                result[meth]=None
        gates.append(dict(n=n,max_abs_bound=gate,fastest_tested_eligible=result))
print(json.dumps(dict(cells=summary,common_error_gates=gates,
    interpretation='same physical modal coordinates and exact reference; median time among tested tolerances meeting achieved absolute error bound in all repetitions; selection may use different rtol; no interpolation; Rust vs Python driver overhead included; no transform cost'),indent=2))
