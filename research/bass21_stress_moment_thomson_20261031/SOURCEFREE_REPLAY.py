"""Standalone exploratory Bianchi I unpolarized bolometric Thomson gain.
Physical status source-free, NOT original owner IVP or generic polarized collision.
Signature (-,+,+,+), beta=v/c, x=ln(a/a_emit), sum s_i=0.
Requires numpy>=2; execute python SOURCEFREE_REPLAY.py.
"""
import csv,json
from pathlib import Path
import numpy as np
from numpy.polynomial.legendre import leggauss
RDIR=Path(__file__).resolve().parent/'generated_sourcefree'
MODELS={'FLRW':(0.,0.,0.),'LRS':(-.035,-.035,.07),
        'nonLRS_A':(.08,-.03,-.05),'nonLRS_B':(.11,-.075,-.035)}
EPOCHS=(.04,.12,.25)
BETA={'rest':(0.,0.,0.),'z005':(0.,0.,.05),'tilt':(.04,-.025,.03)}
DIRECTIONS=[(1,0,0),(0,1,0),(0,0,1),(.6,.8,0),
            (.3,.4,.8660254037844386),(-.3,.7,.65),
            (.35,-.56,.75),(-.58,-.42,.70)]
def unit(a):
    a=np.asarray(a,dtype=float)
    return a/np.linalg.norm(a,axis=-1,keepdims=True)
def sphere(nmu,nphi):
    mu,w=leggauss(nmu); phi=2*np.pi*np.arange(nphi)/nphi
    st=np.sqrt(1-mu**2)
    n=np.stack([np.broadcast_to(st[:,None]*np.cos(phi),(nmu,nphi)),
                np.broadcast_to(st[:,None]*np.sin(phi),(nmu,nphi)),
                np.broadcast_to(mu[:,None],(nmu,nphi))],axis=-1).reshape(-1,3)
    return n,np.repeat(w,nphi)*2*np.pi/nphi
def brightness(n,s,x):
    s=np.asarray(s,dtype=float)
    assert len(s)==3 and abs(sum(s))<1e-14
    return np.sum(unit(n)**2*np.exp(2*x*s),axis=-1)**-2
def moments(n,w,I):
    ell=np.concatenate((np.ones((len(n),1)),n),axis=1)
    ext=np.longdouble
    return np.asarray(np.einsum('n,ni,nj->ij',(w*I).astype(ext),
                     ell.astype(ext),ell.astype(ext),optimize=True),dtype=float)
def lorentz(b):
    b=np.asarray(b,dtype=float);sq=float(b@b)
    if b.shape!=(3,) or not np.isfinite(b).all() or sq>=1:raise ValueError('beta')
    g=(1-sq)**-.5; L=np.eye(4);L[0,0]=g;L[0,1:]=-g*b;L[1:,0]=-g*b
    if sq:L[1:,1:]+=(g-1)/sq*np.outer(b,b)
    return L
def aberration(n,b):
    b=np.asarray(b,dtype=float);g=lorentz(b)[0,0];sq=float(b@b)
    dot=np.einsum('ni,i->n',n,b);D=g*(1-dot)
    if not sq:return n.copy(),D
    return (n+((g-1)*dot/sq-g)[:,None]*b)/D[:,None],D
def direct_moments(n,w,I,b):
    nr,D=aberration(n,b)
    return moments(nr,w*D**2,I)
def screen(out):
    out=unit(out)
    seed=np.array([0.,0.,1.]) if abs(out[2])<.92 else np.array([1.,0.,0.])
    e1=unit(seed-(seed@out)*out)
    return e1,np.cross(out,e1)
def gain(R,out):
    e1,e2=screen(out);E=np.array((e1,e2))
    G=3/(16*np.pi)*(R[0,0]*np.eye(2)-E@R[1:,1:]@E.T)
    return float(G.trace()),float(G[0,0]-G[1,1]),float(2*G[0,1]),float(np.linalg.eigvalsh(G)[0])
def generate(nmu=64,nphi=128):
    n,w=sphere(nmu,nphi); rows=[]; moments_check=[]
    for model,s in MODELS.items():
      for x in EPOCHS:
        I=brightness(n,s,x);R=moments(n,w,I)
        for vel,b in BETA.items():
          L=lorentz(b); Rp=L@R@L.T
          Rd=direct_moments(n,w,I,b)
          moments_check.append(dict(model=model,x=x,velocity=vel,
                              max_abs_diff=float(np.max(np.abs(Rp-Rd))),
                              null_trace=float(abs(R[0,0]-np.trace(R[1:,1:])))))
          for k,out in enumerate(DIRECTIONS):
            Ii,Q,U,gmin=gain(Rp,out)
            if gmin<-1e-12:raise AssertionError('coherency not PSD')
            rows.append(dict(model=model,x=x,velocity=vel,
                        direction_index=k,Q_gain=Q,U_gain=U,I_gain=Ii,
                        min_eigen_G=gmin))
    assert len(rows)==288 and len(moments_check)==36
    return rows,moments_check
if __name__=='__main__':
    rows,chk=generate();RDIR.mkdir(exist_ok=True)
    for name,data in [('local_thomson_gain_288.csv',rows),
                      ('moment_comparison_36.csv',chk)]:
        with (RDIR/name).open('w',newline='') as f:
            wr=csv.DictWriter(f,fieldnames=list(data[0]));wr.writeheader();wr.writerows(data)
    m=max(x['max_abs_diff'] for x in chk)
    assert m < 1e-12
    print(json.dumps({'status':'SOURCEFREE_NOT_OWNER','gain_rows':len(rows),
          'direct_comparisons':len(chk),'maximum_moment_residual':m,
          'new_owner_ivps':0,'VIGILODE_DAG':'7/23'},indent=2))
