#!/usr/bin/env python3
"""Prospectively preregistered BASS21 source-owned linear polarized Thomson response.

Scientific boundary: frozen original 50 BASS21 62-state unpolarized histories,
normal-comoving cold electrons; solves a *coupled second-kind Volterra equation*
for deltaI and Q, without new IVPs. Polarization modifies intensity quadrupole
and the resulting perturbation modifies polarization. No matter/geometry/spectral
backreaction. Local Stokes Q is meridional minus azimuthal intensity.
"""
from __future__ import annotations
import argparse, csv, hashlib, json, pathlib, sys, time
import numpy as np
from numpy.polynomial.legendre import leggauss
from scipy.integrate import simpson

# Byte-exact frozen PR130 source reader, supplied in the new source-inclusive ZIP.
HERE=pathlib.Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'source_code'))
from lrs_polarization_replay import load_owner_archive, integrate_los, brightness, MU, sha256
SOURCE_ARCHIVE_SHA='6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8'
PARENT_ARCHIVE_SHA='d6aaaf75aa82f083eddd577f9d0f44d54fe5fdfdd2978fe4882a6d986edc3e2d'


def gl_half(n):
    z,w=leggauss(n)
    return (z+1)*.5,w*.5


def sampled_simpson_weights(x):
    """Same composite uneven-grid scipy Simpson as original PR130 including j=1 trap."""
    x=np.asarray(x,dtype=float);n=len(x)
    if n<2 or np.any(np.diff(x)<=0): raise ValueError('INVALID_SAVED_EPOCHS')
    out=np.zeros((n,n))
    for j in range(1,n):
        if j==1: out[j,:2]=np.diff(x[:2])[0]/2
        else:out[j,:j+1]=simpson(np.eye(j+1),x=x[:j+1],axis=0)
    return out


class FrozenTransfer:
    def __init__(self,x,b,g,tau,mu):
        x=np.asarray(x);b=np.asarray(b);g=np.asarray(g);tau=np.asarray(tau);mu=np.asarray(mu)
        n=len(x)
        for a in (b,g,tau):
            if a.shape!=(n,):raise ValueError('FROZEN_FIELD_SHAPE')
        if not (np.isfinite(x).all() and np.isfinite(b).all() and np.isfinite(g).all() and np.isfinite(tau).all() and np.min(g)>=0 and np.min(np.diff(tau))>=-1e-12):raise ValueError('FROZEN_CLOCK_INVALID')
        if np.min(mu)<0 or np.max(mu)>1:raise ValueError('INVALID_HALF_SKY')
        W=sampled_simpson_weights(x)
        delta=b[None,:,None]-b[:,None,None]
        mu3=mu[None,None,:]
        ep=np.exp(2*delta); em=np.exp(-4*delta)
        d2=(1-mu3**2)*ep+mu3**2*em
        if np.min(d2)<=0:raise ValueError('NONPOSITIVE_NULL_ENERGY')
        muin=mu3*np.exp(-2*delta)/np.sqrt(d2)
        p2=.5*(3*muin**2-1)
        source_w=W[:,:,None]*g[None,:,None]*np.exp(-(tau[:,None,None]-tau[None,:,None]))/d2**2
        # Same exact-polynomial pole enforcement as PR130, with D^-4 bolometric transfer.
        polar_factor=(1-mu3**2)*ep/d2
        self.K0=source_w
        self.K2=source_w*p2
        self.KQ=source_w*polar_factor
        self.mu=mu

    def apply(self,j2_orig,j0,j2,bq):
        si2=.5*j2-.375*bq
        sq=-.75*(j2_orig+j2)+.5625*bq
        deltaI=np.einsum('jkm,k->jm',self.K0,j0,optimize=True)+np.einsum('jkm,k->jm',self.K2,si2,optimize=True)
        Q=np.einsum('jkm,k->jm',self.KQ,sq,optimize=True)
        return deltaI,Q


def observed_moments(deltaI,Q,mu,weights):
    p2=.5*(3*mu**2-1)
    j0=deltaI@weights
    j2=(deltaI*p2)@weights
    bq=2*(Q*(1-mu**2))@weights
    return j0,j2,bq


