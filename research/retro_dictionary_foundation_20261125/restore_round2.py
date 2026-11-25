"""Transcribed archived Round02 offline certificate and auxiliary generators."""
import hashlib,json,math,itertools
from fractions import Fraction as F
from pathlib import Path
R=Path(__file__).resolve().parent/'restored'
source=R/'PHI_TABLE.json';T=json.loads(source.read_text());m=T['degree'];cells=T['cells']
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def f(x):return F.from_float(float(x))
def outward(x):
    v=float(x)
    if not math.isfinite(v):raise OverflowError('bound')
    return math.nextafter(v,math.inf) if f(v)<x else v
def deriv(p):return [(i+1)*p[i+1] for i in range(m)]+[F(0)]
def endpoint(p,s):return sum(v*s**i for i,v in enumerate(p))
def abs1(p):return sum(map(abs,p))
eps=F(1,2**53);eta=F(1,2**1022);du=F(1,2**50);rho=1+du
rp=[rho**k for k in range(m+1)];gamma=2*m*eps/(1-2*m*eps)
def evaluation_bound(p,r):
    assert 2*eps+eps**2+eta*(1+eps)/r+eta<=du
    amp=sum(abs(v)*rp[k] for k,v in enumerate(p));slope=sum(k*abs(v)*rp[k-1] for k,v in enumerate(p) if k)
    assert amp<1000
    return du*slope+gamma*amp+2*m*eta*rp[m]/(1-2*m*eps)
def residual(p,parent,c,r,d,coef):
    dp=deriv(p)
    if parent is None:return [p[i]-dp[i]/r for i in range(m+1)]
    return [d*p[i]+c/r*dp[i]+(dp[i-1] if i else 0)-coef*parent[i] for i in range(m+1)]
def accumulate(p,par,bpar,prev,c,r,a,b,d,coef,initial):
    delta=abs1(residual(p,par,c,r,d,coef))
    ea=abs(endpoint(p,1)-initial) if prev is None else prev[1]+abs(endpoint(p,1)-prev[0])
    if par is None:
        B=max(ea,delta);eb=delta+(ea-delta)/(1+b-a) if ea>delta else delta
    else:
        target=(coef*bpar+delta)/d;B=max(ea,target);s=(a/b)**d;eb=s*ea+(1-s)*target
    ev=evaluation_bound(p,r)
    return (endpoint(p,-1),eb),B,ev,delta
keys=[(p,k) for p in range(8) for k in range(p+1)]
prev={};bounds=[];derived=[];prev_b=F(0)
for index,cell in enumerate(cells):
    a,b=-f(cell['hi']),-f(cell['lo']);c,r=f(cell['center']),f(cell['radius'])
    assert a==prev_b and b>a and -c-r==a and -c+r==b and r>0
    prev_b=b;phi=[list(map(f,p)) for p in cell['coefficients']]
    assert len(phi)==9 and all(len(p)==m+1 for p in phi)
    B={};rec={'a':float(a),'b':float(b),'phi':[],'bernstein':[]};dc={'center':float(c),'radius':float(r),'lo':float(-b),'hi':float(-a),'coefficients':[]}
    def process(key,p,parent,d,coef,initial,target):
        pa=None if parent is None else parent[0];bp=F(0) if parent is None else B[parent[1]]
        st,bb,ee,rr=accumulate(p,pa,bp,prev.get(key),c,r,a,b,d,coef,initial)
        prev[key]=st;B[key]=bb
        rec[target].append({'polynomial_error_upper':outward(bb),'evaluation_error_upper':outward(ee),'total_error_upper':outward(bb+ee),'ode_residual_upper':outward(rr)})
    for j,p in enumerate(phi):
        process(('phi',j),p,None if j==0 else (phi[j-1],('phi',j-1)),j,1,F(1,math.factorial(j)),'phi')
    h={}
    for p,k in keys:
        terms=[(math.comb(p,k)*(-1)**l*math.comb(p-k,l)*math.factorial(k+l),phi[k+l+1]) for l in range(p-k+1)]
        exact=[sum(w*v[j] for w,v in terms) for j in range(m+1)]
        rounded=[float(v) for v in exact];pp=list(map(f,rounded));h[p,k]=pp;dc['coefficients'].append(rounded)
        d=p-k+1;parent=(phi[0],('phi',0)) if k==0 else (h[p,k-1],('B',p,k-1))
        process(('B',p,k),pp,parent,d,1 if k==0 else d,F(1,p+1),'bernstein')
    bounds.append(rec);derived.append(dc)
    if index%8==0:print('certificate cell',index,flush=True)
