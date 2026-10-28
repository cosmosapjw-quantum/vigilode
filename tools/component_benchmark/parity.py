"""Independent 80/120-digit equation probes under the Astra planning addendum.

This is a diagnostic equality screen. It never creates an ODE certificate.
Sparse forward derivatives keep the 4096-dimensional linear probes O(N).
"""
from __future__ import annotations

import json
import math
from pathlib import Path

import mpmath as mp

from .protocol import digest


class D:
    def __init__(self, value, grad=None):
        self.x = mp.mpf(value)
        self.g = grad or {}

    @staticmethod
    def cast(x):
        return x if isinstance(x, D) else D(x)

    def __add__(self, other):
        other = D.cast(other)
        return D(self.x+other.x, {k:self.g.get(k,0)+other.g.get(k,0) for k in self.g.keys() | other.g.keys()})
    __radd__ = __add__
    def __neg__(self):
        return D(-self.x, {k:-v for k,v in self.g.items()})
    def __sub__(self, other):
        return self+-D.cast(other)
    def __rsub__(self, other):
        return D.cast(other)+-self
    def __mul__(self, other):
        other=D.cast(other)
        return D(self.x*other.x, {k:self.g.get(k,0)*other.x+other.g.get(k,0)*self.x for k in self.g.keys() | other.g.keys()})
    __rmul__=__mul__
    def __truediv__(self, other):
        other=D.cast(other)
        return D(self.x/other.x,{k:(self.g.get(k,0)*other.x-other.g.get(k,0)*self.x)/(other.x*other.x) for k in self.g.keys() | other.g.keys()})
    def __rtruediv__(self, other):
        return D.cast(other)/self
    def __pow__(self, p):
        p=mp.mpf(p)
        return D(self.x**p,{k:p*self.x**(p-1)*v for k,v in self.g.items()})


