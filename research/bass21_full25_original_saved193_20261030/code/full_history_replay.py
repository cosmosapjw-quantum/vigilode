"""Exact-source original BASS21 50 x 193 saved-state replay; NEVER advance an IVP.

Native AtomicNetwork.evaluate gives proper-time F and J at immutable saved states;
postprocess forward BASS clock x=ln(a/a_start), do not simulate new states.
"""
from __future__ import annotations
import csv, hashlib, json, pathlib, sys
import numpy as np
from scipy.integrate import cumulative_simpson, cumulative_trapezoid
ROOT=pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'source'/'previous_PR121'/'rei_pr269'/'code'))
import native_full_rate as source
K=2.99792458e10*6.6524587051e-25/1e6 # c sigma_T (m^3/s); original BASS convention
PAIR_NAMES=[f'm{m:02d}_r{r:02d}' for m in range(5) for r in range(5)]
N=193

def load_case(c,net):
    p=ROOT/'inputs'/(c+'.npz')
    mf=json.loads((ROOT/'results'/'SOURCE_INPUT_MANIFEST.json').read_text())['sources'][c]
    raw=p.read_bytes(); assert hashlib.sha256(raw).hexdigest()==mf['sha256'],('sha',c)
    assert hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\x00'+raw).hexdigest()==mf['git_blob'],('git blob',c)
    with np.load(p,allow_pickle=False) as z: d={k:np.array(z[k],copy=True) for k in z.files}
    assert (d['x'].shape==(N,) and d['Y'].shape==(N,21) and d['raw_state'].shape==(N,62) and
            d['diagnostics'].shape[0]==N and d['z'].shape==(N,)),c
    diag={str(s):np.array(d['diagnostics'][:,j],copy=True) for j,s in enumerate(d['diagnostic_names'])}
    assert np.array_equal(d['q'],net.q) and np.array_equal(d['A'],net.A) and np.array_equal(d['species_names'],np.asarray(net.names))
    assert np.array_equal(d['x'],np.linspace(0,np.log(7),N)) or (d['x'][0]==0 and np.all(np.diff(d['x'])>0))
    assert np.all(np.isfinite(d['Y'])) and np.all(diag['H']>0) and np.all(diag['nb_m3']>0)
    Y=d['Y'];xe=Y@net.q;F=np.empty((N,21));J=np.empty((N,21,21));ne=np.empty(N)
    p_e=np.empty(N);rr_e=np.empty(N);dr_e=np.empty(N);abs_pw=np.empty(N);prim_pw=np.empty(N)
    # H1I, He3I, He3II, He4I, He4II, Li6I, Li7I source-native rates, s^-1
    idx=(1,7,8,10,11,13,17)
    gamma_photo=np.empty((N,len(idx)))
    for i in range(N):
        temp,uv,nb=float(diag['T_K'][i]),float(diag['uv_scale'][i]),float(diag['nb_m3'][i])
        out=net.evaluate(np.ascontiguousarray(Y[i]),temp,uv,nb)
        rates=net.rates(temp,uv)
        F[i]=out['dY_dt'];J[i]=out['jac'];ne[i]=out['ne']
        p_e[i]=net.q@out['photo_rhs'];rr_e[i]=net.q@out['rr_rhs'];dr_e[i]=net.q@out['dr_rhs']
        abs_pw[i]=out['photo_absorbed_power_W_m3'];prim_pw[i]=out['primary_kinetic_power_W_m3']
        gamma_photo[i]=rates['photo_s'][list(idx)]
    gamma=K*ne/diag['H']
    return dict(case=c,raw=d,diag=diag,xe=xe,ne=ne,F=F,J=J,photo_e=p_e,rr_e=rr_e,dr_e=dr_e,
                abs_pw=abs_pw,prim_pw=prim_pw,photo_rates=gamma_photo,gamma=gamma,
                max_ne_defect=float(np.max(np.abs(ne-diag['ne_m3']))),
                max_gamma_defect=float(np.max(np.abs(gamma-diag['gamma_T_per_H']))),
                max_photo_power_defect=float(np.max(np.abs(abs_pw-diag['photo_absorbed_W_m3']))),
                max_baryon_defect=float(np.max(np.abs(Y@net.A-1))))

