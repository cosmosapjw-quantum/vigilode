#!/usr/bin/env python3
"""Reviewer exact recomputation of already-frozen adversarial fixtures; no candidate imports."""
import json,struct,math
from fractions import Fraction as F
from pathlib import Path
ROOT=Path(__file__).resolve().parent

def exact_repr(q):
    return str(q) if q.numerator.bit_length()+q.denominator.bit_length()<10000 else {"representation":"exact Fraction compared internally; large integer strings omitted", "numerator_bits":q.numerator.bit_length(),"denominator_bits":q.denominator.bit_length()}

def load(lane):return [json.loads(l) for l in (ROOT/f'{lane}_independent_native.jsonl').read_text().splitlines()]
def bit(s):return F(struct.unpack('>d',bytes.fromhex(s))[0])
def bound(x):return F(x['mantissa'])*F(2)**x['exponent']

checks=[]
# Each frozen transform operator satisfies A^2=0 (including A=0); hence a finite exact formula.
false_ids=[]
for row in load('polynomial'):
    if row['kind']=='transform':
        n=len(row['b_bits'][0]);A=[[bit(row['a_bits'][i*n+j]) for j in range(n)] for i in range(n)]
        assert all(sum(A[i][k]*A[k][j] for k in range(n))==0 for i in range(n) for j in range(n))
        h=bit(row['h_bits'][0]);error=[F(0)]*n
        for k,bs in enumerate(row['b_bits']):
            delta=[h**k*bit(b)-bit(w) for b,w in zip(bs,row['stored_bits'][k])]
            for i in range(n):error[i]+=delta[i]/math.factorial(k)+h*sum(A[i][j]*delta[j] for j in range(n))/math.factorial(k+1)
        actual_sq=sum(x*x for x in error);claimed=bound(row['result']['upper']);ok=claimed*claimed>=actual_sq
        if not ok:false_ids.append(row['id'])
        checks.append({'type':'nilpotent_transform_exact','id':row['id'],'original_claimed_bound_encloses':ok,'exact_output_error_squared':exact_repr(actual_sq),'claimed_bound':str(claimed)})
    elif row['kind']=='norm_candidate':
        exact_sq=sum(bit(s)**2 for s in row['input_bits']);up=bound(row['bound']);assert up*up>=exact_sq
        checks.append({'type':'scale_safe_norm_candidate','pass':True})
    elif row['kind']=='factorial_candidate':
        assert bound(row['coefficient'])>=F(1,math.factorial(row['order']))
        checks.append({'type':'inverse_factorial_candidate','order':row['order'],'pass':True})
assert set(false_ids)=={'subunit_nilpotent_generated','signed_stored_order0','order_171','order_172'}
for row in load('homotopy'):
    if row['case']=='witness-integrity':
        exact=F(1,16)/(1+F(1,16)*F(row['gamma']));got=F(row['certificate']['stage_bound'][0][0]);ok=got>=exact
        assert ok==(row['mode']=='constructor')
        checks.append({'type':'witness','mode':row['mode'],'bound_encloses_first_stage':ok,'exact_first_stage_magnitude':str(exact)})
    else:
        s=row['s'];exact=F(s);serial=F(row['serial']['output_bound'][0]);doubling=F(row['doubling']['certificate']['output_bound'][0]);assert serial>=exact;assert(doubling>=exact)==(s<=8)
        checks.append({'type':'stage_depth','s':s,'exact_last_stage':str(exact),'serial_encloses':True,'doubling_encloses':doubling>=exact})
for row in load('time'):
    if row['case']=='doubling_geometry_kernel':
        h1,h2=F(row['h1']),F(row['h2']);H=h1+h2;eta=(h1*h1+h2*h2)/(H*H);fine=2*(h1*h1+h2*H)/(H*H);err=fine-1;diff=F(2)-fine;ratio=eta/(1-eta)
        assert(fine,err,diff,ratio)==(F(14,9),F(5,9),F(4,9),F(5,4));assert abs(float(fine)-row['fine'])<1e-15;assert abs(float(err)-row['candidate_abs_error'])<1e-15
        checks.append({'type':'unequal_split_radau1','fine':str(fine),'true_error':str(err),'equal_half_estimator':str(diff),'geometry_correction':str(ratio),'pass':True})
    elif row['case']=='bdf_unit_boundary' and row['method']=='fixed_bdf':
        ts=list(map(F,row['times']));v=F(row['velocity']);ys=[F(0)]
        for i in range(1,len(ts)):
            h=ts[i]-ts[i-1]
            if i==1:ys.append(ys[-1]+h*v);continue
            prev=ts[i-1]-ts[i-2];equal=abs(prev-h)<=F(32)*F(2)**(-52)*max(abs(prev),abs(h),F(1));r=F(1) if equal else h/prev
            ys.append((h*v+(1+r)*ys[-1]-r*r/(1+r)*ys[-2])/((1+2*r)/(1+r)))
        assert abs(float(ys[-1])-row['values'][-1][0])<5e-15
        checks.append({'type':'bdf_mixed_recurrence','power':row['power'],'exact_recurrence_endpoint':str(ys[-1]),'agrees_native_within':5e-15})
out={'schema_version':'1.0','status':'PASS','method':'independent exact Fraction arithmetic on frozen binary64 inputs; no lane oracle imports','checks':checks,'false_transform_bounds':false_ids,'scope':'Finite fixtures only. Proofs have stated domain/identity assumptions; no production fixes or wall-time claims.'}
(ROOT/'independent_exact_spotchecks.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps({'status':'PASS','checks':len(checks),'false_transform_bounds':false_ids}))
