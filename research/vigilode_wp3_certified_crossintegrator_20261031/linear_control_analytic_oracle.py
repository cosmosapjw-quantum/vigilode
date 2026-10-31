#!/usr/bin/env python3
"""Independent exact-form linear cooperative negative-control analytic solution.
Checks WP2 dyadic reference and improved a posteriori bounds, without trusting RK4.
"""
import json,sys,hashlib,struct
from fractions import Fraction as F
from pathlib import Path
import mpmath as mp
from scipy.integrate import solve_ivp
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'sources'))
import DEFECT_CERTIFIER as d

def mfrac(q):return mp.mpf(int(q.numerator))/mp.mpf(int(q.denominator))
def analytical(src,t):
    assert src['n']==2 and src['k']==[F(4),F(4)] and src['g']==[F(0),F(0)]
    assert src['c']==[[F(0),F(6)],[F(6),F(0)]]
    a,b=src['y0']; mean=(a+b)/2;half=(a-b)/2
    # Eigenvalues are -4 and -16 in exact real arithmetic.
    return (mfrac(mean)*mp.exp(-4*mfrac(t))+mfrac(half)*mp.exp(-16*mfrac(t)),
            mfrac(mean)*mp.exp(-4*mfrac(t))-mfrac(half)*mp.exp(-16*mfrac(t)))

def main():
    mp.mp.dps=95
    cohort='linear_exchange_control'
    traces,_=d.parse_log(ROOT/'sources/SWEEP.log')
    tr=traces[(cohort,2,False)];src=d.parse_source(tr);n=len(tr['points'])-1
    cert=json.loads((ROOT/'sources/CERTIFICATE.json').read_text())
    result={}
    for micro in (32,64):
        obj=next(x for x in cert['micro_cohorts'][cohort] if x['reference_micro']==micro)
        blob=(ROOT/'sources'/obj['reference_states_name']).read_bytes()
        assert hashlib.sha256(blob).hexdigest()==obj['reference_states_sha256']
        orig=mp.mpf(0)
        for j in range(n+1):
            c=struct.unpack('>dd',blob[j*micro*16:j*micro*16+16]); exact=analytical(src,src['h']*j)
            for a,b in zip(c,exact):
                orig=max(orig,abs(mfrac(F.from_float(a))-b))
        bounded=mp.mpf(obj['reference_delta_grid_up'])
        assert orig <= bounded,('ORIGINAL_DEFECT_BOUND_VIOLATION',micro,orig,bounded)
        improved=json.loads((ROOT/'results'/f'RATIONAL_ENCLOSURE_{cohort}_micro{micro}.json').read_text()) if (ROOT/'results'/f'RATIONAL_ENCLOSURE_{cohort}_micro{micro}.json').exists() else None
        if improved is not None:
            assert orig<=mp.mpf(improved['best_new_delta'])
        result[str(micro)]={'observed_true_ref_peak':mp.nstr(orig,28),
           'original_wp2_rigorous_upper':str(obj['reference_delta_grid_up']),
           'new_wp3_rigorous_upper':improved['best_new_delta'] if improved else None,
           'reference_sha256':obj['reference_states_sha256']}
    # Compare the same negative-control analytical trajectory to new stiff solver
    # output at the ORIGINAL physical clock labels.
    t=[d.rational(p['time']) for p in tr['points']]
    import numpy as np
    yy=np.array([float(q) for q in src['y0']]);A=np.array([[-10,6],[6,-10.]],float)
    for method in ('RK45','DOP853','Radau','BDF','LSODA'):
        kw={'jac':lambda t,y:A} if method in ('Radau','BDF','LSODA') else {}
        x=solve_ivp(lambda t,y:A@y,(0.0,0.1),yy,t_eval=[float(p) for p in t],rtol=1e-10,atol=1e-12,method=method,**kw)
        assert x.success
        e=mp.mpf(0)
        for i,v in enumerate(x.y.T):
            ax=analytical(src,t[i]);e=max(e,*(abs(mfrac(F.from_float(float(a)))-b) for a,b in zip(v,ax)))
        result[method]={'analytic_peak_global_error':mp.nstr(e,28),'nfev':int(x.nfev),'nlu':int(x.nlu)}
    out=ROOT/'results/LINEAR_CONTROL_ANALYTIC_ORACLE.json';out.write_text(json.dumps(result,indent=2,sort_keys=True)+'\n')
    print('ANALYTIC_LINEARN_CONTROL=PASS',json.dumps(result)[:2600]);return result

if __name__=='__main__':main()