def equation(case, t, state):
    """Independent transcription of the declared equations, with sparse AD."""
    m=case['model']; n=len(state)
    y=[D(v,{i:mp.mpf(1)}) for i,v in enumerate(state)]
    p={k:([mp.mpf(v) for v in x] if isinstance(x,list) else mp.mpf(x) if isinstance(x,(int,float)) else x) for k,x in case['parameters'].items()}
    q=lambda v: mp.mpf(v)
    t=q(t)
    if m in ('scalar_decay','sabr'):return [p.get('rate',p.get('lambda'))*y[0]]
    if m=='logistic':return [p['rate']*y[0]-(p['rate']/p['capacity'])*y[0]*y[0]]
    if m=='kepler':
        r3=(y[0]*y[0]+y[1]*y[1])**q(1.5)
        return [y[2],y[3],-p['mu']*y[0]/r3,-p['mu']*y[1]/r3]
    if m in ('robertson','robertson_constraint'):
        loss=p['k1']*y[0]; exchange=p['k2']*y[1]*y[2]; reaction=p['k3']*y[1]*y[1]
        return [exchange-loss,loss-exchange-reaction,reaction if m=='robertson' else sum(y)-1]
    if m=='vanderpol':return [y[1],p['mu']*(1-y[0]*y[0])*y[1]-y[0]]
    if m=='hires':
        a,b,c,d,e,f,g,h=y; interaction=280*f*h
        return [-q(1.71)*a+q(.43)*b+q(8.32)*c+q(.0007),q(1.71)*a-q(8.75)*b,
                -q(10.03)*c+q(.43)*d+q(.035)*e,q(8.32)*b+q(1.71)*c-q(1.12)*d,
                -q(1.745)*e+q(.43)*(f+g),-interaction+q(.69)*(d+g)+q(1.71)*e-q(.43)*f,
                interaction-q(1.81)*g,-interaction+q(1.81)*g]
    if m in ('pr_linear','pr_nonlinear'):
        delta=y[0]-mp.sin(t)
        return [p['lambda']*delta+mp.cos(t)+p['beta']*delta*delta]
    if m=='rotation':return [-p['omega']*y[1],p['omega']*y[0]]
    if m=='damping':return [y[1],-(p['omega']*p['omega']*y[0]+2*p['zeta']*p['omega']*y[1])]
    if m=='diagonal_stiff':return [a*b for a,b in zip(p['rates'],y)]
    if m=='jordan':return [p['lambda']*y[0]+p['offdiag']*y[1],p['lambda']*y[1]]
    if m=='nonnormal':return [p['a']*y[0]+p['b']*y[1],p['d']*y[1]]
    if m in ('periodic_heat','advection_diffusion'):
        diffusion=p['diffusion']*n*n; transport=p.get('velocity',0)*n
        return [diffusion*(y[(i+1)%n]-2*y[i]+y[(i-1)%n])+transport*(y[(i-1)%n]-y[i]) for i in range(n)]
    if m=='brusselator_1d':
        cells=int(p['cells']); out=[]; k=p['diffusion_factor']
        for i in range(cells):
            u,v=y[2*i:2*i+2]
            left=y[2*i-2:2*i] if i else (p['boundary_u'],p['boundary_v'])
            right=y[2*i+2:2*i+4] if i+1<cells else (p['boundary_u'],p['boundary_v'])
            reaction=u*u*v
            out.extend([1+reaction-4*u+k*(left[0]-2*u+right[0]),3*u-reaction+k*(left[1]-2*v+right[1])])
        return out
    if m=='gray_scott_2d':
        side=int(p['grid']); n2=side*side; out=[None]*n
        for row in range(side):
            for col in range(side):
                k=row*side+col; u,v=y[k],y[k+n2]
                adjacent=[row*side+(col+1)%side,row*side+(col-1)%side,((row+1)%side)*side+col,((row-1)%side)*side+col]
                lapu=sum(y[j]-u for j in adjacent)*n2
                lapv=sum(y[j+n2]-v for j in adjacent)*n2
                conversion=u*v*v
                out[k]=p['du']*lapu-conversion+p['feed']*(1-u)
                out[k+n2]=p['dv']*lapv+conversion-(p['feed']+p['kill'])*v
        return out
    if m=='mass_invertible':return [-2*y[0]-y[1],-3*y[1]]
    if m=='mass_ill_conditioned':return [-y[0],-p['epsilon']*y[1]]
    if m in ('dae_index1','dae_index1_inconsistent'):return [y[1]-y[0],y[0]+y[1]-2*mp.sin(t)-mp.cos(t)]
    if m=='pendulum_index3':return [y[2],y[3],-y[4]*y[0],-p['g']-y[4]*y[1],y[0]*y[0]+y[1]*y[1]-p['length']**2]
    if m=='fourier_path':
        x,z,u,v=y; c=p['g']+p['eps']*(mp.cos(p['omega']*t)*x-mp.sin(p['omega']*t)*z)
        k=p['sigma']*q(1.25)*c
        return [-k*(x*v-z*u),k*(x*u+z*v),-k*x*z,q(.5)*k*(x*x-z*z)]
    raise ValueError('unimplemented independent oracle '+m)


def frozen_points(case):
    t0,t1=case['t_span'];n=len(case['initial_state'])
    v=[.5+.01*i for i in range(n)]
    return [{'time':t0,'state':case['initial_state'],'direction':v},
            {'time':t0+.37*(t1-t0),'state':[x+.01*(i+1) for i,x in enumerate(case['initial_state'])],'direction':v}]


def admit_probe(case_sha256, source_sha256, observed_source_sha256, request, output):
    if source_sha256 != observed_source_sha256:
        return {'status':'HOLD','reason':'SOURCE_IDENTITY_MISMATCH'}
    if digest(request.get('case')) != case_sha256:
        return {'status':'HOLD','reason':'CASE_EQUATION_GRID_IDENTITY_MISMATCH'}
    if request.get('probes') != frozen_points(request['case']):
        return {'status':'HOLD','reason':'FROZEN_PROBE_INPUT_MISMATCH'}
    try:
        return check_probe(request['case'],request['probes'],output)
    except (ArithmeticError,ValueError,TypeError,KeyError,IndexError) as error:
        return {'status':'HOLD','reason':'PROBE_ORACLE_OR_PROTOCOL_FAILURE','detail':str(error)}