def pol(A,B):
    x,z=A['raw']['x'],A['raw']['z']
    assert np.array_equal(x,B['raw']['x']) and np.array_equal(z,B['raw']['z']),'CLOCK_MISMATCH'
    for k in ['T_K','uv_scale','nb_m3']:
        assert np.array_equal(A['diag'][k],B['diag'][k]),'BATH_DIFFERENCE_'+k
    sn=[str(s) for s in A['raw']['signed_observable_names']]
    ia={n:i for i,n in enumerate(sn)}
    a0=A['raw']['signed_observables'][0];b0=B['raw']['signed_observables'][0]
    assert abs(a0[ia['S_F']])<2e-14 and abs(b0[ia['S_F']]-0.6)<2e-14
    for key in ['P_m_F','P_r_F']:
        assert abs(a0[ia[key]]-b0[ia[key]])<2e-14,key
    av=lambda a,b:(a+b)*.5
    V0,V1=1/A['diag']['H'],1/B['diag']['H']
    n0,n1=A['diag']['nb_m3'],B['diag']['nb_m3']
    xe0,xe1=A['xe'],B['xe'];ne0,ne1=A['ne'],B['ne']
    terms={
        'chemistry':K*av(V0,V1)*av(n0,n1)*(xe1-xe0),
        'density':K*av(V0,V1)*av(xe0,xe1)*(n1-n0),
        'geometry':K*av(ne0,ne1)*(V1-V0),
        'total':B['gamma']-A['gamma']}
    pointwise=sum(terms[k] for k in ['chemistry','density','geometry'])-terms['total']
    sims={k:cumulative_simpson(v,x=x,initial=0) for k,v in terms.items()}
    traps={k:cumulative_trapezoid(v,x=x,initial=0) for k,v in terms.items()}
    tau_saved=B['diag']['tau_T']-A['diag']['tau_T']
    q=A['raw']['q'];f0=A['F']@q;f1=B['F']@q
    kinetic=av(V0,V1)*(f1-f0);clock=av(f0,f1)*(V1-V0)
    row={'pair':A['case'][4:], 'matter_axis':float(a0[ia['P_m_F']]),'radiation_axis':float(a0[ia['P_r_F']]),
       'tau_A_stored':float(A['diag']['tau_T'][-1]),'tau_B_stored':float(B['diag']['tau_T'][-1]),
       'delta_tau_stored':float(tau_saved[-1]),'delta_tau_simpson':float(sims['total'][-1]),
       'delta_tau_trapezoid':float(traps['total'][-1]),
       'geometry':float(sims['geometry'][-1]),'chemistry':float(sims['chemistry'][-1]),'density':float(sims['density'][-1]),
       'geometry_fraction':float(sims['geometry'][-1]/sims['total'][-1]),
       'chemistry_fraction':float(sims['chemistry'][-1]/sims['total'][-1]),
       'max_absolute_delta_xe':float(np.max(np.abs(xe1-xe0))),
       'final_delta_xe':float(xe1[-1]-xe0[-1]),
       'max_pointwise_decomposition_residual':float(np.max(np.abs(pointwise))),
       'max_cumulative_decomposition_residual':float(np.max(np.abs(sum(sims[k] for k in ['chemistry','geometry','density'])-sims['total']))),
       'max_simpson_vs_saved':float(np.max(np.abs(sims['total']-tau_saved))),
       'final_simpson_vs_saved':float(sims['total'][-1]-tau_saved[-1]),
       'max_electron_source_derivative_split':float(np.max(np.abs((kinetic+clock)-(f1*V1-f0*V0)))),
       'max_H_delta':float(np.max(np.abs(B['diag']['H']-A['diag']['H']))),
       'max_nb_delta':float(np.max(np.abs(n1-n0))),
       'mean_source_photo_e_A_m3_s':float(np.mean(n0*A['photo_e'])),
       'mean_source_photo_e_B_m3_s':float(np.mean(n1*B['photo_e'])),
       'mean_source_RR_e_A_m3_s':float(np.mean(n0*A['rr_e'])),
       'mean_source_RR_e_B_m3_s':float(np.mean(n1*B['rr_e']))}
    boundaries=[0,48,96,144,192]
    for j,(low,hi) in enumerate(zip(boundaries[:-1],boundaries[1:])):
        row[f'window{j}_z_start']=float(z[low]);row[f'window{j}_z_end']=float(z[hi]);
        for k in ['chemistry','density','geometry','total']:
            row[f'window{j}_{k}']=float(sims[k][hi]-sims[k][low])
    history=[]
    for i in range(N):
        history.append(dict(pair=row['pair'],epoch=i,x=float(x[i]),z=float(z[i]),
          H_A_s_inv=float(A['diag']['H'][i]),H_B_s_inv=float(B['diag']['H'][i]),
          xe_A=float(xe0[i]),xe_B=float(xe1[i]),ne_A_m3=float(ne0[i]),ne_B_m3=float(ne1[i]),
          gamma_A=float(A['gamma'][i]),gamma_B=float(B['gamma'][i]),
          tau_A_saved=float(A['diag']['tau_T'][i]),tau_B_saved=float(B['diag']['tau_T'][i]),
          delta_tau_saved=float(tau_saved[i]),delta_gamma=float(terms['total'][i]),
          gamma_chemistry=float(terms['chemistry'][i]),gamma_geometry=float(terms['geometry'][i]),gamma_density=float(terms['density'][i]),
          **{f'tau_simpson_{k}':float(sims[k][i]) for k in sims},
          **{f'tau_trapezoid_{k}':float(traps[k][i]) for k in traps}))
    return row,history

