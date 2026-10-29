"""Shared equations for the benchmark adapters; singular mass systems stay DAEs."""
from __future__ import annotations
import math


def rhs(case, t, y):
    m,p=case['model'],case['parameters']; y=list(y)
    if m in ('scalar_decay','sabr'): return [p.get('rate',p.get('lambda'))*y[0]]
    if m=='logistic': return [p['rate']*y[0]*(1-y[0]/p['capacity'])]
    if m=='kepler':
        x,z,vx,vz=y; r2=x*x+z*z; a=-p['mu']/(r2*math.sqrt(r2)); return [vx,vz,a*x,a*z]
    if m=='robertson' or m=='robertson_constraint':
        a,b,c=y; u=p['k1']*a; v=p['k2']*b*c; w=p['k3']*b*b
        return [-u+v,u-v-w,w] if m=='robertson' else [-u+v,u-v-w,a+b+c-1]
    if m=='vanderpol': return [y[1],p['mu']*(1-y[0]*y[0])*y[1]-y[0]]
    if m=='hires':
        a,b,c,d,e,f,g,h=y; q=280*f*h
        return [-1.71*a+.43*b+8.32*c+.0007,1.71*a-8.75*b,-10.03*c+.43*d+.035*e,8.32*b+1.71*c-1.12*d,-1.745*e+.43*f+.43*g,-q+.69*d+1.71*e-.43*f+.69*g,q-1.81*g,-q+1.81*g]
    if m in ('pr_linear','pr_nonlinear'):
        d=y[0]-math.sin(t); return [p['lambda']*d+math.cos(t)+p['beta']*d*d]
    if m=='rotation': return [-p['omega']*y[1],p['omega']*y[0]]
    if m=='damping': return [y[1],-2*p['zeta']*p['omega']*y[1]-p['omega']**2*y[0]]
    if m=='diagonal_stiff': return [a*b for a,b in zip(p['rates'],y)]
    if m=='jordan': return [p['lambda']*y[0]+p['offdiag']*y[1],p['lambda']*y[1]]
    if m=='nonnormal': return [p['a']*y[0]+p['b']*y[1],p['d']*y[1]]
    if m in ('periodic_heat','advection_diffusion'):
        n=p['cells']; d=p['diffusion']*n*n
        return [d*(y[(i-1)%n]-2*y[i]+y[(i+1)%n])-(p.get('velocity',0)*n*(y[i]-y[(i-1)%n])) for i in range(n)]
    if m=='brusselator_1d':
        n=p['cells']; c=p['diffusion_factor']; out=[]
        for i in range(n):
            u,v=y[2*i:2*i+2]
            ul,vl=(y[2*i-2:2*i] if i else [p['boundary_u'],p['boundary_v']])
            ur,vr=(y[2*i+2:2*i+4] if i+1<n else [p['boundary_u'],p['boundary_v']])
            out.extend([1+u*u*v-4*u+c*(ul-2*u+ur),3*u-u*u*v+c*(vl-2*v+vr)])
        return out
    if m=='gray_scott_2d':
        n=p['grid']; n2=n*n; out=[0.0]*(2*n2)
        for k in range(n2):
            i,j=k%n,k//n
            neighbors=[((i-1)%n)+j*n,((i+1)%n)+j*n,i+((j+1)%n)*n,i+((j-1)%n)*n]
            u,v=y[k],y[n2+k]; uv2=u*v*v
            lu=(sum(y[q] for q in neighbors)-4*u)*n*n
            lv=(sum(y[n2+q] for q in neighbors)-4*v)*n*n
            out[k]=p['du']*lu-uv2+p['feed']*(1-u)
            out[n2+k]=p['dv']*lv+uv2-(p['feed']+p['kill'])*v
        return out
    if m=='mass_invertible': return [-2*y[0]-y[1],-3*y[1]]
    if m=='mass_ill_conditioned': return [-y[0],-p['epsilon']*y[1]]
    if m in ('dae_index1','dae_index1_inconsistent'):
        return [-y[0]+y[1],y[0]+y[1]-(2*math.sin(t)+math.cos(t))]
    if m=='pendulum_index3':
        x,z,vx,vz,lam=y; return [vx,vz,-lam*x,-p['g']-lam*z,x*x+z*z-p['length']**2]
    if m=='fourier_path':
        a=complex(y[0],y[1]); b=complex(y[2],y[3]); phase=complex(math.cos(p['omega']*t),math.sin(p['omega']*t))
        c=p['g']+p['eps']*(phase*a).real; q=p['sigma']*1.25
        da=1j*q*c*a.conjugate()*b; db=.5j*q*c*a*a
        return [da.real,da.imag,db.real,db.imag]
    raise ValueError('unknown model '+str(m))


