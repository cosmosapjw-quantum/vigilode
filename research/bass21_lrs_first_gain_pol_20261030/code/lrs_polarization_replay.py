#!/usr/bin/env python3
"""Source-owned BASS21 LRS first-gain polarized Thomson line-of-sight integral.

No new initial-value problem; read original full NPZ archive only.
Q = Q_meridional - Q_azimuthal, in a screen basis transported by an
axisymmetric diagonal Bianchi-I normal tetrad. Not full polarized Boltzmann.
"""
from __future__ import annotations
import argparse, csv, hashlib, io, itertools, json, math, pathlib, zipfile
import numpy as np
from numpy.polynomial.legendre import legvander
from scipy.integrate import simpson

ARCHIVE_SHA = '6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8'
ATOM_SHA = 'baecce07aded483fa60006cc576feff67d068c0d70394f7095abacc6d47eaa80'
C = 299792458.0
SIGMA_CM2 = 6.6524587051e-25
K_T = C * SIGMA_CM2 * 1e-4 # m^3/s; c(m/s) * sigma(cm^2 to m^2)
MU = np.linspace(0., 1., 17)
PREFIX = 'inputs/'


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def load_owner_archive(path):
    """Exact 50 original owner SHA check and one-epoch multi-owner fields."""
    path = pathlib.Path(path)
    if sha256(path.read_bytes()) != ARCHIVE_SHA:
        raise ValueError('ORIGINAL_BASS21_ARCHIVE_SHA_DRIFT')
    histories = {}
    with zipfile.ZipFile(path) as z:
        if z.testzip() is not None:
            raise ValueError('ORIGINAL_BASS21_ZIP_CRC_FAILURE')
        manifest = json.loads(z.read('results/SOURCE_INPUT_MANIFEST.json'))
        assert manifest['owner'] == 'cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c'
        assert manifest['source_model_sha256'] == ATOM_SHA
        sources=manifest['sources']
        if len(sources)!=50: raise ValueError('BAD_HISTORY_COUNT')
        frozen_x=None
        for case, row in sorted(sources.items()):
            binary=z.read(PREFIX+case+'.npz')
            if len(binary)!=row['bytes'] or sha256(binary)!=row['sha256']:
                raise ValueError('BASS21_SOURCE_BYTES_CHANGED_'+case)
            with np.load(io.BytesIO(binary),allow_pickle=False) as a:
                x=a['x'].copy();y=a['raw_state62'].copy();di=a['diagnostics'].copy()
                names=list(a['diagnostic_names'].tolist())
                assert y.shape==(193,62) and x.shape==(193,)
                if frozen_x is None: frozen_x=x
                elif not np.array_equal(x,frozen_x): raise ValueError('EPOCH_GRID_MISMATCH')
                def get(name): return di[:,names.index(name)].copy()
                b=get('b');h=get('H');ne=get('ne_m3')
                g=get('gamma_T_per_H'); tau=get('tau_T')
                if not (np.array_equal(b,y[:,1]) and np.array_equal(tau,y[:,61])):
                    raise ValueError('ORIGINAL_GEOMETRY_TAU_STATE_MISMATCH')
                if not np.allclose(g,K_T*ne/h,atol=1e-16,rtol=1e-12):
                    raise ValueError('ORIGINAL_RATE_CONVENTION_MISMATCH')
                if np.any(np.diff(x)<=0) or np.min(g)<0 or np.min(np.diff(tau))<-1e-12:
                    raise ValueError('SOURCE_CLOCK_NOT_PHYSICAL')
                if np.min(h)<=0 or not np.isfinite(y).all():raise ValueError('BAD_OWNER_PHYSICAL_STATE')
                histories[case] = dict(x=x,b=b,h=h,ne=ne,g=g,tau=tau,
                                        photon_energy_moments=y[:,4:13].copy(),
                                        source_sha256=row['sha256'],source_git_blob=row['git_blob'])
    return histories


