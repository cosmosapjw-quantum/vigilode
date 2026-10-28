"""Model-specific rational adaptive curve and binary64 dense-output certificate.

Equation: y'=-2y-y^2, y(0)=1/2; time in a fixed unit, state nondimensional.
No supplied/computed state Jacobian, no JVP or nonlinear derivative. Polynomial
compositions are exact Fraction arithmetic. The only rounded state operations
are explicit binary64 output conversions, and their displacement is included.
This is a proof-of-concept, not a performance-competitive production solver.
"""
from fractions import Fraction as F
from pathlib import Path
import json, math
import mpmath as mp
from experiments import bernstein,polyadd,polymul
ROOT=Path(__file__).resolve().parent
ROUND=F(1,2**53) # uniform safe bound on nearest float conversion of values in[0,1]

def polynomial(center,p=5):
    c=[center]
    for k in range(p):
        c.append((-2*c[k]-sum((c[j]*c[k-j] for j in range(k+1)),F(0)))/F(k+1))
    defect=polyadd(polyadd([(i+1)*c[i+1] for i in range(p)],[2*x for x in c]),polymul(c,c))
    return c,defect

def exp_bounds(t):
    z=2*t
    assert 0<=z<=1
    lower=sum(((-z)**k/F(math.factorial(k)) for k in range(12)),F(0))
    upper=lower+z**12/F(math.factorial(12))
    assert 0<lower<=upper<=1
    return lower,upper

def propagate(initial,delta,t):
    lo,hi=exp_bounds(t)
    return hi*initial+delta*(1-lo)/2

def value(c,t):
    q=F(0)
    for a in reversed(c):q=q*t+a
    return q

def solve(t_end=F(1),tol=F(1,10**6)):
    t=F(0);center=F(1,2);radius=F(0);h=F(1,4)
    segments=[];trials=[]
    for attempt in range(1000):
        if t==t_end:break
        h=min(h,t_end-t)
        before=(t,center,radius)
        c,d=polynomial(center)
        bc=bernstein(c,h);bd=bernstein(d,h)
        delta=max(abs(x) for x in bd)
        positive=min(bc)>=0 and max(bc)<=1
        flow_end=propagate(radius,delta,h)
        continuous_output_bound=max(radius,flow_end)+ROUND
        accepted=positive and continuous_output_bound<=tol
        trials.append({'t':str(t),'h':str(h),'accepted':accepted,
                       'bound_float':float(continuous_output_bound)})
        if not accepted:
            h/=2
            assert before==(t,center,radius)
            continue
        qend=value(c,h);ynew=F.from_float(float(qend));rounding=abs(qend-ynew)
        assert rounding<=ROUND
        new_radius=flow_end+rounding
        segments.append({'left':t,'h':h,'coefficients':c,'initial_radius':radius,
                         'defect_upper':delta,'endpoint_center':ynew,'endpoint_radius':new_radius,
                         'continuous_output_bound':continuous_output_bound})
        t+=h;center=ynew;radius=new_radius
        if continuous_output_bound<tol/4:h=min(2*h,F(1,4))
    assert t==t_end
    return segments,trials

def evaluate(segment,t):
    local=t-segment['left'];assert 0<=local<=segment['h']
    exact_curve=value(segment['coefficients'],local)
    output=float(exact_curve);displacement=abs(F.from_float(output)-exact_curve)
    assert displacement<=ROUND
    bound=propagate(segment['initial_radius'],segment['defect_upper'],local)+displacement
    # Both the query-specific comparison and the whole-window comparison are
    # valid bounds; intersect them without changing any tolerance or data.
    bound=min(bound,segment['continuous_output_bound'])
    return output,bound

def main():
    tol=F(1,10**6);segments,trials=solve(tol=tol)
    mp.mp.dps=90;rows=[]
    def high(q):return mp.mpf(q.numerator)/q.denominator
    for i in range(101):
        t=F(i,100)
        s=next(x for x in segments if x['left']<=t<=x['left']+x['h'])
        y,bound=evaluate(s,t)
        z=mp.exp(-2*high(t));exact=mp.mpf('0.5')*z/(1+mp.mpf('.25')*(1-z))
        actual=abs(mp.mpf(y)-exact)
        assert actual<=high(bound)+mp.mpf('1e-80')
        assert bound<=tol
        rows.append({'time_rational':str(t),'returned_binary64':y,'bound_rational':str(bound),
                     'bound_float':float(bound),'observed_error_90digits':float(actual)})
    # Check every entire-segment gate and chain link with rational arithmetic.
    for i,s in enumerate(segments):
        assert s['continuous_output_bound']<=tol
        if i:
            p=segments[i-1]
            assert s['left']==p['left']+p['h']
            assert s['coefficients'][0]==p['endpoint_center']
            assert s['initial_radius']==p['endpoint_radius']
    serial=[{k:[str(v) for v in x] if isinstance(x,list) else str(x) for k,x in s.items()} for s in segments]
    result={'status':'MODEL_SPECIFIC_RATIONAL_CERTIFICATE_AND_BINARY64_OUTPUT_CHECKED',
        'scope':'scalar polynomial ODE; exact Fraction proof data; binary64 centers with explicit rounding displacement',
        'equation':"y'=-2y-y^2",'initial':'1/2','end_time':'1','tolerance':str(tol),
        'accepted_windows':len(segments),'rejected_trials':sum(not r['accepted'] for r in trials),
        'dense_points_checked':len(rows),'derivative_calls':0,
        'max_dense_actual_error':max(x['observed_error_90digits'] for x in rows),
        'max_dense_certified_radius':max(x['bound_float'] for x in rows),
        'nonnegative_candidate_on_every_window':True,'failed_trials_mutated_accepted_state':False,
        'performance_claim':False,'generic_rhs_or_FVM_certificate':False,
        'segments':serial,'trials':trials,'dense_outputs':rows}
    (ROOT/'ADAPTIVE_SCALAR_RESULT.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({k:v for k,v in result.items() if k not in ('segments','trials','dense_outputs')},indent=2))

if __name__=='__main__':main()