def solve_case(h, nangular=24, mu_out=MU, epochs=None, abs_tol=1e-12, max_picard=24):
    """Fixed point on saved x, independent angular GL; returns full normalized sky rows.

    Use Jacobi/Picard of the complete two-field collision operator, not just Q-alone.
    A residual is measured by a fresh operator application on converged fields.
    """
    ix=np.arange(len(h['x'])) if epochs is None else np.asarray(epochs)
    if not np.all(np.diff(ix)>0):raise ValueError('INVALID_SUBGRID')
    x=h['x'][ix];b=h['b'][ix];g=h['g'][ix];tau=h['tau'][ix]
    moments=h['photon_energy_moments'][ix];j2orig=moments[:,2]
    mu,weights=gl_half(nangular)
    transfer=FrozenTransfer(x,b,g,tau,mu)
    j0=np.zeros_like(x);j2=np.zeros_like(x);bq=np.zeros_like(x)
    diffs=[]
    for step in range(max_picard):
        di,q=transfer.apply(j2orig,j0,j2,bq)
        nxt=observed_moments(di,q,mu,weights)
        diff=max(np.max(np.abs(a-b)) for a,b in zip(nxt,(j0,j2,bq)))
        diffs.append(float(diff));j0,j2,bq=nxt
        if diff<abs_tol:break
    if diffs[-1]>=abs_tol:raise RuntimeError('POLARIZED_VOLterra_NONCONVERGENCE')
    checkI,checkQ=transfer.apply(j2orig,j0,j2,bq)
    check_m=observed_moments(checkI,checkQ,mu,weights)
    fixed_res=max(np.max(np.abs(a-b)) for a,b in zip(check_m,(j0,j2,bq)))
    if fixed_res>=abs_tol:raise RuntimeError('POLARIZED_FIXED_POINT_RESIDUAL')
    export=FrozenTransfer(x,b,g,tau,np.asarray(mu_out))
    deltaI,Q=export.apply(j2orig,j0,j2,bq)
    baseI=brightness(moments,np.asarray(mu_out))
    fullI=baseI+deltaI
    if np.min(fullI)<=0:raise ValueError('NEGATIVE_TOTAL_INTENSITY')
    if np.any(np.abs(Q)>fullI):raise ValueError('POLARIZATION_EXCEEDS_INTENSITY')
    qfirst=export.apply(j2orig,np.zeros_like(j0),np.zeros_like(j2),np.zeros_like(bq))[1]
    qparent=integrate_los(g,j2orig,b,tau,x,np.asarray(mu_out))
    first_err=float(np.max(np.abs(qfirst-qparent)))
    if first_err>1e-12:raise RuntimeError('PARENT_FIRST_GAIN_NOT_REPRODUCED')
    if np.any(Q[:,np.asarray(mu_out)==1]!=0) or np.any(Q[0]!=0):raise RuntimeError('POLE_OR_INITIAL_Q_NONZERO')
    return dict(x=x,mu=np.asarray(mu_out),deltaI=deltaI,Q=Q,fullI=fullI,baseI=baseI,
                first_gain=qfirst,j0=j0,j2=j2,B=bq,
                steps=len(diffs),diffs=diffs,fixed_res=float(fixed_res),first_err=first_err)


def write_csv(path,rows):
    with open(path,'w',newline='') as f:
        wr=csv.DictWriter(f,fieldnames=list(rows[0]),lineterminator='\n');wr.writeheader();wr.writerows(rows)


