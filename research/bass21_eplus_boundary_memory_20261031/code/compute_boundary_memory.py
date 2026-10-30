#!/usr/bin/env python3
"""No-new-IVP source-frozen BASS21 spin-2 truncation boundary Volterra memory.

Original BASS histories and polarization are NEVER evolved here. Only use
source-frozen saved parent tau,g,bprime and existing measured E, boundary flux.
Output is a test Green-weighted source diagnostic, NOT full hierarchy error.
"""
from __future__ import annotations
import argparse, hashlib, io, json, csv, pathlib, zipfile, math
from dataclasses import dataclass
import numpy as np
from scipy.integrate import cumulative_simpson

SOURCE_SHA = 'c7626d6f7e01e578e4e9825e60ea8e1631e6ec79cc535867d95fc690b0e0c564'
PARENT_SHA = 'a2fa5e7cbae98cb7a28df19314b1c6da8b90ad5277c08831384290171f4231ee'
OWNER_REPO = 'cosmosapjw-quantum/bass'
OWNER_COMMIT = 'cc602e209e7deeb511ab2a1f3b2268d55206c43c'
SRC_PR145_HEAD = '6bb55eed42e01db9c3715d17c858d8e5632d6a38'
CUT = (16,18,20,22)


def sha(data: bytes)->str: return hashlib.sha256(data).hexdigest()

def read_source(zip_path: pathlib.Path):
    assert sha(zip_path.read_bytes()) == SOURCE_SHA, 'source ZIP byte-identity changed'
    with zipfile.ZipFile(zip_path) as z:
        assert z.testzip() is None
        mf=json.loads(z.read('FILE_MANIFEST.json'))
        assert mf['file_count']==37
        for name,expected in mf['files_sha256'].items():
            assert sha(z.read(name))==expected,(name,'manifest mismatch')
        with np.load(io.BytesIO(z.read('results/HIGH_E_HIERARCHY_50x193.npz')),allow_pickle=False) as n:
            atlas={k:n[k].copy() for k in n.files}
        with np.load(io.BytesIO(z.read('source/PR141_EHARM_50x193.npz')),allow_pickle=False) as n:
            old_cases=n['cases'].copy();old_x=n['x'].copy()
        pb=z.read('source/PARENT_PR143_FULL_SOURCE.zip')
        assert sha(pb)==PARENT_SHA
        with zipfile.ZipFile(io.BytesIO(pb)) as parent:
            assert parent.testzip() is None
            with np.load(io.BytesIO(parent.read('results/EIGENMODE_50x193_SOURCE_PHYSICS.npz')),allow_pickle=False) as n:
                p={k:n[k].copy() for k in ('cases','x','g','tau','bprime')}
    assert np.array_equal(atlas['cases'],p['cases']) and np.array_equal(atlas['cases'],old_cases)
    assert np.array_equal(atlas['x'],p['x']) and np.array_equal(atlas['x'],old_x)
    assert np.all(np.diff(atlas['x'])>0) and np.all(p['g']>=0)
    assert np.all(np.diff(p['tau'],axis=1)>=0) and np.all(p['tau']>=0)
    assert p['tau'][:,0].max()==0
    assert atlas['E_l0_plus'].shape==(50,193,12) and atlas['boundary_flux_16_18_20_22'].shape==(50,193,4)
    assert tuple(atlas['l'])==tuple(range(2,25,2))
    return atlas,p


def factor(l): return math.sqrt(4*math.pi/(2*l+1)*math.factorial(l+2)/math.factorial(l-2))

def ac_up(l): return 3*(l-1)*(l+3)*(l+4)/((2*l+3)*(2*l+5))

def reconstruct_flux(atlas, parent):
    cf = np.array([ac_up(l)*factor(l)/factor(l+2) for l in CUT])
    return parent['bprime'][:,:,None]*atlas['E_l0_plus'][:,:,8:12]*cf[None,None,:]


def direct_weighted_trapezoid(x,tau,f):
    """Original saved samples, no IVP or new owner solve.
    Weighted integral exp(-tau(x)) integral_0^x exp(tau(s))*f(s) ds,
    composite trapezoid in x for each case and cutoff.
    """
    weighted=f*np.exp(tau[:,:,None])
    steps=0.5*(weighted[:,1:,:]+weighted[:,:-1,:])*np.diff(x)[None,:,None]
    summed=np.concatenate([np.zeros_like(f[:,:1,:]),np.cumsum(steps,axis=1)],axis=1)
    return np.exp(-tau[:,:,None])*summed


def recurrence_weighted_trapezoid(x,tau,f):
    """Independent recurrence for the same weighted trapezoid rule."""
    out=np.zeros_like(f)
    for k in range(1,len(x)):
        decay=np.exp(-(tau[:,k]-tau[:,k-1]))[:,None]
        h=x[k]-x[k-1]
        out[:,k,:]=decay*out[:,k-1,:]+0.5*h*(decay*f[:,k-1,:]+f[:,k,:])
    return out


