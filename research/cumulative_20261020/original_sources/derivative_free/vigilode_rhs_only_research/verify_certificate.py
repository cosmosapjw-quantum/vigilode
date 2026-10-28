"""Read stored rational certificates, without regenerating solver candidates.

This checker uses a separate polynomial convolution/Bernstein implementation.
No ODE solution or high-precision truth is used for acceptance. It verifies the
model-specific algebraic sufficient conditions explained in REPORT_KO.md, not
arbitrary client models or a proof-assistant formalization.
"""
from fractions import Fraction as Q
from pathlib import Path
from math import comb,factorial
import json,sys
ROOT=Path(__file__).resolve().parent

def bernstein(p,h):
    n=len(p)-1
    return [sum((p[j]*(h**j)*Q(comb(k,j),comb(n,j)) for j in range(k+1)),Q(0)) for k in range(n+1)]
def defect(coeff):
    n=len(coeff);d=[Q(0)]*(2*n-1)
    for k in range(n):
        d[k]+=2*coeff[k]
        if k:d[k-1]+=k*coeff[k]
        for j in range(n):d[k+j]+=coeff[k]*coeff[j]
    return d

def exp_bracket(h):
    z=2*h
    assert 0<=z<=1
    partial=sum((Q((-1)**k)*z**k/Q(factorial(k)) for k in range(12)),Q(0))
    return partial,partial+z**12/Q(factorial(12))
def flow_bound(initial,delta,h):
    low,high=exp_bracket(h)
    return high*initial+delta*(1-low)/2

def value(coeff,t):
    return sum((a*t**i for i,a in enumerate(coeff)),Q(0))

def verify():
    assert sys.float_info.radix==2 and sys.float_info.mant_dig==53
    data=json.loads((ROOT/'ADAPTIVE_SCALAR_RESULT.json').read_text())
    assert data['equation']=="y'=-2y-y^2" and data['initial']=='1/2'
    tol=Q(data['tolerance']);round_bound=Q(1,2**53)
    segments=[];last=None
    for record in data['segments']:
        c=[Q(x) for x in record['coefficients']]
        s={k:Q(v) for k,v in record.items() if k!='coefficients'}
        assert len(c)==6 and s['h']>0
        assert min(bernstein(c,s['h']))>=0 and max(bernstein(c,s['h']))<=1
        d=defect(c)
        assert all(v==0 for v in d[:5]),'coefficient recurrence mismatch'
        delta=max(abs(v) for v in bernstein(d,s['h']))
        assert delta==s['defect_upper']
        assert 0<=s['initial_radius'] and s['left']>=0
        end=flow_bound(s['initial_radius'],delta,s['h'])
        assert s['continuous_output_bound']==max(s['initial_radius'],end)+round_bound
        assert s['continuous_output_bound']<=tol
        rational_end=value(c,s['h'])
        actual_center=Q.from_float(float(rational_end))
        displacement=abs(actual_center-rational_end)
        assert displacement<=round_bound
        assert actual_center==s['endpoint_center']
        assert s['endpoint_radius']==end+displacement
        if last is None:
            assert s['left']==0 and c[0]==Q(1,2) and s['initial_radius']==0
        else:
            assert s['left']==last['left']+last['h']
            assert c[0]==last['endpoint_center']
            assert s['initial_radius']==last['endpoint_radius']
        s['coefficients']=c;segments.append(s);last=s
    assert last['left']+last['h']==Q(data['end_time'])==1
    dense_count=0
    for row in data['dense_outputs']:
        t=Q(row['time_rational'])
        s=next(s for s in segments if s['left']<=t<=s['left']+s['h'])
        dt=t-s['left'];q=value(s['coefficients'],dt);yy=float(q)
        assert yy==row['returned_binary64']
        dis=abs(Q.from_float(yy)-q)
        assert dis<=round_bound
        bound=min(flow_bound(s['initial_radius'],s['defect_upper'],dt)+dis,s['continuous_output_bound'])
        assert bound==Q(row['bound_rational']) and bound<=tol
        dense_count+=1
    # Recompute trial admission without advancing a rejected state.
    index=0
    for row in data['trials']:
        assert index<len(segments)
        s=segments[index];h=Q(row['h']);left=Q(row['t'])
        assert left==s['left']
        c=s['coefficients'];delta=max(abs(v) for v in bernstein(defect(c),h))
        whole=max(s['initial_radius'],flow_bound(s['initial_radius'],delta,h))+round_bound
        admitted=min(bernstein(c,h))>=0 and max(bernstein(c,h))<=1 and whole<=tol
        assert admitted==row['accepted']
        if admitted:
            assert h==s['h'];index+=1
    assert index==len(segments)
    result={'status':'PASS_MODEL_SPECIFIC_EXACT_PROOF_DATA_READBACK',
        'segments_verified':len(segments),'trials_verified':len(data['trials']),
        'dense_outputs_verified':dense_count,'reference_truth_used_for_admission':False,
        'state_derivative_oracles_used':False,
        'proof_premises':['exact rational arithmetic','Bernstein convex hull',
            'nonnegative true solution invariance for stated scalar ODE',
            'two-point dissipativity mu=-2','alternating exponential remainder bound',
            'binary64 Fraction conversion with explicit displacement'],
        'not_claimed':['generic FVM certification','independent-personnel review','proof-assistant formalization','native speed']}
    (ROOT/'CERTIFICATE_READBACK.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))

if __name__=='__main__':verify()
