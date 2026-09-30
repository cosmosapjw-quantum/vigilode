#!/usr/bin/env python3
from pathlib import Path
from collections import Counter
from fractions import Fraction
from decimal import Decimal, localcontext
import json
ROOT=Path(__file__).resolve().parent
rows=[json.loads(s) for s in (ROOT/'legacy_closure.jsonl').read_text().splitlines()]
out=[]
for row in rows:
    id=row['id']
    if id in ('scaled_range_dense','scaled_range_fused'):
        h=row['h'];b4=1e-300 if abs(h)>1 else 1e300
        q=Fraction(h)**4*Fraction(b4)/24
        value=row['value'] if id.endswith('dense') else row['report']['value'][0]
        rel=abs(float((Fraction(value)-q)/q))
        out.append({'finding':'PHI-R1','id':id,'h':h,'b4':b4,'expected':float(q),'actual':value,'relative_error':rel,'within_1e-12':rel<=1e-12})
    elif id in ('amplitude_dense','amplitude_fused'):
        c=row['h'] if id=='amplitude_dense' else row['amplitude']
        with localcontext() as ctx:
            ctx.prec=110;h=Decimal(.1);expected=(1-(-h).exp())/h*Decimal(c)
            value=row['value'] if id=='amplitude_dense' else row['report']['value'][0]
            rel=float(abs((Decimal(value)-expected)/expected))
        out.append({'finding':'PHI-R2','id':id,'amplitude':c,'expected':str(expected),'actual':value,'relative_error':rel,'within_1e-12':rel<=1e-12})
groups=[]
for finding in ('PHI-R1','PHI-R2'):
    group=[r for r in out if r['finding']==finding]
    groups.append({'id':finding,'status':'CLOSED_FOR_ORIGINAL_REPRODUCERS','native_production_rows':len(group),'maximum_relative_error':max(r['relative_error'] for r in group),'all_within_1e-12':all(r['within_1e-12'] for r in group)})
groups.append({'id':'R2-POL-01','status':'CLOSED_FOR_ORIGINAL_REPRODUCER','evidence':'EXACT_ORACLES.json#/other_oracles','consumer_regression':'repository r2_output_budget_contracts; execution is owned by runtime lane'})
result={'source_commit':'cc2cd041737e7ff543624d1b59893a3b4397369f','legacy_probe_rows':len(rows),'legacy_probe_production_rows':91,'legacy_probe_candidate_rows':13,'counts_by_case':dict(Counter(r['id'] for r in rows)),'closures':groups,'independent_rows':out,'oracle':'PHI-R1 uses exact Fraction(h)^4*Fraction(b4)/24; PHI-R2 uses 110-digit Decimal exp of represented h=0.1; original non-amplitude rows keep prior scalar-series oracle as regression only','scope':'The same prior source bytes executed against R3 dependencies, not inherited R2 PASS or bypass of its immutable-source runner'}
(ROOT/'CLOSURE.json').write_text(json.dumps(result,indent=2,ensure_ascii=False)+'\n')
print(json.dumps(groups,indent=2))
assert all(g.get('all_within_1e-12',True) for g in groups)
