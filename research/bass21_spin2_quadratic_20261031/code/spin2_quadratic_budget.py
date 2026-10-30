#!/usr/bin/env python3
"""Source-frozen spin-2 finite-band quadratic angular power and shear-work.

Reuses immutable PR145/PR143 physical histories.  No owner IVP is integrated.
All quantities are squared polarisation angular norm, NOT radiation energy density.
"""
from __future__ import annotations
import argparse,csv,hashlib,io,json,math,pathlib,zipfile
import numpy as np
from scipy.special import lpmv
from numpy.polynomial.legendre import leggauss,Legendre
from scipy.integrate import cumulative_trapezoid

CHILD_SHA='626ea52c9fae38db23e0dd9e5f628ed0cba35ee05d54391b02b91ca34a0b3b1c'
PARENT_SHA='c7626d6f7e01e578e4e9825e60ea8e1631e6ec79cc535867d95fc690b0e0c564'
PR145_HEAD='6bb55eed42e01db9c3715d17c858d8e5632d6a38'
PR148_HEAD='2e29f98379d3a7d56cf3cbcda0c71355fbc0117c'
BASS_COMMIT='cc602e209e7deeb511ab2a1f3b2268d55206c43c'
CUT=[16,18,20,22]
LL=np.arange(2,25,2)


