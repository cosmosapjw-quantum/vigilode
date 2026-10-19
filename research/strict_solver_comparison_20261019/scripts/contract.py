"""Fail-closed sampled-error accounting. The radius is empirical, not certified."""
import math

def assess(row, reference, radius, times, scale):
    bad = {'admitted': False, 'reason': None, 'E_raw': None, 'E_reference': None, 'E_up': None, 'R_raw': None, 'R_reference': None, 'R_up': None}
    def reject(reason):
        return dict(bad, reason=reason)
    def finite(x):
        return isinstance(x,(int,float)) and not isinstance(x,bool) and math.isfinite(x)
    if row.get('status')!='ok':return reject('SOLVER_NOT_SUCCESSFUL')
    if row.get('deterministic',True) is not True:return reject('NONDETERMINISTIC_REPETITIONS')
    if not isinstance(times,list) or len(times)<2 or not all(finite(t) for t in times):return reject('INVALID_TIMES')
    if any(b<=a for a,b in zip(times,times[1:])):return reject('NONINCREASING_TIMES')
    if row.get('times')!=times:return reject('OUTPUT_TIME_MISMATCH')
    y=row.get('states')
    if not isinstance(reference,list) or len(reference)!=len(times) or not reference or not isinstance(reference[0],list) or not reference[0]:return reject('REFERENCE_SHAPE')
    n=len(reference[0]);nt=len(times)
    for label,a in [('STATE',y),('REFERENCE',reference),('RADIUS',radius)]:
        if not isinstance(a,list) or len(a)!=nt or any(not isinstance(v,list) or len(v)!=n for v in a):return reject(label+'_SHAPE')
        if not all(finite(x) for v in a for x in v):return reject(label+'_NONFINITE')
    if any(x<0 for v in radius for x in v):return reject('NEGATIVE_RADIUS')
    rt=row.get('rtol');at=row.get('atol')
    if not all(finite(x) and x>0 for x in [scale,rt,at]):return reject('INVALID_TOLERANCE')
    def rms(a):return math.sqrt(math.fsum(x*x for x in a)/n)
    E=[];Er=[];R=[];Rr=[]
    for yy,rr,rho in zip(y,reference,radius):
        d=[scale+abs(x) for x in rr];w=[at+rt*abs(x) for x in rr]
        E.append(rms([(a-b)/z for a,b,z in zip(yy,rr,d)]));Er.append(rms([a/z for a,z in zip(rho,d)]))
        R.append(rms([(a-b)/z for a,b,z in zip(yy,rr,w)]));Rr.append(rms([a/z for a,z in zip(rho,w)]))
    result={'admitted':True,'reason':'FULL_FINITE_GRID','E_raw':max(E),'E_reference':max(Er),'E_up':max(E)+max(Er),'R_raw':max(R),'R_reference':max(Rr),'R_up':max(R)+max(Rr)}
    if not all(finite(result[k]) for k in ['E_raw','E_reference','E_up','R_raw','R_reference','R_up']):return reject('NONFINITE_METRIC')
    return result
