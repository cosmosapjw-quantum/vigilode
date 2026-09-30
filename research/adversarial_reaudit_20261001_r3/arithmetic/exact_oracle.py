#!/usr/bin/env python3
"""Independent represented-input exact arithmetic. Never calls native solver."""
from pathlib import Path
import json, math, struct
from fractions import Fraction
from decimal import Decimal, localcontext

ROOT=Path(__file__).resolve().parent
rows=[json.loads(x) for x in (ROOT/'native.jsonl').read_text().splitlines()]
def fbits(i): return struct.unpack('>d',int(i).to_bytes(8,'big'))[0]
def bits(f): return struct.unpack('>Q',struct.pack('>d',f))[0]
def rounded(q):
    try: return float(q)
    except OverflowError: return math.inf if q>0 else -math.inf
def de(q):
    with localcontext() as ctx:
        ctx.prec=110
        return str(Decimal(q.numerator)/Decimal(q.denominator))

nilpotent=[]; powers=[]; decimals=[]
for r in rows:
    if 'inputs' in r:
        p={k:Fraction(v) for k,v in r['inputs'].items()}
        q=[p['b0']+p['h']**3*p['off']*p['b2']/6,p['h']**2*p['b2']/2]
        expected=[rounded(x) for x in q]
        original={}
        for name in ('fused','prefix','dense'):
            output=r[name] if name=='dense' else r[name]['value'] if r[name] else None
            if output is not None:
                original[name]={'output':output,'relative_component0_error':abs(float((Fraction(output[0])-q[0])/q[0])),'error_above_declared_relative_budget':abs(Fraction(output[0])-q[0]) > Fraction(1e-12)*abs(q[0])}
        candidate_error=max(abs(float((Fraction(x)-qv)/qv)) if qv and rounded(qv)!=0 else 0.0 for x,qv in zip(r['nilpotent_candidate'],q))
        nilpotent.append({'id':r['id'],'formula':'[b0+h^3*off*b2/6, h^2*b2/2]; A^2=0, b0 aligned with kernel','exact_decimal':[de(x) for x in q],'rounded':expected,'original':original,'nilpotent_candidate_relative_error':candidate_error,'guard':r['guarded'],'candidate_scope':'this 2D nilpotent structure only; fail-closed weighting guard is general but conservative'})
    elif r['id']=='power_fraction':
        v,s=fbits(r['value_bits']),fbits(r['scale_bits']);q=Fraction(v)*Fraction(s)**r['k'];ref=rounded(q);got=fbits(r['result_bits'])
        finite=math.isfinite(ref) and math.isfinite(got)
        ulp=abs(bits(abs(ref))-bits(abs(got))) if finite else None
        rel=abs(float((Fraction(got)-q)/q)) if finite and q else 0.0 if finite else None
        match_special= (math.isinf(ref) and ref==got) or (ref==0 and got==0)
        powers.append({'i':r['i'],'finite_reference':math.isfinite(ref),'rounded_reference_bits':bits(ref),'result_bits':r['result_bits'],'ulp_distance':ulp,'relative_error':rel,'within_8_ulp':(ulp is not None and ulp<=8) or match_special})
    elif r['id']=='original_budget':
        q=Fraction(r['epsilon_ref'])*(Fraction(r['h'])/Fraction(r['h_ref']))**r['exponent'];got=r['decision']['budget']
        decimals.append({'id':r['id'],'exact_numerator':str(q.numerator),'exact_denominator':str(q.denominator),'exact_decimal':de(q),'actual':got,'relative_error':abs(float((Fraction(got)-q)/q)),'wrms5_rejected':not r['decision']['accepted']})
    elif r['id']=='dense_amplified_mixed_range':
        with localcontext() as ctx:
            ctx.prec=110
            ref=[Decimal(x).exp()*Decimal(w) for x,w in zip(r['diagonal'],r['weights'][0])]
        decimals.append({'id':r['id'],'exact_decimal_precision':110,'expected':[str(x) for x in ref],'actual':r['value'],'mixed_range':r['mixed_range'],'output_below_input_half_precision':r['output_below_input_half_precision'],'classification':'documented labelled limitation; not a separate new finding'})

out={'source_commit':'cc2cd041737e7ff543624d1b59893a3b4397369f','oracle_independence':'Fraction/Decimal standard library; analytic nilpotent polynomial; no native phi/matrix exponential implementation reused','nilpotent':nilpotent,'power_summary':{'count':len(powers),'within_8_ulp_count':sum(x['within_8_ulp'] for x in powers),'maximum_ulp':max(x['ulp_distance'] or 0 for x in powers),'special_note':'subnormal relative error is not O(eps); compare absolute ulps instead'},'power_rows':powers,'other_oracles':decimals}
(ROOT/'EXACT_ORACLES.json').write_text(json.dumps(out,indent=2,ensure_ascii=False)+'\n')
print(json.dumps({'nilpotent':nilpotent,'power_summary':out['power_summary'],'other_oracles':decimals},indent=2,ensure_ascii=False))