def sha_bytes(b:bytes):return hashlib.sha256(b).hexdigest()
def sha_file(p):
    with open(p,'rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def npz_bytes(b):
    with np.load(io.BytesIO(b),allow_pickle=False) as n:
        return {k:n[k].copy() for k in n.files}

def source_tables(child_zip):
    child_zip=pathlib.Path(child_zip)
    if sha_file(child_zip)!=CHILD_SHA:raise ValueError('PR148_SOURCE_SHA_MISMATCH')
    with zipfile.ZipFile(child_zip) as ch:
        if ch.testzip() is not None:raise ValueError('PR148_CRC_FAIL')
        parent=ch.read('source/PR145_CORRECTED_FULL_SOURCE.zip')
        if sha_bytes(parent)!=PARENT_SHA:raise ValueError('PR145_SOURCE_SHA_MISMATCH')
        p_mem=npz_bytes(ch.read('results/SOURCE_FROZEN_BOUNDARY_MEMORY_50x193.npz'))
    with zipfile.ZipFile(io.BytesIO(parent)) as pr145:
        high=npz_bytes(pr145.read('results/HIGH_E_HIERARCHY_50x193.npz'))
        op=json.loads(pr145.read('results/EXACT_LRS_E_OPERATOR.json'))
        p143=pr145.read('source/PARENT_PR143_FULL_SOURCE.zip')
        p141=npz_bytes(pr145.read('source/PR141_EHARM_50x193.npz'))
    with zipfile.ZipFile(io.BytesIO(p143)) as pr143:
        anc=npz_bytes(pr143.read('results/EIGENMODE_50x193_SOURCE_PHYSICS.npz'))
    if any(not np.array_equal(high['cases'], v['cases']) for v in [anc,p141,p_mem]):raise ValueError('SOURCE_CASE_MISMATCH')
    if any(not np.array_equal(high['x'], v['x']) for v in [anc,p141,p_mem]):raise ValueError('SOURCE_CLOCK_MISMATCH')
    if not np.array_equal(high['l'],LL) or not np.array_equal(p_mem['cutoffs'],np.asarray(CUT)):raise ValueError('SOURCE_ELL_MISMATCH')
    if high['E_l0_plus'].shape!=(50,193,12) or high['shear_E'].shape!=(50,193,11):raise ValueError('SOURCE_SHAPE_MISMATCH')
    if not np.array_equal(anc['tau'],p_mem['tau']):raise ValueError('SOURCE_TAU_MISMATCH')
    if not np.isfinite(anc['g']).all() or np.any(anc['g']<0):raise ValueError('SOURCE_G_NOT_POSITIVE')
    if abs(np.max(abs(high['E_l0_plus'][:,:,:8]-p141['E_l0_plus'])))>1e-10:raise ValueError('SOURCE_OLD_E_MISMATCH')
    if np.any(np.diff(high['x'])<=0):raise ValueError('CLOCK_NOT_ASCENDING')
    return high,anc,p141,p_mem,op

def galerk_matrix(op):
    k=np.array(op['k_plus'],dtype=float)
    if not np.array_equal(op['l'],LL.tolist()):raise ValueError('OPERATOR_ELL_MISMATCH')
    ac=np.zeros((12,12),dtype=float)
    for i,row in enumerate(op['A_c_rows']):
        for key,val in row.items():
            import fractions
            j=int(np.nonzero(LL==int(key))[0][0]);ac[i,j]=float(fractions.Fraction(val))
    ae=k[:,None]*ac/k[None,:]
    return ae,k

def sky_independent(E,k,L,nquad=48,second_basis=False):
    # Exact polynomial integrand, 48-point Gauss quadrature is overkill for l<=22.
    mu,w=leggauss(nquad);mu=(mu+1)/2;w=w/2
    ix=np.where(LL<=L)[0]
    if second_basis:
        basis=np.array([(1-mu**2)*Legendre.basis(int(LL[j])).deriv(2)(mu) for j in ix])
    else:
        basis=np.array([lpmv(2,int(LL[j]),mu) for j in ix])
    Q=np.einsum('ctm,mq->ctq',E[:,:,ix]/k[ix],basis,optimize=True)
    norm=np.einsum('ctq,q->ct',Q**2,w,optimize=True)
    weighted=np.einsum('ctq,q->ct',Q**2,w*(1-3*mu**2),optimize=True)
    return norm,weighted

def emitted_csv(path,rows):
    with open(path,'w',encoding='utf8',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(rows[0]),lineterminator='\n');w.writeheader();w.writerows(rows)

def process(source,out,make_figures=True):
    high,anc,p141,p_mem,op=source_tables(source)
    out=pathlib.Path(out);out.mkdir(parents=True,exist_ok=True)
    x=high['x'];e=high['E_l0_plus'];sh=high['shear_E'];coll=high['collision_E'];b=anc['bprime'];g=anc['g'];
    ae,k=galerk_matrix(op)
    # Full source-to-output crosswalk with original owner's b'(x) and J2.
    predicted=b[:,:,None]*np.einsum('ctm,lm->ctl',e,ae[:11,:],optimize=True)
    max_shear_gap=float(np.max(abs(predicted-sh)))
    expect_collision=-g[:,:,None]*e[:,:,:11];expect_collision[:,:,0]=-g*(k[0]*p141['J2_total']/4+2*e[:,:,0]/5)
    max_collision_gap=float(np.max(abs(expect_collision-coll)))
    flux_pred=np.stack([b*ae[np.where(LL==L)[0][0],np.where(LL==L+2)[0][0]]*e[:,:,np.where(LL==L+2)[0][0]] for L in CUT],axis=-1)
    max_flux_gap=float(np.max(abs(flux_pred-p_mem['original_flux'])))
    if max_shear_gap>2e-12 or max_collision_gap>2e-12 or max_flux_gap>1e-24:raise AssertionError(('PARENT_OPERATOR_MISMATCH',max_shear_gap,max_collision_gap,max_flux_gap))
    ncase,nt=e.shape[:2];ns=len(CUT);shape=(ncase,nt,ns)
    fields={name:np.zeros(shape) for name in ['angular_half_norm','quadratic_power','shear_internal_work','upper_neighbour_work','thomson_quadrupole_source','thomson_loss','model_quadratic_rhs','source_grid_derivative_defect','cumulative_boundary_work','sky_work_independent','sky_norm_independent']}
    norm_gap=work_gap=work_basis_gap=closure_gap=0.
    for z,L in enumerate(CUT):
        ix=int(np.where(LL==L)[0][0]);sl=slice(0,ix+1);E=e[:,:,sl]
        fields['quadratic_power'][:,:,z]=np.sum(E*E,axis=-1)/(8*math.pi)
        fields['angular_half_norm'][:,:,z]=2*fields['quadratic_power'][:,:,z]
        all_shear=np.sum(E*sh[:,:,sl],axis=-1)/(4*math.pi)
        boundary=E[:,:,-1]*flux_pred[:,:,z]/(4*math.pi)
        internal=all_shear-boundary
        fields['upper_neighbour_work'][:,:,z]=boundary
        fields['shear_internal_work'][:,:,z]=internal
        qsource=(-g*k[0]*p141['J2_total']/4)*e[:,:,0]/(4*math.pi)
        damping=-g*(2*e[:,:,0]**2/5+np.sum(e[:,:,1:ix+1]**2,axis=-1))/(4*math.pi)
        fields['thomson_quadrupole_source'][:,:,z]=qsource
        fields['thomson_loss'][:,:,z]=damping
        coll_power=np.sum(E*coll[:,:,sl],axis=-1)/(4*math.pi)
        closure_gap=max(closure_gap,float(np.max(abs(coll_power-(qsource+damping)))))
        fields['model_quadratic_rhs'][:,:,z]=internal+boundary+qsource+damping
        fields['source_grid_derivative_defect'][:,:,z]=np.gradient(fields['quadratic_power'][:,:,z],x,axis=1,edge_order=2)-fields['model_quadratic_rhs'][:,:,z]
        fields['cumulative_boundary_work'][:,:,z]=cumulative_trapezoid(boundary,x,axis=1,initial=0)
        sky_n,sky_w=sky_independent(e,k,L,48)
        sky_n2,sky_w2=sky_independent(e,k,L,64,second_basis=True)
        fields['sky_norm_independent'][:,:,z]=sky_n
        fields['sky_work_independent'][:,:,z]=(5/2)*b*sky_w
        norm_gap=max(norm_gap,float(np.max(abs(sky_n-fields['angular_half_norm'][:,:,z]))))
        work_gap=max(work_gap,float(np.max(abs((5/2)*b*sky_w-internal))))
        work_basis_gap=max(work_basis_gap,float(np.max(abs((5/2)*b*sky_w-(5/2)*b*sky_w2))))
        assert np.all(damping<=1e-28)
        assert np.all(fields['quadratic_power'][:,:,z]>=0)
        assert np.all(fields['cumulative_boundary_work'][:,0,z]==0)
    assert norm_gap<2e-14 and work_gap<2e-14 and work_basis_gap<2e-14,(norm_gap,work_gap,work_basis_gap)
    assert closure_gap<2e-14 and max_shear_gap<2e-12
    # Five source primary physical fields plus cumulative sixth, and independently measured/diagnostic RHS.
    np.savez_compressed(out/'SOURCE_FROZEN_SPIN2_POWER_50x193.npz',cases=high['cases'],x=x,cutoffs=np.array(CUT),bprime=b,g=g,tau=anc['tau'],**fields)
    names=high['cases'].tolist();index={name:i for i,name in enumerate(names)}
    endpoints=[]
    for ci,name in enumerate(names):
        row={'case':name,'source_PR148_HEAD':PR148_HEAD}
        for z,L in enumerate(CUT):
            for field in ['quadratic_power','shear_internal_work','upper_neighbour_work','thomson_quadrupole_source','thomson_loss','cumulative_boundary_work']:
                row[f'{field}_L{L}_final']=float(fields[field][ci,-1,z])
        endpoints.append(row)
    matched=[]
    for mi in range(5):
      for ri in range(5):
        low=index[f's00_m{mi:02d}_r{ri:02d}'];high_id=index[f's04_m{mi:02d}_r{ri:02d}']
        row={'matched_pair':f'm{mi:02d}_r{ri:02d}'}
        for z,L in enumerate(CUT):
            for field in ['quadratic_power','upper_neighbour_work','cumulative_boundary_work','shear_internal_work']:
                row[f'delta_{field}_L{L}_final']=float(fields[field][high_id,-1,z]-fields[field][low,-1,z])
        matched.append(row)
    times=[]
    s04=np.array([i for i,name in enumerate(names) if name.startswith('s04')]);s00=np.array([i for i,name in enumerate(names) if name.startswith('s00')])
    for ti,t in enumerate(x):
        row={'x':float(t)}
        for z,L in enumerate(CUT):
            for field in ['quadratic_power','upper_neighbour_work','shear_internal_work','thomson_loss','cumulative_boundary_work']:
                row[f's04_mean_{field}_L{L}']=float(fields[field][s04,ti,z].mean())
                row[f's00_mean_{field}_L{L}']=float(fields[field][s00,ti,z].mean())
        times.append(row)
    emitted_csv(out/'ENDPOINT_50_SPIN2_POWER.csv',endpoints)
    emitted_csv(out/'MATCHED_25_SPIN2_POWER.csv',matched)
    emitted_csv(out/'TIME193_SPIN2_POWER.csv',times)
    summary={
    'schema':'vigilode.bass21.source_frozen_spin2_finite_band_quadratic_power_v1',
    'source_pr148_zip_sha256':CHILD_SHA,'source_pr145_zip_sha256':PARENT_SHA,
    'parent_pr148_head':PR148_HEAD,'immutable_owner_commit':BASS_COMMIT,
    'cases':50,'epochs':193,'cutoffs':CUT,'new_ivps':0,
    'primary_source_sample_count':50*193*4,
    'new_six_field_samples':50*193*4*6,
    'model_rhs_and_grid_defect_samples':50*193*4*2,
    'max_shear_generator_reconstruction_gap':max_shear_gap,'max_thomson_reconstruction_gap':max_collision_gap,
    'max_parent_upper_flux_gap':max_flux_gap,'max_angular_norm_identity_gap':norm_gap,
    'max_angular_shear_work_identity_gap':work_gap,'max_independent_basis_work_gap':work_basis_gap,
    'max_collision_channel_split_gap':closure_gap,
    'max_source_grid_derivative_defect':float(np.max(abs(fields['source_grid_derivative_defect']))),
    'max_abs_upper_neighbour_work':np.max(abs(fields['upper_neighbour_work']),axis=(0,1)).tolist(),
    'max_abs_integrated_upper_work':np.max(abs(fields['cumulative_boundary_work']),axis=(0,1)).tolist(),
    'max_abs_shear_internal_work':np.max(abs(fields['shear_internal_work']),axis=(0,1)).tolist(),
    'max_abs_model_rhs':np.max(abs(fields['model_quadratic_rhs']),axis=(0,1)).tolist(),
    'max_abs_thomson_loss':np.max(abs(fields['thomson_loss']),axis=(0,1)).tolist(),
    'max_abs_quadratic_power':np.max(fields['quadratic_power'],axis=(0,1)).tolist(),
    'mean_matched_end_cum_work_delta':np.mean([[v[f'delta_cumulative_boundary_work_L{L}_final'] for L in CUT] for v in matched],axis=0).tolist(),
    'convention':'E_l0^+=k_l c_l; integral_0^1 |Q_L|^2 dmu = sum E_l^2/(4pi), P_L=sum E_l^2/(8pi).',
    'interpretation':'SOURCE-FROZEN quadratic polarization angular norm local generator; NOT photon stress-energy, NOT time-certified E evolution, NOT actual truncated hierarchy error.',
    'official_DAG':{'vigilode':'7/23','rei_central':'28/88','rei_working':'33/88','bass_ce1':'9/15','successor_admitted':'0/12','official_delta':0},
    'production':'HOLD','distinct_author_R6_C9':'NOT_RUN','native_subagent':'NOT_RUN','auto_next':False,
    }
    (out/'SCIENCE_SUMMARY.json').write_text(json.dumps(summary,sort_keys=True,indent=2,ensure_ascii=False)+'\n',encoding='utf8')
    if make_figures:
        import matplotlib
        matplotlib.use('Agg')
        import matplotlib.pyplot as plt
        plt.rcParams['svg.hashsalt']='vigilode-spin2-angular-budget-20261031'
        figdir=out.parent/'figures';figdir.mkdir(exist_ok=True)
        fig,ax=plt.subplots(figsize=(7.2,4.2))
        for z,L in enumerate(CUT):ax.plot(x,fields['quadratic_power'][s04,:,z].mean(axis=0),label=f'L={L}')
        ax.set(xlabel=r'$x=\ln(a/a_\mathrm{ref})$',ylabel=r'Mean $\mathcal{P}_L=\sum E_l^2/(8\pi)$',title='Original BASS21 strong-shear spin-2 finite-band power');ax.legend();ax.grid(alpha=.3);fig.tight_layout()
        fig.savefig(figdir/'S04_QUADRATIC_POWER.svg',metadata={'Date':None});fig.savefig(figdir/'S04_QUADRATIC_POWER.png',dpi=180);plt.close(fig)
        fig,ax=plt.subplots(figsize=(7.2,4.2))
        for z,L in enumerate(CUT):ax.plot(x,np.mean(abs(fields['upper_neighbour_work'][s04,:,z]),axis=0),label=f'L={L}')
        ax.set(xlabel=r'$x=\ln(a/a_\mathrm{ref})$',ylabel='Mean |finite-band boundary work|',yscale='log',title='Source-owned high-E shear work crossing truncation boundary');ax.legend();ax.grid(alpha=.3);fig.tight_layout()
        fig.savefig(figdir/'S04_UPPER_BOUNDARY_WORK.svg',metadata={'Date':None});fig.savefig(figdir/'S04_UPPER_BOUNDARY_WORK.png',dpi=180);plt.close(fig)
        fig,ax=plt.subplots(figsize=(7.2,4.2))
        for z,L in enumerate(CUT):
            deltas=np.array([v[f'delta_cumulative_boundary_work_L{L}_final'] for v in matched]);ax.plot(np.arange(25),deltas,marker='.',label=f'L={L}')
        ax.set(xlabel='Original 25 matched (m,r) pairs',ylabel='s04−s00 integrated boundary work',title='Quadratic shear-boundary work: source-matched anisotropy contrast');ax.legend();ax.grid(alpha=.3);fig.tight_layout()
        fig.savefig(figdir/'MATCHED25_INTEGRATED_WORK.svg',metadata={'Date':None});fig.savefig(figdir/'MATCHED25_INTEGRATED_WORK.png',dpi=180);plt.close(fig)
    return summary

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--source',required=True);p.add_argument('--output',required=True);p.add_argument('--no-figures',action='store_true');a=p.parse_args()
    print(json.dumps(process(a.source,a.output,not a.no_figures),ensure_ascii=False,indent=2,sort_keys=True))
