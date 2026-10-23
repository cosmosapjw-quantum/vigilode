"""Independent computation path: direct Decimal block formula, no C rerun.
Same author; NOT independent personnel review or native RODAS certification.
"""
import hashlib,json,math
from decimal import Decimal, localcontext
from fractions import Fraction
from pathlib import Path
root=Path(__file__).resolve().parent
r=json.loads((root/'CROSSCHECK.json').read_text())
checks=[]
with localcontext() as ctx:
    ctx.prec=800
    for row in r['cases']:
        e=[Decimal.from_float(x) for x in row['error30']]
        s=[Decimal.from_float(x) for x in row['scale33']]
        # Different construction from the sparse-loop Fraction implementation.
        full=e[:23]+[-e[23],e[23],-e[24]-e[25],e[24],e[25],
                      -e[26]-e[27]-e[28],e[26],e[27],e[28],e[29]]
        assert len(full)==33
        v=sum((x/d)**2 for x,d in zip(full,s))/Decimal(33)
        q=Fraction(row['exact_squared']); other=Decimal(q.numerator)/Decimal(q.denominator)
        assert abs(v-other)<=abs(other)*Decimal('1e-790'),row['name']
        assert (v<=1)==row['exact_accepts']
        if row['C99_status']==0:
            actual=Decimal.from_float(row['C99_norm'])
            assert abs(actual-v.sqrt())<=v.sqrt()*Decimal('4e-15')
        checks.append({'name':row['name'],'decimal_accepts':v<=1,'C99_scope_confirmed':row['C99_status']==0})
assert [x['decimal_accepts'] for x in checks]==[False,True,True,True,True]
review={'status':'PASS_INDEPENDENT_COMPUTATION_SCOPE','reviewer':'separate Decimal direct-block computation; same author',
        'input_sha256':hashlib.sha256((root/'CROSSCHECK.json').read_bytes()).hexdigest(),
        'cases':checks,'native_claim':'NOT_VERIFIED','independent_personnel':'NOT_RUN',
        'findings':['receipt schema completeness alone cannot authenticate execution evidence',
                    'published harmonic-flow PASS is not native physical-error-norm PASS',
                    'overflow in unnormalized lifted components need not imply overflow of normalized norm',
                    'no allocation or performance claim was measured in these five probes']}
(root/'INDEPENDENT_COMPUTATION_REVIEW.json').write_text(json.dumps(review,indent=2)+'\n')
print(json.dumps(review))
