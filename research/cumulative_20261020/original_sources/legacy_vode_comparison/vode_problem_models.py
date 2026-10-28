"""Original VigilODE benchmark IVP reconstructions from the pinned Rust sources.
No performance conclusions or benchmark run in this module.
"""
import numpy as np

class Problem:
    def __init__(self,name,n,y0,tf,atol_scale,rhs,jac,lband=None,uband=None):
        self.name,self.n,self.y0,self.tf,self.atol_scale,self.rhs,self.jac,self.lband,self.uband=name,n,np.asarray(y0,dtype=float),tf,atol_scale,rhs,jac,lband,uband

def robertson():
    def f(t,y):
        a,b,c=y
        return np.array([-0.04*a+1e4*b*c,0.04*a-1e4*b*c-3e7*b*b,3e7*b*b])
    def j(t,y):
        a,b,c=y
        return np.array([[-0.04,1e4*c,1e4*b],[0.04,-1e4*c-6e7*b,-1e4*b],[0,6e7*b,0]])
    return Problem('robertson',3,[1,0,0],40.,1e-4,f,j)

def hires():
    def f(t,y):
        a,b,c,d,e,f,g,h=y
        return np.array([-1.71*a+0.43*b+8.32*c+0.0007,1.71*a-8.75*b,-10.03*c+0.43*d+0.035*e,8.32*b+1.71*c-1.12*d,-1.745*e+0.43*f+0.43*g,-280*f*h+0.69*d+1.71*e-0.43*f+0.69*g,280*f*h-1.81*g,-280*f*h+1.81*g])
    def j(t,y):
        a,b,c,d,e,f,g,h=y
        J=np.zeros((8,8)); J[0,0]=-1.71;J[0,1]=.43;J[0,2]=8.32;J[1,0]=1.71;J[1,1]=-8.75
        J[2,2]=-10.03;J[2,3]=.43;J[2,4]=.035;J[3,1]=8.32;J[3,2]=1.71;J[3,3]=-1.12
        J[4,4]=-1.745;J[4,5]=.43;J[4,6]=.43;J[5,3]=.69;J[5,4]=1.71;J[5,5]=-280*h-.43
        J[5,6]=.69;J[5,7]=-280*f;J[6,5]=280*h;J[6,6]=-1.81;J[6,7]=280*f;J[7,5]=-280*h;J[7,6]=1.81;J[7,7]=-280*f
        return J
    return Problem('hires',8,[1,0,0,0,0,0,0,.0057],321.8122,1e-4,f,j)

def vdp():
    mu=1000.
    def f(t,y):
        return np.array([y[1],mu*(1-y[0]**2)*y[1]-y[0]])
    def j(t,y):
        return np.array([[0,1],[-2*mu*y[0]*y[1]-1,mu*(1-y[0]**2)]])
    return Problem('van-der-pol-mu1000',2,[2,0],2000.,1.,f,j)

def brusselator(cells):
    c=(cells+1)**2/50.;n=2*cells
    x=(np.arange(cells)+1)/(cells+1)
    y0=np.column_stack([1+np.sin(2*np.pi*x),np.full(cells,3.)]).reshape(-1)
    def f(t,y):
        u=y[::2];v=y[1::2];ul=np.concatenate(([1.],u[:-1]));ur=np.concatenate((u[1:],[1.]));vl=np.concatenate(([3.],v[:-1]));vr=np.concatenate((v[1:],[3.]));out=np.empty_like(y)
        out[::2]=1+u*u*v-4*u+c*(ul-2*u+ur)
        out[1::2]=3*u-u*u*v+c*(vl-2*v+vr)
        return out
    def j(t,y):
        u=y[::2];v=y[1::2];J=np.zeros((n,n));idx=np.arange(cells);a=2*idx;b=a+1
        J[a,a]=2*u*v-4-2*c;J[a,b]=u*u;J[b,a]=3-2*u*v;J[b,b]=-u*u-2*c
        J[a[1:],a[:-1]]=c;J[b[1:],b[:-1]]=c;J[a[:-1],a[1:]]=c;J[b[:-1],b[1:]]=c
        return J
    return Problem(f'brusselator-1d-{cells}',n,y0,10.,1.,f,j,2,2)

def packed(J,lband,uband):
    n=J.shape[0];k=np.zeros((lband+uband+1,n));
    for q in range(-lband,uband+1):
        # row r=j-q, offset index uband-q; only valid diagonals
        js=np.arange(max(0,q),min(n,n+q));is_=js-q
        if len(js):k[uband-q,js]=J[is_,js]
    return k