def jac(case,t,y):
    """Analytic where simple; centered finite differences for the remaining nonlinear fixtures."""
    m,p=case['model'],case['parameters']
    if m in ('scalar_decay','sabr'): return [[p.get('rate',p.get('lambda'))]]
    if m=='logistic': return [[p['rate']*(1-2*y[0]/p['capacity'])]]
    if m in ('pr_linear','pr_nonlinear'): return [[p['lambda']+2*p['beta']*(y[0]-math.sin(t))]]
    if m=='rotation': return [[0,-p['omega']],[p['omega'],0]]
    if m=='damping': return [[0,1],[-p['omega']**2,-2*p['zeta']*p['omega']]]
    if m=='diagonal_stiff': return [[v if i==j else 0 for j in range(len(y))] for i,v in enumerate(p['rates'])]
    if m=='jordan': return [[p['lambda'],p['offdiag']],[0,p['lambda']]]
    if m=='nonnormal': return [[p['a'],p['b']],[0,p['d']]]
    if m=='robertson':
        a,b,c=y; k1,k2,k3=p['k1'],p['k2'],p['k3']
        return [[-k1,k2*c,k2*b],[k1,-k2*c-2*k3*b,-k2*b],[0,2*k3*b,0]]
    if m=='vanderpol': return [[0,1],[-2*p['mu']*y[0]*y[1]-1,p['mu']*(1-y[0]*y[0])]]
    if m=='kepler':
        x,z=y[:2]; mu=p['mu'];r2=x*x+z*z;r=math.sqrt(r2);r3=r2*r;r5=r3*r2
        return [[0,0,1,0],[0,0,0,1],
                [-mu/r3+3*mu*x*x/r5,3*mu*x*z/r5,0,0],
                [3*mu*x*z/r5,-mu/r3+3*mu*z*z/r5,0,0]]
    if m=='hires':
        a,b,c,d,e,f,g,h=y
        return [[-1.71,.43,8.32,0,0,0,0,0],
                [1.71,-8.75,0,0,0,0,0,0],
                [0,0,-10.03,.43,.035,0,0,0],
                [0,8.32,1.71,-1.12,0,0,0,0],
                [0,0,0,0,-1.745,.43,.43,0],
                [0,0,0,.69,1.71,-280*h-.43,.69,-280*f],
                [0,0,0,0,0,280*h,-1.81,280*f],
                [0,0,0,0,0,-280*h,1.81,-280*f]]
    if m=='robertson_constraint':
        a,b,c=y;k1,k2,k3=p['k1'],p['k2'],p['k3']
        return [[-k1,k2*c,k2*b],[k1,-k2*c-2*k3*b,-k2*b],[1,1,1]]
    if m=='mass_invertible': return [[-2,-1],[0,-3]]
    if m=='mass_ill_conditioned': return [[-1,0],[0,-p['epsilon']]]
    if m in ('dae_index1','dae_index1_inconsistent'): return [[-1,1],[1,1]]
    if m=='pendulum_index3':
        x,z,vx,vz,lam=y
        return [[0,0,1,0,0],[0,0,0,1,0],[-lam,0,0,0,-x],
                [0,-lam,0,0,-z],[2*x,2*z,0,0,0]]
    if m=='gray_scott_2d':
        n=p['grid'];n2=n*n;out=[[0.0]*(2*n2) for _ in range(2*n2)]
        for k in range(n2):
            i,j=k%n,k//n;u,v=y[k],y[n2+k]
            neighbors=[((i-1)%n)+j*n,((i+1)%n)+j*n,i+((j+1)%n)*n,i+((j-1)%n)*n]
            out[k][k]=-4*p['du']*n*n-v*v-p['feed']
            out[k][n2+k]=-2*u*v
            out[n2+k][k]=v*v
            out[n2+k][n2+k]=-4*p['dv']*n*n+2*u*v-(p['feed']+p['kill'])
            for nb in neighbors:
                out[k][nb]+=p['du']*n*n
                out[n2+k][n2+nb]+=p['dv']*n*n
        return out
    if m in ('periodic_heat','advection_diffusion'):
        n=p['cells']; D=p['diffusion']*n*n; v=p.get('velocity',0)*n
        out=[[0.0]*n for _ in range(n)]
        for i in range(n):
            out[i][i]=-2*D-v
            out[i][(i-1)%n]+=D+v
            out[i][(i+1)%n]+=D
        return out
    if m=='brusselator_1d':
        n=p['cells'];D=p['diffusion_factor'];out=[[0.0]*(2*n) for _ in range(2*n)]
        for i in range(n):
            u,v=y[2*i:2*i+2];a,b=2*i,2*i+1
            out[a][a]=2*u*v-4-2*D;out[a][b]=u*u
            out[b][a]=3-2*u*v;out[b][b]=-u*u-2*D
            if i>0: out[a][a-2]=D;out[b][b-2]=D
            if i+1<n: out[a][a+2]=D;out[b][b+2]=D
        return out
    if m=='fourier_path':
        a=complex(y[0],y[1]); b=complex(y[2],y[3]); phase=complex(math.cos(p['omega']*t),math.sin(p['omega']*t))
        c=p['g']+p['eps']*(phase*a).real; q=p['sigma']*1.25
        cols=[]
        for k in range(4):
            da=complex(int(k==0),int(k==1)); db=complex(int(k==2),int(k==3))
            dc=p['eps']*(phase*da).real
            fa=1j*q*(dc*a.conjugate()*b+c*da.conjugate()*b+c*a.conjugate()*db)
            fb=.5j*q*(dc*a*a+2*c*a*da)
            cols.append([fa.real,fa.imag,fb.real,fb.imag])
        return [list(row) for row in zip(*cols)]
    raise ValueError('unknown model '+str(m))