def ray_factor(bx, bo, muo):
    """D^2=E_x^2(a_x/a_o)^2/E_o^2; source-sky mu propagation exact LRS."""
    delta=np.asarray(bx)[...,None]-float(bo)
    muo=np.asarray(muo)
    d2 = (1-muo**2)*np.exp(2*delta)+muo**2*np.exp(-4*delta)
    if np.any(d2<=0): raise ValueError('NEGATIVE_NULL_ENERGY')
    mux=muo*np.exp(-2*delta)/np.sqrt(d2)
    return d2,mux


def integrand(g,j2,b,tau,x,mu,observer_idx):
    """Normalized emitted Q after geodesic screen transport/redshift/extinction.

    Q_o = -.75 int g J2 sin²theta_x / D^4 * exp(-(tau_o-tau_x)) dx.
    Uses local F=a^4 I/(I_ref a_ref^4). Positive stored forward tau is
    accumulated from 0 to x; emission-to-observer attenuation uses tau_o-tau_x.
    """
    j=int(observer_idx)
    d2,mux=ray_factor(b[:j+1],b[j],mu)
    # Algebraically equivalent but guarantees exact zero on the symmetry axis.
    delta=np.asarray(b[:j+1])[:,None]-b[j]
    geometry=(1-np.asarray(mu)**2)[None,:]*np.exp(2*delta)/d2**3
    if np.any(geometry<-1e-12): raise ValueError('NEGATIVE_SCREEN_FACTOR')
    attenuation=np.exp(-(tau[j]-tau[:j+1]))[:,None]
    return -.75*(g[:j+1]*j2[:j+1])[:,None]*geometry*attenuation


def integrate_los(g,j2,b,tau,x,mu=MU,all_epochs=True,method='simpson'):
    g=np.asarray(g);j2=np.asarray(j2);b=np.asarray(b);tau=np.asarray(tau);x=np.asarray(x);mu=np.asarray(mu)
    assert all(q.shape==x.shape for q in (g,j2,b,tau))
    n=len(x); out=np.zeros((n,len(mu))) if all_epochs else np.zeros((1,len(mu)))
    for j in (range(1,n) if all_epochs else [n-1]):
        yy=integrand(g,j2,b,tau,x,mu,j)
        if method=='simpson' and j>=2: value=simpson(yy, x=x[:j+1],axis=0)
        else:value=np.trapezoid(yy,x=x[:j+1],axis=0)
        out[j if all_epochs else 0]=value
    return out


def source_local_Q(j2,mu=MU):
    return -.75*np.asarray(j2)[...,None]*(1-np.asarray(mu)**2)


def brightness(moments,mu=MU):
    moments=np.asarray(moments)
    if moments.shape[-1]!=9: raise ValueError('EXPECTED_L8_PHOTON_MOMENTS')
    return (moments*(2*np.arange(9)+1))@legvander(np.asarray(mu),8).T


def swapped_final(A,B,mask,mu=MU):
    """8 corners: group bit1=b geometry, bit2=J2, bit4=(g,tau) collision clock."""
    srcb=B if mask&1 else A
    srcj=B if mask&2 else A
    srcc=B if mask&4 else A
    return integrate_los(srcc['g'],srcj['photon_energy_moments'][:,2],srcb['b'],
              srcc['tau'],srcc['x'],mu=mu,all_epochs=False)[0]


def exact_three_block_shapley(corner):
    """Finite cooperative exact telescoping mean over all 3! orders, not causal."""
    out={1:np.zeros_like(corner[0]),2:np.zeros_like(corner[0]),4:np.zeros_like(corner[0])}
    for perm in itertools.permutations((1,2,4)):
        mask=0
        for bit in perm:
            nextmask=mask|bit;out[bit]+=(corner[nextmask]-corner[mask])/6
            mask=nextmask
    return out


def write_csv(path,rows,columns):
    with pathlib.Path(path).open('w',newline='') as f:
        w=csv.DictWriter(f,fieldnames=columns,lineterminator='\n')
        w.writeheader(); w.writerows(rows)