def to_csv(path, rows):
    with path.open('w',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)

def run():
    source.source_manifest_check()
    net=source.new_native_network()
    cases={};allchem=[];fjs=[]
    for s in [0,4]:
      for m in range(5):
       for r in range(5):
        name=f's{s:02d}_m{m:02d}_r{r:02d}'
        a=load_case(name,net);cases[name]=a;fjs.append((a['F'],a['J']))
        for k in range(N):
            d=a['diag'];Y=a['raw']['Y'][k]
            allchem.append(dict(case=name,epoch=k,x=float(a['raw']['x'][k]),z=float(a['raw']['z'][k]),
              H_s_inv=float(d['H'][k]),nb_m3=float(d['nb_m3'][k]),ne_m3=float(a['ne'][k]),xe=float(a['xe'][k]),
              T_K=float(d['T_K'][k]),uv_scale=float(d['uv_scale'][k]),gamma_T_per_H=float(a['gamma'][k]),tau_T_saved=float(d['tau_T'][k]),
              photo_e_source_m3_s=float(d['nb_m3'][k]*a['photo_e'][k]),
              RR_e_source_m3_s=float(d['nb_m3'][k]*a['rr_e'][k]),
              DR_e_source_m3_s=float(d['nb_m3'][k]*a['dr_e'][k]),
              net_e_source_m3_s=float(d['nb_m3'][k]*(a['F'][k]@net.q)),
              photo_absorbed_W_m3=float(a['abs_pw'][k]),
              photo_primary_electron_W_m3=float(a['prim_pw'][k]),
              photo_HI_s_inv=float(a['photo_rates'][k,0]),photo_He3I_s_inv=float(a['photo_rates'][k,1]),
              photo_He3II_s_inv=float(a['photo_rates'][k,2]),photo_He4I_s_inv=float(a['photo_rates'][k,3]),
              photo_He4II_s_inv=float(a['photo_rates'][k,4]),photo_Li6I_s_inv=float(a['photo_rates'][k,5]),photo_Li7I_s_inv=float(a['photo_rates'][k,6]),
              abundance_HII=float(Y[2]),abundance_He4II=float(Y[11]),abundance_Li7I=float(Y[17])))
    rows=[];hist=[]
    for pair in PAIR_NAMES:
        row,tab=pol(cases['s00_'+pair],cases['s04_'+pair]);rows.append(row);hist.extend(tab)
    res=ROOT/'results';res.mkdir(exist_ok=True)
    to_csv(res/'ALL_50_NATIVE_193_CHEMISTRY.csv',allchem)
    to_csv(res/'MATCHED_25_PAIR_SUMMARY.csv',rows)
    to_csv(res/'MATCHED_25_X_193_OPTICAL_DEPTH.csv',hist)
    # All F and J in stable source-name order, original saved states, no new ODE paths.
    np.savez_compressed(res/'ALL_50_NATIVE_FJ.npz',names=np.array(sorted(cases)),
        F=np.stack([cases[n]['F'] for n in sorted(cases)]),
        J=np.stack([cases[n]['J'] for n in sorted(cases)]),
        photo_source=np.stack([cases[n]['photo_e'] for n in sorted(cases)]),
        rr_source=np.stack([cases[n]['rr_e'] for n in sorted(cases)]),
        dr_source=np.stack([cases[n]['dr_e'] for n in sorted(cases)]),
        q=net.q,A=net.A)
    ge=np.array([a['geometry_fraction'] for a in rows]); chem=np.array([a['chemistry_fraction'] for a in rows]);
    taum=np.array([a['delta_tau_stored'] for a in rows]);
    rechecks={k:max(c[k] for c in cases.values()) for k in ('max_ne_defect','max_gamma_defect','max_photo_power_defect','max_baryon_defect')}
    legacy=json.loads((ROOT/'source'/'previous_PR122'/'results'/'FIVE_PAIRS_SUMMARY.json').read_text())
    five={p['pair']:p for p in legacy['pairs']}
    legacy_errors={p:{key:float(next(a for a in rows if a['pair']==p)[new]-d[old]) for key,(new,old) in {
       'stored_tau':('delta_tau_stored','stored_delta_tau'),
       'simpson_tau':('delta_tau_simpson','simpson_delta_tau'),
       'geometry':('geometry','geometry'),
       'ionization':('chemistry','ionization')}.items()} for p,d in five.items()}
    report={'schema':'vigilode.bass21.25_pair_full_saved_history.v1',
        'owner':'cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c',
        'new_ivps':0,'native_original_evaluate_calls':len(allchem),
        'n_trajectories':len(cases),'n_pairs':len(rows),'epochs_per_case':N,
        'science':{'all_delta_tau_negative':bool(np.all(taum<0)),
           'stored_delta_tau_range':[float(np.min(taum)),float(np.max(taum))],
           'stored_delta_tau_mean':float(np.mean(taum)),
           'geometry_share_range':[float(np.min(ge)),float(np.max(ge))],
           'geometry_share_mean':float(np.mean(ge)),
           'chemistry_share_range':[float(np.min(chem)),float(np.max(chem))],
           'chemistry_share_mean':float(np.mean(chem)),
           'max_electron_fraction_abs_diff':max(a['max_absolute_delta_xe'] for a in rows),
           'max_nb_delta':max(a['max_nb_delta'] for a in rows),
           'max_source_vs_saved_Simpson_diff':max(a['max_simpson_vs_saved'] for a in rows),
           'max_abs_rational_pointwise_residual_float':max(a['max_pointwise_decomposition_residual'] for a in rows),
           'max_abs_cumulative_split_residual':max(a['max_cumulative_decomposition_residual'] for a in rows),
           'max_electron_source_derivative_split':max(a['max_electron_source_derivative_split'] for a in rows),
           'sums_by_window':{str(i):{'geo_mean':float(np.mean([r[f'window{i}_geometry'] for r in rows])),
                  'chem_mean':float(np.mean([r[f'window{i}_chemistry'] for r in rows]))} for i in range(4)}},
        'validation':{'source_manifest':source.source_manifest_check(), 'saved_state_defects':rechecks,'previous_five_pair_deltas':legacy_errors},
        'not_claimed':['causal H vs chemistry independence','CMB directional scattering','observational tau','solver-global BDF convergence','physical equivalence of BASS/REI17/P19','independent R6 or C9','production readiness'],
        'DAG':{'VigilODE':'7/23','REI_central':'28/88','REI_working':'33/88','successor':'0/12','auxiliary_complete':'1/1 scoped source-native saved-time evidence','original_node_delta':0}}
    (res/'SCIENCE_SUMMARY.json').write_text(json.dumps(report,sort_keys=True,indent=2,allow_nan=False)+'\n')
    print(json.dumps(report,sort_keys=True,indent=2,allow_nan=False))
    return report
if __name__=='__main__':run()