def jvp(case,t,y,v):
    if case['model'] in ('periodic_heat','advection_diffusion'):
        return rhs(case,t,v)
    if case['model']=='brusselator_1d':
        p=case['parameters'];n=p['cells'];D=p['diffusion_factor'];out=[]
        for i in range(n):
            u,w=y[2*i:2*i+2];du,dw=v[2*i:2*i+2]
            dul,dwl=(v[2*i-2:2*i] if i else [0.0,0.0])
            dur,dwr=(v[2*i+2:2*i+4] if i+1<n else [0.0,0.0])
            out.extend([(2*u*w-4)*du+u*u*dw+D*(dul-2*du+dur),
                        (3-2*u*w)*du-u*u*dw+D*(dwl-2*dw+dwr)])
        return out
    if case['model']=='gray_scott_2d':
        p=case['parameters'];n=p['grid'];n2=n*n;out=[0.0]*(2*n2)
        for k in range(n2):
            i,j=k%n,k//n; neighbors=[((i-1)%n)+j*n,((i+1)%n)+j*n,i+((j+1)%n)*n,i+((j-1)%n)*n]
            u,w=y[k],y[n2+k];du,dw=v[k],v[n2+k]
            lu=(sum(v[q] for q in neighbors)-4*du)*n*n
            lw=(sum(v[n2+q] for q in neighbors)-4*dw)*n*n
            out[k]=p['du']*lu-(w*w+p['feed'])*du-2*u*w*dw
            out[n2+k]=p['dv']*lw+w*w*du+(2*u*w-p['feed']-p['kill'])*dw
        return out
    j=jac(case,t,y)
    if len(v)!=len(j): raise ValueError('JVP dimension mismatch')
    return [sum(a*b for a,b in zip(row,v)) for row in j]


def dae_residual(case,t,y,yp):
    m=case['mass_matrix']
    if m is None: raise ValueError('ordinary ODE has no DAE residual')
    f=rhs(case,t,y)
    return [sum(a*b for a,b in zip(row,yp))-fi for row,fi in zip(m,f)]
