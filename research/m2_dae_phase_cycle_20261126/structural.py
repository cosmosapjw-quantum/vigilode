from fractions import Fraction as Q
from math import comb
import bisect
ZERO=Q(0);ONE=Q(1)
def q(x):return Q(x)
def add(a,b):
    return [ (a[i] if i<len(a) else ZERO)+(b[i] if i<len(b) else ZERO) for i in range(max(len(a),len(b)))]
def scale(a,k):return [x*k for x in a]
def sub(a,b):return add(a,scale(b,-1))
def mul(a,b):
    r=[ZERO]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b):r[i+j]+=x*y
    return r
def deriv(a):return [k*v for k,v in enumerate(a) if k] or [ZERO]
def ev(a,t):
    r=ZERO
    for x in a[::-1]:r=r*t+x
    return r
def restrict(a,l,w):
    n=len(a)-1
    return [sum((a[j]*comb(j,k)*l**(j-k)*w**k for j in range(k,n+1)),ZERO) for k in range(n+1)]
def bern(a):
    n=len(a)-1
    return [sum((a[j]*Q(comb(i,j),comb(n,j)) for j in range(i+1)),ZERO) for i in range(n+1)]
def br(a,l,w):return bern(restrict(a,l,w))
def mag(a):return max(map(abs,a))
def ceil80(x):
    assert x>=0
    z=x*(1<<80)
    return Q(-(-z.numerator//z.denominator),1<<80)
def native(s):
    r=s['native'];n=len(r['y0']);co=list(map(q,r['coefficients']))
    assert len(co)==24 and len(r['stages'])==8 and all(len(v)==n for v in r['stages'])
    out=[]
    for i in range(n):
        ds=[sum((co[k*8+j]*q(r['stages'][j][i])for j in range(8)),ZERO) for k in range(3)]
        a,b=q(r['y0'][i]),q(r['y1'][i])
        pp=[a,b-a+ds[0],ds[1]-ds[0],ds[2]-ds[1],-ds[2]]
        assert ev(pp,ZERO)==a and ev(pp,ONE)==b
        assert all(q(box[0])<=c<=q(box[1]) for c,box in zip(pp,s['power'][i]))
        out.append(pp)
    return out
def f_rober(u,v,c):
    z=sub([ONE],add(u,v));b=mul(v,z);a=scale(u,q(.04));f=sub(scale(b,Q(10000)),a)
    return [f,sub(scale(f,-1),scale(mul(v,v),Q(c)))]
def full_approx(case,p,u,time):
    if case=='robertson':
        a,b=ev(p[0],u),ev(p[1],u);return [a,b,1-a-b]
    if case=='sqrt-stiff':
        z=ev(p[0],u);return [z*z,z]
    x=ev(p[0],u);return [x,x/(1+time/8)]
def proof(case,run,method,initial,rate=30000000,prior_error=ZERO):
    if not (run['driver_success'] and run['pointwise'] and run['branch'] and run['slabs']):
        return {'status':'no-native-branch'}
    ns=1 if method=='StructMax'else 4
    err=prior_error;last=list(map(q,initial));last_t=q(run['slabs'][0]['times'][0])
    total_res=ZERO;max_mu=ZERO;max_gap=ZERO;max_ideal=ZERO;records=[];jumps=ZERO;coeff_checks=0
    for slab in run['slabs']:
        a,b=map(q,slab['times']);h=b-a
        assert a==last_t and h>0
        p=native(slab);coeff_checks+=5*len(p)
        if case=='robertson':p=p[:2];f=f_rober(*p,rate)
        elif case=='sqrt-stiff':
            if len(p)==1:
                p=[[(q(a)+q(b))/2 for a,b in slab['power'][1]]]
            else:p=p[1:2]
            f=[scale(sub(p[0],[ONE]),-1024)]
        elif case=='pencil-stiff':p=p[:1];f=[scale(sub(p[0],[ONE]),-1024)]
        else:raise ValueError(case)
        j=sum((abs(ev(row,ZERO)-v)for row,v in zip(p,last)),ZERO)
        err=ceil80(err+j);jumps+=j
        residual=[sub(deriv(x),scale(y,h))for x,y in zip(p,f)]
        cells=[]
        for k in range(ns):
            l=Q(k,ns);w=Q(1,ns)
            mu=ZERO
            if case=='robertson':
                vl=min(br(p[1],l,w));zl=min(br(sub([ONE],add(*p)),l,w))
                vm=max(ZERO,-vl);zm=max(ZERO,-zl)
                mu=max(ZERO,2*(-q(.04)+10000*vm),2*rate*vm+20000*zm)
            v=h*w*mu;max_mu=max(max_mu,mu);max_gap=max(max_gap,v)
            if v>=1:return {'status':'growth-cap','slab':[float(a),float(b)],'mu':float(mu),'h_mu':float(v)}
            bs=[br(r,l,w)for r in residual]
            rho=w*sum((mag(bc)if ns==1 else sum(map(abs,bc),ZERO)/len(bc)for bc in bs),ZERO)
            total_res+=rho
            e_in=err;err=ceil80((err+rho)/(1-v))
            conv=max(ONE,Q(17,16)+mag(br(p[0],l,w)))if case=='sqrt-stiff'else ONE
            bd=ceil80(err*conv);max_ideal=max(max_ideal,bd)
            cells.append({'normalized':[l,l+w],'e_in':e_in,'e_out':err,'state_bound':bd,'mu':mu,'defect_integral':rho})
        records.append({'a':a,'b':b,'p':p,'cells':cells,'jump':j})
        last=[ev(row,ONE)for row in p];last_t=b
    ends=[r['b']for r in records]
    queries=[];max_bound=max_ideal
    for arr in ['accepted','queries']:
        for x in run[arr]:
            time=q(x['t']);idx=bisect.bisect_left(ends,time);assert idx<len(records)
            r=records[idx];u=(time-r['a'])/(r['b']-r['a']);assert ZERO<=u<=ONE
            cell=next(c for c in r['cells']if c['normalized'][0]<=u<=c['normalized'][1])
            ideal=full_approx(case,r['p'],u,time);corr=max(abs(a-q(b))for a,b in zip(ideal,x['state']))
            assert len(ideal)==len(x['state'])
            bd=ceil80(cell['state_bound']+corr);max_bound=max(max_bound,bd)
            queries.append({'kind':arr,'t':x['t'],'bound':bd,'correction':corr})
    return {'status':'certified','bound':max_bound,'max_ideal':max_ideal,'max_mu':max_mu,'max_h_mu':max_gap,'residual_integral':total_res,'jumps':jumps,'e_out':err,'endpoint':last,'queries':queries,'records':records,'coefficient_checks':coeff_checks}
def summary(p):
    return {k:(float(v) if isinstance(v,Q)else v)for k,v in p.items()if k not in ('queries','records','endpoint')}