def independent_cumulative_simpson(x,tau,f):
    z=f*np.exp(tau[:,:,None]); val=cumulative_simpson(z,x=x,axis=1,initial=0)
    return val*np.exp(-tau[:,:,None])


def emit_csv(path,rows):
    assert rows
    with open(path,'w',encoding='utf-8',newline='') as fp:
        wr=csv.DictWriter(fp,fieldnames=list(rows[0]),lineterminator='\n')
        wr.writeheader();wr.writerows(rows)


def compute(bundle:str,out_dir:str):
    atlas,p=read_source(pathlib.Path(bundle));out=pathlib.Path(out_dir);out.mkdir(parents=True,exist_ok=True)
    x=atlas['x']; f=atlas['boundary_flux_16_18_20_22']; tau=p['tau']; cases=atlas['cases']
    source_reconstructed=reconstruct_flux(atlas,p)
    max_flux_gap=float(np.max(np.abs(source_reconstructed-f)))
    assert max_flux_gap<5e-26,max_flux_gap
    signed=direct_weighted_trapezoid(x,tau,f)
    gross=direct_weighted_trapezoid(x,tau,np.abs(f))
    alt=recurrence_weighted_trapezoid(x,tau,f)
    alt_gross=recurrence_weighted_trapezoid(x,tau,np.abs(f))
    simpson=independent_cumulative_simpson(x,tau,f)
    alt_gap=float(np.max(np.abs(signed-alt)))
    alt_gross_gap=float(np.max(np.abs(gross-alt_gross)))
    simpson_gap=float(np.max(np.abs(signed-simpson)))
    assert alt_gap<2e-25 and alt_gross_gap<2e-25,(alt_gap,alt_gross_gap)
    assert np.max(np.abs(signed)-gross)<1e-25
    assert np.all(signed[:,0,:]==0) and np.all(gross[:,0,:]==0)
    assert np.max(np.abs(signed))>0 and np.max(np.abs(f))>0
    assert np.all(np.isfinite(signed))
    # Exact intermediate conditions: b' = 0 or E_{L+2}=0 gives zero boundary source.
    names=list(map(str,cases));ids={s:i for i,s in enumerate(names)}
    assert len(set(names))==50 and all(n.startswith(('s00','s04')) for n in names)
    pair_records=[]
    for m in range(5):
        for r in range(5):
            c0=ids[f's00_m{m:02d}_r{r:02d}'];c4=ids[f's04_m{m:02d}_r{r:02d}']
            row={'pair':f'm{m:02d}_r{r:02d}'}
            for j,L in enumerate(CUT):
                row.update({f'delta_memory_L{L}_end':float(signed[c4,-1,j]-signed[c0,-1,j]),
                            f'delta_abs_memory_L{L}_end':float(gross[c4,-1,j]-gross[c0,-1,j]),
                            f'delta_flux_L{L}_end':float(f[c4,-1,j]-f[c0,-1,j])})
            pair_records.append(row)
    end_records=[]
    for i,name in enumerate(names):
        row={'case':name,'source_PR145':SRC_PR145_HEAD,'final_tau':float(tau[i,-1])}
        for j,L in enumerate(CUT):
            row.update({f'memory_L{L}_end':float(signed[i,-1,j]),
                        f'gross_L{L}_end':float(gross[i,-1,j]),
                        f'max_abs_memory_L{L}':float(np.max(abs(signed[i,:,j]))),
                        f'flux_L{L}_end':float(f[i,-1,j])})
        end_records.append(row)
    mean_rows=[]
    for k,v in enumerate(x):
        row={'x':float(v)}
        for j,L in enumerate(CUT):
            row.update({f'mean_memory_L{L}':float(np.mean(signed[:,k,j])),
                        f'mean_abs_memory_L{L}':float(np.mean(np.abs(signed[:,k,j]))),
                        f'mean_gross_L{L}':float(np.mean(gross[:,k,j]))})
        mean_rows.append(row)
    np.savez_compressed(out/'SOURCE_FROZEN_BOUNDARY_MEMORY_50x193.npz',cases=cases,x=x,cutoffs=np.asarray(CUT),tau=tau,
                        original_flux=f,signed_memory=signed,gross_memory=gross,simpson_reference=simpson)
    emit_csv(out/'ENDPOINT_50_BOUNDARY_MEMORY.csv',end_records)
    emit_csv(out/'MATCHED_25_BOUNDARY_MEMORY.csv',pair_records)
    emit_csv(out/'TIME193_BOUNDARY_MEMORY_MEANS.csv',mean_rows)
    def family(q):return np.array([i for i,s in enumerate(names) if s.startswith(q)],dtype=int)
    s00=family('s00');s04=family('s04')
    summary={
      'schema':'vigilode.bass21.eplus_source_frozen_boundary_memory_v1',
      'interpretation':'Direct source-grid Duhamel-weighted boundary-source diagnostics, NOT full E_l truncation error, not owner-native J/E IVP.',
      'source_full_sha256':SOURCE_SHA,'source_parent_sha256':PARENT_SHA,
      'owner_repository':OWNER_REPO,'owner_commit':OWNER_COMMIT,
      'pr145_head':SRC_PR145_HEAD,'cases':50,'epochs':193,'cutoffs':list(CUT),
      'input_e_count':50*193*12,'input_flux_count':50*193*4,
      'new_signed_memory_values':50*193*4,'new_gross_memory_values':50*193*4,
      'new_ivps':0,'new_owner_rhs_evolutions':0,
      'x_start':float(x[0]),'x_end':float(x[-1]),'tau_min':float(tau.min()),'tau_max':float(tau.max()),
      'max_input_flux_reconstruction_gap':max_flux_gap,
      'max_independent_recurrence_gap':alt_gap,
      'max_independent_gross_recurrence_gap':alt_gross_gap,
      'max_trapezoid_vs_simpson_difference':simpson_gap,
      'max_abs_signed_memory_per_L':np.max(np.abs(signed),axis=(0,1)).tolist(),
      'max_gross_memory_per_L':np.max(gross,axis=(0,1)).tolist(),
      'max_abs_signed_endpoint_per_L':np.max(np.abs(signed[:,-1,:]),axis=0).tolist(),
      'mean_s04_signed_endpoint_per_L':np.mean(signed[s04,-1,:],axis=0).tolist(),
      'mean_s00_signed_endpoint_per_L':np.mean(signed[s00,-1,:],axis=0).tolist(),
      'mean_25_pairs_delta_endpoint_per_L':np.mean([ [v[f'delta_memory_L{L}_end'] for L in CUT] for v in pair_records],axis=0).tolist(),
      'median_25_pairs_abs_delta_endpoint_per_L':np.median(np.abs([[v[f'delta_memory_L{L}_end'] for L in CUT] for v in pair_records]),axis=0).tolist(),
      'mean_s04_gross_endpoint_per_L':np.mean(gross[s04,-1,:],axis=0).tolist(),
      'mean_s00_gross_endpoint_per_L':np.mean(gross[s00,-1,:],axis=0).tolist(),
      'max_abs_flux_per_L':np.max(np.abs(f),axis=(0,1)).tolist(),
      's00_l18_to_24_unresolved_gl_quadrature':'YES, inherited PR145; interpret s00 high-rank memory only as computational diagnostic',
      'official_dag':{'vigilode':'7/23','rei_central':'28/88','rei_working':'33/88','bass_ce1':'9/15','successor_admitted':'0/12','delta':0},
      'production':'HOLD','auto_next_node':False,'distinct_author_R6_C9':'NOT_RUN'
    }
    (out/'SCIENCE_SUMMARY.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
    # Two physically motivated independent scientific graphs. No manual colors.
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    plt.rcParams['svg.hashsalt']='vigilode-bass21-boundary-memory-v1'
    figdir=out.parent/'figures';figdir.mkdir(parents=True,exist_ok=True)
    fig,ax=plt.subplots(figsize=(7.2,4.6))
    for j,L in enumerate(CUT):
        ax.plot(x,np.mean(np.abs(signed[s04,:,j]),axis=0),label=f'L={L}')
    ax.set(yscale='log',xlabel=r'$x=\ln(a/a_\mathrm{ref})$',ylabel='Mean |attenuated boundary-source memory|',title='Original strong-shear BASS21 source: passive leakage memory')
    ax.legend();ax.grid(alpha=0.3);fig.tight_layout()
    fig.savefig(figdir/'S04_MEMORY_EVOLUTION.svg',metadata={'Date':None});fig.savefig(figdir/'S04_MEMORY_EVOLUTION.png',dpi=180);plt.close(fig)
    fig,ax=plt.subplots(figsize=(7.2,4.6))
    for i, L in enumerate(CUT):
        delta=np.array([v[f'delta_memory_L{L}_end'] for v in pair_records])
        ax.plot(np.arange(25),delta,marker='.',label=f'L={L}')
    ax.set(xlabel='Original matched shear pair (m,r)',ylabel='s04 − s00 signed boundary-source memory',title='25 source-matched shear contrasts at final epoch')
    ax.legend();ax.grid(alpha=0.3);fig.tight_layout()
    fig.savefig(figdir/'MATCHED25_MEMORY_DIFF.svg',metadata={'Date':None});fig.savefig(figdir/'MATCHED25_MEMORY_DIFF.png',dpi=180);plt.close(fig)
    return summary

if __name__=='__main__':
    a=argparse.ArgumentParser();a.add_argument('--source',required=True);a.add_argument('--output',required=True)
    args=a.parse_args();print(json.dumps(compute(args.source,args.output),sort_keys=True,indent=2))
