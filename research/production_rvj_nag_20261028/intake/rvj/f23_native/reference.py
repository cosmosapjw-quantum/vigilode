"""Exact Fraction oracle for original-equation polynomial Hermite slab bounds."""
from fractions import Fraction as F
from math import comb

def solve(q,f):
    n=len(q);b=[q[(i+1)%n]/2 for i in range(n)];r=f[-1];p=-b[-1]
    for i in range(n-2,-1,-1):r,p=f[i]-b[i]*r,-b[i]*p
    v=[F(0)]*n;v[0]=r/(1-p);v[-1]=f[-1]-b[-1]*v[0]
    for i in range(n-2,0,-1):v[i]=f[i]-b[i]*v[i+1]
    return v

def slow(x,q):return [-a+(q[i]+q[(i+1)%len(q)])/16+F(1,32)+F(i%3,256) for i,a in enumerate(x)]
def slope(y):
    n=len(y)//2;f=slow(y[:n],y[n:]);return f+solve(y[n:],f)
def add(a,b):return [(a[i] if i<len(a) else F(0))+(b[i] if i<len(b) else F(0)) for i in range(max(len(a),len(b)))]
def scale(a,c):return [v*c for v in a]
def mul(a,b):
    r=[F(0)]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b):r[i+j]+=x*y
    return r

def bounds(a):
    n=len(a)-1;b=[sum((F(comb(i,k),comb(n,k))*a[k] for k in range(i+1)),F(0)) for i in range(n+1)]
    return min(b),max(b)
def mag(a):return max(map(abs,bounds(a)))
def certificate(y0,y1,h,e):
    y0=list(map(F,y0));y1=list(map(F,y1));h=F(h);e=F(e);n=len(y0)//2
    f0=slope(y0);f1=slope(y1)
    p=[[a,h*c,3*(b-a)-h*(2*c+d),2*(a-b)+h*(c+d)] for a,b,c,d in zip(y0,y1,f0,f1)]
    assert all(mag(v)<=(F(1,8) if i<n else F(1,4)) for i,v in enumerate(p))
    dg=df=F(0)
    for i in range(n):
        j=(i+1)%n
        g=add(add(p[n+i],scale(mul(p[n+j],p[n+j]),F(1,4))),scale(p[i],-1))
        ff=add(add(scale(p[i],-1),scale(add(p[n+i],p[n+j]),F(1,16))),[F(1,32)+F(i%3,256)])
        d=add([k*p[i][k]/h for k in range(1,4)],scale(ff,-1))
        dg=max(dg,mag(g));df=max(df,mag(d))
    delta=df+dg/7;ew=e+h*delta;ee=e/(1+6*h/7)+h*delta;qw=F(8,7)*(ew+dg)
    return dict(dg=dg,df=df,ew=ew,ee=ee,qw=qw)