assert prev_b==16384
cert={'schema':'retro-warp.table-certificate.v2','source_sha256':sha(source),'degree':m,'keys':keys,'cells':bounds,'assumptions':'exact represented binary64 table coefficients and argument; exact rational residual l1 in u; continuous x ODE; runtime RN binary64, gradual underflow, separate rounded operations; no external exp accuracy dependency for bounds'}
Btab={'schema':'retro-warp.bernstein-moment.v1','derived_from':sha(source),'degree':m,'keys':keys,'cells':derived}
(R/'BERNSTEIN_MOMENTS.json').write_text(json.dumps(Btab,separators=(',',':'))+'\n')
cert['bernstein_sha256']=sha(R/'BERNSTEIN_MOMENTS.json')
(R/'TABLE_CERTIFICATE.json').write_text(json.dumps(cert,separators=(',',':'))+'\n')
A=64;hi=16384;expu=1/sum(F(A**i,math.factorial(i)) for i in range(101))
rows=[]
for p in range(8):
    for k in range(p+1):
        j=p-k
        coeff=[math.comb(p,k)*(-1)**l*math.comb(k,l)*math.factorial(j+l) for l in range(k+1)]
        amp=sum(F(abs(c),A**l) for l,c in enumerate(coeff));assert max(map(abs,coeff))<2**53
        tail=math.comb(p,k)*expu*sum(math.comb(j,l)*math.factorial(k+l)*F(A)**(j-k-l) for l in range(j+1))
        rounding=40*eps/(1-40*eps)*amp+40*eta*hi**(j+1)/(1-40*eps)
        rows.append({'p':p,'k':k,'inverse_power':j+1,'coefficients':coeff,'scaled_tail_upper':outward(tail),'scaled_roundoff_upper':outward(rounding),'scaled_total_upper':outward(tail+rounding)})
(R/'SCALED_MOMENTS.json').write_text(json.dumps({'schema':'retro-warp.scaled-moment.v1','domain':[A,hi],'rows':rows},indent=2)+'\n')
src=R/'LAGRANGE_TABLE.json';lagrange=json.loads(src.read_text())['records'];records={}
for name,row in lagrange.items():
    m=int(name);p=m-1;L=[[F(v) for v in r] for r in row['coefficients_ascending']]
    B=[[sum(L[j][k]*F(math.comb(i,k),math.comb(p,k)) for k in range(i+1)) for i in range(m)] for j in range(m)]
    for j in range(m):
        for k in range(m):
            val=sum(B[j][i]*math.comb(p,i)*math.comb(p-i,k-i)*(-1)**(k-i) for i in range(k+1))
            assert val==L[j][k]
    records[name]={'nodes':row['nodes'],'monomial':row['coefficients_ascending'],'bernstein':[[str(v) for v in r] for r in B],'restriction_binomial':[[math.comb(i,k) if k<=i else 0 for k in range(m)] for i in range(m)],'damping_lift_ratios':{str(l):[str(F(math.comb(p,i),math.comb(p+l,i))) for i in range(m)] for l in range(8-p)}}
(R/'BASIS_OPERATOR_TABLE.json').write_text(json.dumps({'schema':'retro-warp.basis-operator.v1','source_sha256':sha(src),'records':records},indent=2)+'\n')
words=[]
for length in range(1,6):
    for w in itertools.product(range(3),repeat=length):
        d=math.prod(sum(k+1 for k in w[i:]) for i in range(length));words.append({'word':w,'degree':length+sum(w),'coefficient':str(F(1,d))})
(R/'ORDERED_PICARD_TABLE.json').write_text(json.dumps({'schema':'retro-warp.ordered-picard.v1','outermost_first':True,'max_time_degree':2,'max_word_length':5,'records':words},indent=2)+'\n')
pmax=8;mmax=40
series=[[str(F(1,math.factorial(j+k))) for k in range(mmax+1)] for j in range(pmax+1)]
(R/'COMPLEX_PHI_SERIES.json').write_text(json.dumps({'domain':'Re z<=0; Taylor for |z|<=4, recurrence otherwise','degree':mmax,'coefficients':series,'remainder_at_4':[str(F(4**(mmax+1),math.factorial(mmax+1+j))) for j in range(pmax+1)]},indent=2)+'\n')
print('round2 restored',flush=True)