def process(archive,output):
    """Generate actual physical-source arrays and matched attribution."""
    out=pathlib.Path(output);out.mkdir(parents=True,exist_ok=True)
    orig=load_owner_archive(archive);cases=sorted(orig)
    mu=MU.copy();npairs=25
    q=np.empty((len(cases),193,len(mu)),dtype=np.float64)
    intensity=np.empty_like(q)
    qloc=np.empty_like(q)
    rows=[]
    for i,case in enumerate(cases):
        h=orig[case];mom=h['photon_energy_moments']
        q[i]=integrate_los(h['g'],mom[:,2],h['b'],h['tau'],h['x'],mu)
        intensity[i]=brightness(mom,mu)
        qloc[i]=source_local_Q(mom[:,2],mu)
        if np.any(intensity[i]<=0):raise ValueError('NEGATIVE_OR_ZERO_OWNER_BRIGHTNESS')
        if np.any(np.abs(q[i])>intensity[i]):raise ValueError('UNPHYSICAL_FIRST_ORDER_Q_LARGER_THAN_I')
        rows.append({'case':case,'source_sha256':h['source_sha256'],
              'final_tau_saved':float(h['tau'][-1]),'final_b':float(h['b'][-1]),
              'final_J2':float(mom[-1,2]),'Q_equator_end':float(q[i,-1,0]),
              'Q_over_I_equator_end':float(q[i,-1,0]/intensity[i,-1,0]),
              'max_abs_Q_all':float(np.max(np.abs(q[i]))),
              'max_abs_Q_over_I_all':float(np.max(np.abs(q[i]/intensity[i])))})
    pairrows=[];shapleyrows=[];rawrows=[]
    pair_q=np.empty((npairs,193,len(mu)))
    attribution_residual=0.0
    for pi,(m,r) in enumerate(itertools.product(range(5),repeat=2)):
        caseA=f's00_m{m:02d}_r{r:02d}';caseB=f's04_m{m:02d}_r{r:02d}'
        A=orig[caseA];B=orig[caseB];ia=cases.index(caseA);ib=cases.index(caseB)
        pair_q[pi]=q[ib]-q[ia]
        corners={mask:swapped_final(A,B,mask,mu) for mask in range(8)}
        s=exact_three_block_shapley(corners)
        if not np.allclose(corners[0],q[ia,-1],rtol=0,atol=2e-14) or not np.allclose(corners[7],q[ib,-1],rtol=0,atol=2e-14):
            raise ValueError('SHAPLEY_ENDPOINT_MISMATCH')
        residual=float(np.max(np.abs(sum(s.values())-pair_q[pi,-1])))
        attribution_residual=max(attribution_residual,residual)
        for k,v in enumerate(mu):
            pairrows.append({'pair':f'm{m:02d}_r{r:02d}','mu_observer':float(v),
              'Q_A_end':float(q[ia,-1,k]),'Q_B_end':float(q[ib,-1,k]),
              'delta_Q_end':float(pair_q[pi,-1,k]),
              'Q_over_I_A_end':float(q[ia,-1,k]/intensity[ia,-1,k]),
              'Q_over_I_B_end':float(q[ib,-1,k]/intensity[ib,-1,k]),
              'delta_Q_over_I':float(q[ib,-1,k]/intensity[ib,-1,k]-q[ia,-1,k]/intensity[ia,-1,k])})
            shapleyrows.append({'pair':f'm{m:02d}_r{r:02d}','mu_observer':float(v),
              'delta_Q_end':float(pair_q[pi,-1,k]),'geometry_b':float(s[1][k]),
              'photon_J2':float(s[2][k]),'opacity_g_tau':float(s[4][k]),
              'reconstruction_residual':float(sum(si[k] for si in s.values())-pair_q[pi,-1,k])})
    # Closed-source grid result: 50*193*17 Q entries, not independent new IVPs.
    np.savez_compressed(out/'LRS_50x193x17_POLARIZATION.npz',cases=np.array(cases),
           x=orig[cases[0]]['x'],mu=mu,Q=q,Q_local_gain=qloc,I_baseline=intensity,
           pair_delta_Q=pair_q,pairs=np.array([f'm{m:02d}_r{r:02d}' for m,r in itertools.product(range(5),repeat=2)]))
    write_csv(out/'FINAL_50_DIRECTIONAL_POLARIZATION.csv',rows,list(rows[0]))
    write_csv(out/'MATCHED_25_DIRECTIONAL_POLARIZATION.csv',pairrows,list(pairrows[0]))
    write_csv(out/'MATCHED_25_SHAPLEY_3BLOCK.csv',shapleyrows,list(shapleyrows[0]))
    # Angle error diagnostic one selected case: old trapezoid vs stored-grid Simpson only.
    sample=orig['s04_m02_r02'];qt=integrate_los(sample['g'],sample['photon_energy_moments'][:,2],sample['b'],sample['tau'],sample['x'],mu,method='trapezoid')
    qs=q[cases.index('s04_m02_r02')]
    sumn={
      'schema':'vigilode.bass21.lrs_first_scatter_stokesQ.v1',
      'unit':'BASS21_SOURCE_OWNED_LRS_SINGLE_SCATTER_POLARIZATION',
      'date_kst':'2026-10-30','input_archive_sha256':ARCHIVE_SHA,
      'original_atomic_network_sha256':ATOM_SHA,
      'original_owner':'cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c',
      'source_case_count':50,'saved_epochs_per_case':193,'mu_half_sky_count':len(mu),
      'stokes_Q_saved_model_rows':int(q.size),'paired_delta_Q_rows':int(pair_q.size),
      'matched_endpoint_rows':len(pairrows),'shapley_rows':len(shapleyrows),
      'new_original_geometry_chemistry_IVPs':0,'new_polarized_ivps':0,
      'max_abs_Q_over_I':float(np.max(np.abs(q/intensity))),
      'mean_delta_Q_equator':float(np.mean(pair_q[:,-1,0])),
      'delta_Q_equator_range':[float(pair_q[:,-1,0].min()),float(pair_q[:,-1,0].max())],
      'mean_Q_A_equator':float(np.mean([q[cases.index(f's00_m{m:02d}_r{r:02d}'),-1,0] for m,r in itertools.product(range(5),repeat=2)])),
      'mean_Q_B_equator':float(np.mean([q[cases.index(f's04_m{m:02d}_r{r:02d}'),-1,0] for m,r in itertools.product(range(5),repeat=2)])),
      'shapley_identity_max_abs':attribution_residual,
      'simpson_vs_trapezoid_max_abs_selected':float(np.max(np.abs(qs-qt))),
      'all_Q_poles_exact_zero':bool(np.array_equal(q[:,:,-1],np.zeros((50,193)))),
      'all_Q_initial_exact_zero':bool(np.array_equal(q[:,0,:],np.zeros((50,len(mu))))),
      'limits':{'source':'unpolarized L8 photon baseline with polarized backreaction neglected',
                'collision':'cold normal-comoving electron Thomson, first-gain with extinction; no polarized re-scattering gain',
                'geometry':'same-source LRS axisymmetric Bianchi-I metric and b(x), no CE1 six-cell substitution',
                'sky':'model-frame E-integrated Q at finite z=2, not observed CMB E/B or tau',
                'numerics':'sampled 193-grid Simpson; no BDF global error certificate; endpoint Q/I may not be a full self-consistent polarization solution'},
      'reproduction':'python code/lrs_polarization_replay.py --archive ORIGINAL_BASS21_SOURCE_FULL.zip --out results',
    }
    (out/'SCIENCE_SUMMARY.json').write_text(json.dumps(sumn,indent=2,sort_keys=True,allow_nan=False)+'\n')
    return sumn


def main():
    p=argparse.ArgumentParser();p.add_argument('--archive',required=True);p.add_argument('--out',required=True)
    args=p.parse_args();print(json.dumps(process(args.archive,args.out),indent=2,sort_keys=True))
if __name__=='__main__':main()