def _oracle(case, point):
    with mp.workdps(80):low=equation(case,point['time'],point['state'])
    with mp.workdps(120):high=equation(case,point['time'],point['state'])
    return low,high


def check_probe(case, points, output):
    """Apply the frozen diagnostic bounds, recording the first counterexample."""
    if output.get('status')!='OK' or output.get('case_id')!=case['case_id'] or len(output.get('probes',[]))!=len(points):
        return {'status':'HOLD','reason':'PROBE_IDENTITY_OR_CAPABILITY'}
    mode=output.get('derivative_mode','')
    fd='finite_difference' in mode
    if not fd and 'analytic' not in mode:
        return {'status':'HOLD','reason':'UNDECLARED_DERIVATIVE_MODE'}
    max_fraction=mp.mpf(0); entries=0
    def failure(reason, point, kind, index, got=None, expected=None, bound=None):
        return {'status':'HOLD','reason':reason,'point':point,'kind':kind,'index':index,
                'candidate':str(got),'oracle':str(expected),'allowance':str(bound),'derivative_mode':mode}
    with mp.workdps(120):
        eps=mp.mpf(2)**-52; floor=8*mp.mpf(2)**-1074
        for pi,(point,observed) in enumerate(zip(points,output['probes'])):
            if any(observed.get(k)!=point[k] for k in ('time','state','direction')):
                return failure('REPRESENTED_INPUT_MISMATCH',pi,'input',0)
            low,high=_oracle(case,point);n=len(high);v=[mp.mpf(x) for x in point['direction']]
            sr=[max(1,abs(a.x),sum(abs(z)*max(1,abs(mp.mpf(point['state'][j]))) for j,z in a.g.items())) for a in high]
            sj=[max(1,sum(abs(z) for z in a.g.values())) for a in high]
            sv=[max(1,sum(abs(z*v[j]) for j,z in a.g.items())) for a in high]
            def bound(delta,scale):
                if delta>mp.mpf('1e-40')*scale:raise ArithmeticError('HOLD_ORACLE_PRECISION')
                return 256*eps*scale+4*delta+floor
            def compare(got,want,allow,kind,index):
                nonlocal max_fraction,entries
                if not isinstance(got,(float,int)) or not math.isfinite(got):return failure('NONFINITE_PROBE',pi,kind,index,got,want,allow)
                error=abs(mp.mpf(got)-want);entries+=1;max_fraction=max(max_fraction,error/allow)
                if error>allow:return failure('NUMERICAL_PARITY_FAILURE',pi,kind,index,got,want,allow)
                return None
            if len(observed.get('rhs',[]))!=n or len(observed.get('jvp',[]))!=n:
                return failure('PROBE_DIMENSION_MISMATCH',pi,'vector',0)
            for i in range(n):
                bad=compare(observed['rhs'][i],high[i].x,bound(abs(high[i].x-low[i].x),sr[i]),'rhs',i)
                if bad:return bad
            columns=list(range(n)) if n<=512 else [0,n//2,n-1]
            if n>512 and observed.get('jacobian_column_indices')!=columns:
                return failure('PROBE_COLUMN_MISMATCH',pi,'jacobian',0)
            fd_allow={};fd_q={}
            if fd:
                metadata=observed.get('fd_columns',[])
                if len(metadata)!=len(columns):return failure('MISSING_FD_ENDPOINTS',pi,'jacobian',0)
                for j,meta in zip(columns,metadata):
                    if meta.get('index')!=j:return failure('FD_COLUMN_IDENTITY',pi,'jacobian',j)
                    yp=list(point['state']);ym=list(yp);yp[j]=meta['plus'];ym[j]=meta['minus']
                    denominator=mp.mpf(meta['denominator'])
                    if denominator<=0:return failure('INVALID_FD_DENOMINATOR',pi,'jacobian',j)
                    lp,fp=_oracle(case,{**point,'state':yp});lm,fm=_oracle(case,{**point,'state':ym})
                    fd_allow[j]=[];fd_q[j]=[]
                    for i in range(n):
                        q=(fp[i].x-fm[i].x)/denominator
                        def scale(a,state):return max(1,abs(a.x),sum(abs(g)*max(1,abs(mp.mpf(state[k]))) for k,g in a.g.items()))
                        r=(bound(abs(fp[i].x-lp[i].x),scale(fp[i],yp))+bound(abs(fm[i].x-lm[i].x),scale(fm[i],ym)))/denominator+16*eps*max(1,abs(q))
                        allowance=r+abs(q-high[i].g.get(j,0))
                        if allowance>mp.mpf('1e-5')*sj[i]:return failure('HOLD_FD_RESOLUTION',pi,'jacobian',[i,j],None,None,allowance)
                        fd_allow[j].append(allowance);fd_q[j].append(q)
            for ji,j in enumerate(columns):
                for i in range(n):
                    want=high[i].g.get(j,0)
                    allow=fd_allow[j][i] if fd else bound(abs(want-low[i].g.get(j,0)),sj[i])
                    try:got=observed['jacobian'][i][j] if n<=512 else observed['jacobian_columns'][ji][i]
                    except (KeyError,IndexError,TypeError):return failure('JACOBIAN_SHAPE',pi,'jacobian',[i,j])
                    bad=compare(got,want,allow,'jacobian',[i,j])
                    if bad:return bad
            directional=None
            if fd and n>512:
                meta=observed.get('fd_directional')
                if not meta:return failure('MISSING_FD_ENDPOINTS',pi,'jvp',0)
                yp,ym=meta['plus_state'],meta['minus_state'];den=mp.mpf(meta['denominator'])
                if len(yp)!=n or len(ym)!=n or den<=0:return failure('INVALID_FD_ENDPOINTS',pi,'jvp',0)
                lp,fp=_oracle(case,{**point,'state':yp});lm,fm=_oracle(case,{**point,'state':ym});directional=(yp,ym,den,fp,fm,lp,lm)
            for i in range(n):
                want=sum(g*v[j] for j,g in high[i].g.items())
                if fd and n<=512:
                    allow=sum(fd_allow[j][i]*abs(v[j]) for j in columns)+16*eps*sum(abs(fd_q[j][i]*v[j]) for j in columns)
                elif fd:
                    yp,ym,den,fp,fm,lp,lm=directional;q=(fp[i].x-fm[i].x)/den
                    def s(a,y):return max(1,abs(a.x),sum(abs(g)*max(1,abs(mp.mpf(y[k]))) for k,g in a.g.items()))
                    allow=(bound(abs(fp[i].x-lp[i].x),s(fp[i],yp))+bound(abs(fm[i].x-lm[i].x),s(fm[i],ym)))/den+16*eps*max(1,abs(q))+abs(q-want)
                else:
                    lowv=sum(g*v[j] for j,g in low[i].g.items());allow=bound(abs(want-lowv),sv[i])
                if fd and allow>mp.mpf('1e-5')*sv[i]:return failure('HOLD_FD_RESOLUTION',pi,'jvp',i,None,None,allow)
                bad=compare(observed['jvp'][i],want,allow,'jvp',i)
                if bad:return bad
    return {'status':'PASS_DIAGNOSTIC','entries':entries,'max_allowance_fraction':float(max_fraction),
            'derivative_mode':mode,'case_sha256':digest(case),'probes_sha256':digest(points),'oracle':'independent_sparse_AD_80_120dps',
            'verified_enclosure':False,'equation_equivalence_everywhere':False}
