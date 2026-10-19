"""Independent equations and analytic derivatives, separate from Rust callbacks."""
import numpy as np

def model(name, n):
    if name == 'robertson':
        def f(t,y):
            a,b,c=y
            return np.array([-0.04*a+1e4*b*c,0.04*a-1e4*b*c-3e7*b*b,3e7*b*b])
        def j(t,y):
            a,b,c=y
            return np.array([[-.04,1e4*c,1e4*b],[.04,-1e4*c-6e7*b,-1e4*b],[0.,6e7*b,0.]])
    elif name == 'hires':
        def f(t,y):
            return np.array([-1.71*y[0]+.43*y[1]+8.32*y[2]+.0007,1.71*y[0]-8.75*y[1],
                -10.03*y[2]+.43*y[3]+.035*y[4],8.32*y[1]+1.71*y[2]-1.12*y[3],
                -1.745*y[4]+.43*y[5]+.43*y[6],-280*y[5]*y[7]+.69*y[3]+1.71*y[4]-.43*y[5]+.69*y[6],
                280*y[5]*y[7]-1.81*y[6],-280*y[5]*y[7]+1.81*y[6]])
        def j(t,y):
            a=np.zeros((8,8),dtype=np.result_type(y,float))
            for i,k,z in [(0,0,-1.71),(0,1,.43),(0,2,8.32),(1,0,1.71),(1,1,-8.75),
                (2,2,-10.03),(2,3,.43),(2,4,.035),(3,1,8.32),(3,2,1.71),(3,3,-1.12),
                (4,4,-1.745),(4,5,.43),(4,6,.43),(5,3,.69),(5,4,1.71),(5,6,.69),
                (5,5,-280*y[7]-.43),(5,7,-280*y[5]),(6,5,280*y[7]),(6,6,-1.81),
                (6,7,280*y[5]),(7,5,-280*y[7]),(7,6,1.81),(7,7,-280*y[5])]: a[i,k]=z
            return a
    elif name == 'van-der-pol-mu1000':
        def f(t,y): return np.array([y[1],1000*(1-y[0]*y[0])*y[1]-y[0]])
        def j(t,y): return np.array([[0.,1.],[-2000*y[0]*y[1]-1,1000*(1-y[0]*y[0])]])
    elif name.startswith('brusselator-1d-'):
        m=n//2; d=(m+1)**2/50
        def f(t,y):
            u=y[::2]; v=y[1::2]
            up=np.r_[1.,u,1.]; vp=np.r_[3.,v,3.]
            out=np.empty(n,dtype=np.result_type(y,float))
            out[::2]=1+u*u*v-4*u+d*(up[:-2]-2*u+up[2:])
            out[1::2]=3*u-u*u*v+d*(vp[:-2]-2*v+vp[2:])
            return out
        def j(t,y):
            a=np.zeros((n,n),dtype=np.result_type(y,float))
            for k in range(m):
                i=2*k; u,v=y[i:i+2]
                a[i,i]=2*u*v-4-2*d; a[i,i+1]=u*u
                a[i+1,i]=3-2*u*v; a[i+1,i+1]=-u*u-2*d
                if k: a[i,i-2]=a[i+1,i-1]=d
                if k+1<m: a[i,i+2]=a[i+1,i+3]=d
            return a
    elif name == 'pivot-linear':
        A=np.array([[-1.,0.],[1000.,-1000.]])
        def f(t,y): return A@y
        def j(t,y): return A.copy()
    else: raise ValueError(name)
    return f,j

def exact_pivot(times):
    t=np.asarray(times)
    return np.c_[np.exp(-t),(1000/999)*np.exp(-t)*(-np.expm1(-999*t))]

def packed_band(j, lower, upper):
    n=j.shape[0]; out=np.zeros((lower+upper+1,n))
    for k in range(n):
        for i in range(max(0,k-upper),min(n,k+lower+1)):
            out[upper+i-k,k]=j[i,k]
    return out
