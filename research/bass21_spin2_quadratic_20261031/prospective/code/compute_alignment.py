#!/usr/bin/env python3
"""Source-owned prospective LRS spin-2 polarization angular-shape alignment.

New experiment registered by Git commit 553ab8afd7f2d699ef0b2b172375860bdbbfefa1 BEFORE execution.
Original 231600-value quadratic work atlas was POST-HOC; do not retroactively preregister it.
No new owner IVPs; frozen original 50x193 physical histories only.
"""
from __future__ import annotations
import sys, pathlib, hashlib, json, csv, zipfile, io, argparse, math
import numpy as np
from scipy.special import lpmv
from numpy.polynomial.legendre import leggauss, Legendre

PREREG_COMMIT="553ab8afd7f2d699ef0b2b172375860bdbbfefa1"
ORIGINAL_ARCHIVE_SHA="e7e57f937eea61a225b47de91aab7fc79cb394c3b287dc02529decb829573383"
BASELINE_PR148_SHA="626ea52c9fae38db23e0dd9e5f628ed0cba35ee05d54391b02b91ca34a0b3b1c"
CUT=(16,18,20,22)
LL=np.arange(2,25,2,dtype=int)

def sha(path):
    with open(path,"rb") as f:return hashlib.file_digest(f,"sha256").hexdigest()

def extract_parent(source_zip):
    source_zip=pathlib.Path(source_zip)
    if sha(source_zip)!=ORIGINAL_ARCHIVE_SHA: raise ValueError("SOURCE_ZIP_SHA_MISMATCH")
    with zipfile.ZipFile(source_zip) as z:
        if z.testzip() is not None: raise ValueError("SOURCE_ZIP_CRC_MISMATCH")
        with np.load(io.BytesIO(z.read("results/SOURCE_FROZEN_SPIN2_POWER_50x193.npz")),allow_pickle=False) as p:
            state={k:p[k].copy() for k in p.files}
        raw=z.read("source/PR148_SOURCE_FULL.zip")
    if hashlib.sha256(raw).hexdigest()!=BASELINE_PR148_SHA: raise ValueError("NESTED_PR148_SOURCE_SHA_MISMATCH")
    with zipfile.ZipFile(io.BytesIO(raw)) as z:
        inner=z.read("source/PR145_CORRECTED_FULL_SOURCE.zip")
    with zipfile.ZipFile(io.BytesIO(inner)) as z:
        with np.load(io.BytesIO(z.read("results/HIGH_E_HIERARCHY_50x193.npz")),allow_pickle=False) as d:
            hi={k:d[k].copy() for k in d.files}
        op=json.loads(z.read("results/EXACT_LRS_E_OPERATOR.json"))
    assert np.array_equal(state['cases'],hi['cases']) and np.array_equal(state['x'],hi['x'])
    assert state['quadratic_power'].shape==(50,193,4) and np.array_equal(state['cutoffs'],np.array(CUT))
    assert hi['E_l0_plus'].shape==(50,193,12) and np.array_equal(hi['l'],LL)
    return state,hi,np.array(op['k_plus'],float)

def integrate_alignment(e:np.ndarray,k:np.ndarray,L:int,n:int,derivative_basis:bool):
    mu,w=leggauss(n);mu=0.5*(mu+1);w*=0.5
    ids=np.nonzero(LL<=L)[0]
    if derivative_basis:
        basis=np.stack([(1-mu**2)*Legendre.basis(int(LL[j])).deriv(2)(mu) for j in ids],axis=0)
    else:
        basis=np.stack([lpmv(2,int(LL[j]),mu) for j in ids],axis=0)
    q=np.einsum('ctm,mq->ctq',e[:,:,:len(ids)]/k[:len(ids)],basis,optimize=True)
    q2=q*q
    i0=np.einsum('ctq,q->ct',q2,w,optimize=True)
    i2=np.einsum('ctq,q->ct',q2,w*mu*mu,optimize=True)
    valid=i0>0
    eta=np.full_like(i0,np.nan)
    eta[valid]=i2[valid]/i0[valid]
    align=1-3*eta
    return i0,i2,eta,align,valid