def run(source,out, nangular=24, refine_nangular=48):
    out=pathlib.Path(out);out.mkdir(parents=True,exist_ok=True)
    a=load_owner_archive(source);cases=sorted(a);npairs=25
    q=np.zeros((50,193,17));di=np.zeros_like(q);full=np.zeros_like(q);fg=np.zeros_like(q)
    q_angular_gap=[];di_angular_gap=[];records=[];case_results={};q_residual=[]
    for k,name in enumerate(cases):
        h=a[name];r=solve_case(h,nangular=nangular)
        r2=solve_case(h,nangular=refine_nangular)
        q[k]=r['Q'];di[k]=r['deltaI'];full[k]=r['fullI'];fg[k]=r['first_gain']
        dqref=float(np.max(np.abs(r['Q']-r2['Q'])))
        diref=float(np.max(np.abs(r['deltaI']-r2['deltaI'])))
        q_angular_gap.append(dqref);di_angular_gap.append(diref)
        case_results[name]=(r['Q'][-1,0],r['first_gain'][-1,0],r['fullI'][-1,0])
        q_residual.append(r['fixed_res'])
        records.append(dict(case=name,Q_equator_final=float(r['Q'][-1,0]),
          Q_first_gain_equator_final=float(r['first_gain'][-1,0]),
          feedback_increment_final=float(r['Q'][-1,0]-r['first_gain'][-1,0]),
          Q_over_I_final=float(r['Q'][-1,0]/r['fullI'][-1,0]),
          max_abs_Q_over_fullI=float(np.max(np.abs(r['Q']/r['fullI']))),
          deltaI_equator_final=float(r['deltaI'][-1,0]),
          max_deltaI_over_baseI=float(np.max(np.abs(r['deltaI']/r['baseI']))),
          angular_Q_24_48_max_abs=dqref,angular_deltaI_24_48_max_abs=diref,
          picard_iterations=int(r['steps']),operator_residual=r['fixed_res'],
          first_gain_reconstruction_error=r['first_err'],
          source_sha256=h['source_sha256']))
    paired_q=np.zeros((25,193,17)); paired_delta_increment=np.zeros_like(paired_q)
    pair_rows=[]
    for ip,m in enumerate(range(5)):
      for r in range(5):
        pidx=5*m+r
        ia=cases.index(f's00_m{m:02d}_r{r:02d}');ib=cases.index(f's04_m{m:02d}_r{r:02d}')
        paired_q[pidx]=q[ib]-q[ia]
        paired_delta_increment[pidx]=(q[ib]-fg[ib])-(q[ia]-fg[ia])
        for j,mu in enumerate(MU):
          pair_rows.append(dict(pair=f'm{m:02d}_r{r:02d}',mu=float(mu),
           Q_full_B_final=float(q[ib,-1,j]),Q_full_A_final=float(q[ia,-1,j]),
           delta_Q_full_final=float(paired_q[pidx,-1,j]),
           delta_Q_first_gain_final=float(fg[ib,-1,j]-fg[ia,-1,j]),
           delta_Q_feedback_final=float(paired_delta_increment[pidx,-1,j]),
           delta_Q_over_I_full_final=float(q[ib,-1,j]/full[ib,-1,j]-q[ia,-1,j]/full[ia,-1,j])))
    # Original time grid is not uniform in x. Subsampled-grid checks use stored source ONLY.
    sample_names=['s00_m00_r00','s04_m00_r00','s04_m02_r02','s04_m04_r04']
    tchecks=[]
    for name in sample_names:
        h=a[name];ix=cases.index(name)
        for stride in (2,4):
            r=solve_case(h,nangular=nangular,epochs=np.arange(0,193,stride))
            gap=float(np.max(np.abs(r['Q']-q[ix,::stride,:])))
            tchecks.append(dict(case=name,sampling=f'193_to_{len(r["x"])}',max_Q_abs_difference=gap))
    np.savez_compressed(out/'LRS50_CPLD_193x17_POLARIZED_RESPONSE.npz',
      cases=np.array(cases),x=a[cases[0]]['x'],mu=MU.copy(),Q_coupled=q,
      deltaI=di,I_full=full,Q_first_gain=fg,paired_deltaQ=paired_q,
      paired_feedback_deltaQ=paired_delta_increment,pairs=np.array([f'm{m:02d}_r{r:02d}' for m in range(5) for r in range(5)]))
    write_csv(out/'CASE50_ENDPOINT_CPLD.csv',records)
    write_csv(out/'PAIR25_ENDPOINT_CPLD.csv',pair_rows)
    write_csv(out/'SAVED_GRID_REFINEMENT.csv',tchecks)
    summary={
      'schema':'vigilode.bass21.lrs.coupled_thomson_frozen_response.v1',
      'scope':'first fully coupled linear deltaI,Q radiative response with original unpolarized BASS21 I0 frozen; no chemistry/geometry or spectral feedback',
      'source_archive_sha256':SOURCE_ARCHIVE_SHA,
      'original_cases':50,'saved_epochs':193,'directions_half_sky':17,
      'coupled_Q_rows':int(q.size),'deltaI_rows':int(di.size),'paired_deltaQ_rows':int(paired_q.size),
      'new_ivps':0,'angular_gl_primary':nangular,'angular_gl_check':refine_nangular,
      'max_absolute_fixed_operator_residual':float(max(q_residual)),
      'max_first_gain_reconstruction_error':float(max(r['first_gain_reconstruction_error'] for r in records)),
      'max_angular_gl_Q_difference':float(max(q_angular_gap)),
      'max_angular_gl_deltaI_difference':float(max(di_angular_gap)),
      'max_time_sample_Q_difference':float(max(r['max_Q_abs_difference'] for r in tchecks)),
      'mean_equatorial_paired_deltaQ_full':float(np.mean(paired_q[:,-1,0])),
      'mean_equatorial_paired_deltaQ_first_gain':float(np.mean([(fg[cases.index(f's04_m{m:02d}_r{rr:02d}'),-1,0]-fg[cases.index(f's00_m{m:02d}_r{rr:02d}'),-1,0]) for m in range(5) for rr in range(5)])),
      'mean_equatorial_paired_feedback_deltaQ':float(np.mean(paired_delta_increment[:,-1,0])),
      'max_abs_Q_full_over_I_full':float(np.max(np.abs(q/full))),
      'max_abs_deltaI_over_I_base':float(np.max(np.abs(di/(full-di)))),
      'min_full_I':float(np.min(full)),
      'max_picard_steps':int(max(r['picard_iterations'] for r in records)),
      'time_subsample_comparisons':tchecks,
      'DAG':{'vigilode':'7/23','rei_central':'28/88','rei_working':'33/88','bass_ce1':'9/15','successor':'0/12','official_delta':0,'new_scoped_exploratory':'1/1 on independent proof and results after verification'},
      'production':'HOLD',
      'conventions':'metric (-,+,+,+); c and sigma_T embedded in original g=c*sigma_T*n_e/H; Q=meridional minus azimuthal',
      'limits':'finite z2, frozen unpolarized L8 BASS background, no spectral/thermal, no real CMB E/B, no BDF global bound, no independent-author R6/C9',
      'preregistered_before_run':True,
    }
    (out/'SCIENCE_SUMMARY.json').write_text(json.dumps(summary,sort_keys=True,indent=2,allow_nan=False)+'\n')
    return summary


def main():
    p=argparse.ArgumentParser();p.add_argument('--archive',required=True);p.add_argument('--out',required=True)
    p.add_argument('--nangular',type=int,default=24);p.add_argument('--refine',type=int,default=48)
    args=p.parse_args();print(json.dumps(run(args.archive,args.out,args.nangular,args.refine),indent=2))

if __name__=='__main__':main()