def output_csv(path,rows):
    with open(path,"w",encoding="utf8",newline="") as f:
        w=csv.DictWriter(f,fieldnames=list(rows[0]),lineterminator='\n');w.writeheader();w.writerows(rows)

def float_or_blank(a):
    return float(a) if np.isfinite(a) else ''

def run(source,out,figures=True):
    out=pathlib.Path(out);out.mkdir(parents=True,exist_ok=True)
    st,hi,k=extract_parent(source)
    cases=hi['cases'];x=hi['x'];e=hi['E_l0_plus'];b=st['bprime']
    shape=(50,193,4)
    eta=np.full(shape,np.nan);alignment=np.full(shape,np.nan);angle_moment0=np.zeros(shape);angle_moment2=np.zeros(shape)
    valid=np.zeros(shape,dtype=bool)
    max_i0_gap=max_i2_gap=max_qwork_gap=max_eta_gap=0.
    masks=[]
    for u,L in enumerate(CUT):
        i0,i2,h,a,v=integrate_alignment(e,k,L,64,False)
        r0,r2,rh,ra,rv=integrate_alignment(e,k,L,96,True)
        if not np.array_equal(v,rv):raise AssertionError("ANGULAR_ZERO_MASK_DISAGREEMENT")
        if np.any(i2>i0+1e-12) or np.any(i2<-1e-12):raise AssertionError("ANGULAR_MOMENT_OUT_OF_RANGE")
        if np.any(h[v]<-1e-12) or np.any(h[v]>1+1e-12):raise AssertionError("ETA_OUT_OF_BOUNDS")
        if np.any(a[v]<-2-1e-12) or np.any(a[v]>1+1e-12):raise AssertionError("ALIGNMENT_OUT_OF_BOUNDS")
        max_i0_gap=max(max_i0_gap,float(np.max(abs(i0-r0))))
        max_i2_gap=max(max_i2_gap,float(np.max(abs(i2-r2))))
        if np.any(v):max_eta_gap=max(max_eta_gap,float(np.max(abs(h[v]-rh[v]))))
        wmodel=(5/2)*b*(i0-3*i2)
        max_qwork_gap=max(max_qwork_gap,float(np.max(abs(st['shear_internal_work'][:,:,u]-wmodel))))
        # zero norm convention only for exactly zero source; retain NaN for ratio.
        eta[:,:,u]=h;alignment[:,:,u]=a;angle_moment0[:,:,u]=i0;angle_moment2[:,:,u]=i2;valid[:,:,u]=v
        masks.append(int(np.sum(~v)))
    if max(max_i0_gap,max_i2_gap,max_qwork_gap)>1e-10: raise AssertionError("ANGULAR_IDENTITY_TOLERANCE_FAILURE")
    np.savez_compressed(out/'ANGULAR_ALIGNMENT_50x193.npz',cases=cases,x=x,cutoffs=np.array(CUT),eta=eta,alignment=alignment,I0=angle_moment0,I2=angle_moment2,defined=valid)
    names=cases.tolist();by={c:i for i,c in enumerate(names)}
    ends=[]
    for j,c in enumerate(names):
        row={'case':c,'preregistration_commit':PREREG_COMMIT}
        for u,L in enumerate(CUT):
            row[f'alignment_L{L}_end']=float_or_blank(alignment[j,-1,u])
            row[f'eta_L{L}_end']=float_or_blank(eta[j,-1,u])
            row[f'defined_L{L}_end']=int(valid[j,-1,u])
        ends.append(row)
    output_csv(out/'ANGULAR_ALIGNMENT_50_ENDPOINT.csv',ends)
    pairs=[]
    for m in range(5):
      for r in range(5):
        a=by[f's00_m{m:02d}_r{r:02d}'];h=by[f's04_m{m:02d}_r{r:02d}']
        row={'matched_pair':f'm{m:02d}_r{r:02d}'}
        for u,L in enumerate(CUT):
            row[f'delta_alignment_L{L}_end']=float_or_blank(alignment[h,-1,u]-alignment[a,-1,u])
            row[f'delta_eta_L{L}_end']=float_or_blank(eta[h,-1,u]-eta[a,-1,u])
        pairs.append(row)
    output_csv(out/'ANGULAR_ALIGNMENT_25_MATCHED.csv',pairs)
    s04=np.array([i for i,c in enumerate(names) if c.startswith('s04')]);s00=np.array([i for i,c in enumerate(names) if c.startswith('s00')]);assert len(s04)==len(s00)==25
    times=[]
    for j,t in enumerate(x):
        row={'x':float(t)}
        for u,L in enumerate(CUT):
            for family,ids in [('s04',s04),('s00',s00)]:
                values=alignment[ids,j,u];good=np.isfinite(values)
                row[f'{family}_mean_alignment_L{L}']=float_or_blank(np.mean(values[good]) if np.any(good) else np.nan)
                row[f'{family}_defined_n_L{L}']=int(np.sum(good))
        times.append(row)
    output_csv(out/'ANGULAR_ALIGNMENT_193_MEAN.csv',times)
    summary={'schema':'vigilode.bass21.prospective_lrs_polarization_alignment_v1','registered_before_this_experiment':PREREG_COMMIT,'earlier_work_status':'POSTHOC_NOT_PREREGISTERED',
        'source_pr151_full_sha256':ORIGINAL_ARCHIVE_SHA,'source_pr148_full_sha256':BASELINE_PR148_SHA,
        'cases':50,'epochs':193,'cutoffs':list(CUT),'new_ivps':0,'alignment_values_total':int(np.prod(shape)), 'eta_values_total':int(np.prod(shape)),
        'null_counts_by_cutoff':masks,'defined_values_by_cutoff':[int(valid[:,:,u].sum()) for u in range(4)],
        'max_i0_GL64_vs_GL96_basis_gap':max_i0_gap,'max_i2_GL64_vs_GL96_basis_gap':max_i2_gap,'max_eta_GL64_vs_GL96_basis_gap':max_eta_gap,'max_work_identity_gap':max_qwork_gap,
        'min_max_eta_by_cutoff':[[float(np.nanmin(eta[:,:,u])),float(np.nanmax(eta[:,:,u]))] for u in range(4)],
        'min_max_alignment_by_cutoff':[[float(np.nanmin(alignment[:,:,u])),float(np.nanmax(alignment[:,:,u]))] for u in range(4)],
        's04_mean_final_alignment_by_cutoff':[float(np.nanmean(alignment[s04,-1,u])) for u in range(4)],
        's00_mean_final_alignment_by_cutoff':[float(np.nanmean(alignment[s00,-1,u])) for u in range(4)],
        "s04_and_s00_final_quadrature_status":"Frozen s00 high-l polarization unresolved against parent original GL64/96 Q; do not infer physical detection",
        'admission':'EXPLORATORY_SCOPED_NON_AUTHORITATIVE','official_DAG_vigilode':'7/23','official_DAG_delta':0}
    (out/'ANGULAR_ALIGNMENT_SUMMARY.json').write_text(json.dumps(summary,indent=2,sort_keys=True,ensure_ascii=False)+'\n',encoding='utf8')
    if figures:
        import matplotlib
        matplotlib.use('Agg')
        import matplotlib.pyplot as plt
        plt.rcParams['svg.hashsalt']='vigilode-alignment-prospective-20261031'
        figdir=out.parent/'figures';figdir.mkdir(exist_ok=True)
        fig,ax=plt.subplots(figsize=(7.2,4.1))
        for u,L in enumerate(CUT):
            data=alignment[s04,:,u];counts=np.isfinite(data).sum(axis=0)
            mean=np.array([np.nanmean(data[:,j]) if counts[j] else np.nan for j in range(len(x))])
            ax.plot(x,mean,label=f'L={L}')
        ax.set(xlabel='x = ln(a/a_ref)',ylabel='Mean polarization alignment <1-3mu^2>',title='Original BASS21 s04: source-frozen angular polarization shape');ax.legend();ax.grid(alpha=.3);fig.tight_layout()
        fig.savefig(figdir/'S04_ALIGNMENT_EVOLUTION.svg',metadata={'Date':None})
        fig.savefig(figdir/'S04_ALIGNMENT_EVOLUTION.png',dpi=160)
        plt.close(fig)
    return summary

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--source',required=True);p.add_argument('--out',required=True);p.add_argument('--no-figures',action='store_true');a=p.parse_args()
    print(json.dumps(run(a.source,a.out,not a.no_figures),indent=2,sort_keys=True))
