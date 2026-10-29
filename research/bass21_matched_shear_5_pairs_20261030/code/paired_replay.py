"""Frozen BASS21 193x2 initial-shear matched history: exact opacity/product decomposition.
No solve_ivp and no mutation of frozen trajectories. Original native AtomicNetwork only.
"""
import argparse
import csv
import hashlib
import json
import pathlib
import sys
import numpy as np
from scipy.integrate import cumulative_simpson,cumulative_trapezoid

ROOT=pathlib.Path(__file__).resolve().parents[1]
ARCHIVE=ROOT/'work'
sys.path.insert(0,str(ARCHIVE/'rei_pr269'/'code'))
import native_full_rate as source
PIN={
 's00_m00_r00': ('original/trajectory.npz', '37d1bab173e81449cf474b21a678077edd0150cf0b2cb386d014764580678086','57a5375c5183f83d3eb3dd094cac31a212c39de3'),
 's04_m00_r00': ('s04_m00_r00_trajectory.npz','c770ba3182af6df6e4239971cc453783622a99cbf1f5a0b967a16c124a8f7d00','528294868d295bc1ced7d62611714fbc321ba693'),
 's00_m02_r02': ('s00_m02_r02_trajectory.npz','840b19b75c6008770e4f393f198deac1b6beea55e7b351ad5f531b50cf3f1a4f','5390e0607f89e03611fe0974c53fb427e2bb7ab1'),
 's04_m02_r02': ('s04_m02_r02_trajectory.npz','53ecf9e65b6fda8c45b22c9d275288c2c77d0b0d29c198bc6c315ee51759928e','be73d39ce615683d5c285efa83d4be7b13ea2390'),
 's00_m04_r04': ('s00_m04_r04_trajectory.npz','ad33e06a96795db196ca88be0e45b785ccfa47d2adb3692f0c57b527ec6432a8','ef3162b87ef74ebc6c911e833705a924fd5fc9e5'),
 's04_m04_r04': ('s04_m04_r04_trajectory.npz','5cda620f703d167de03e0ef9776085dcea6e0bf21ddcbfd1562a1a7cf0821fb8','17bbc5d0d20a4822d4395462855ea49b6f4ac846'),
 's00_m00_r04': ('s00_m00_r04_trajectory.npz','cb707cd487efc019d28ba47f949bb6d95be7394f7097e3be38e456e9c3e232c7','3cbd7013eff59a9c3e625f3deec93ce3fc8d4c59'),
 's04_m00_r04': ('s04_m00_r04_trajectory.npz','bb6374688b3618d3ab4e0f928d71d769d03b28ad8eb843e63db03759b715d500','a478f23a84320e553bd751df1ea7044ea300baad'),
 's00_m04_r00': ('s00_m04_r00_trajectory.npz','18ad2934761f19ca35b5c478ced50fad97527592378e26d033008b9672230106','86b7a9140537f98cafaa5ed7e2e9ffda38c6ff24'),
 's04_m04_r00': ('s04_m04_r00_trajectory.npz','f8736f261228b2216212860ad86f86a1fa36105020fde05cb417341a00aa0b59','d44106dc699878a76078826074c85d93e5e09b4a'),
}
K=2.99792458e10*6.6524587051e-25/1e6 # SI m^3/s, original BASS source constant

def read_one(case):
    rel, sha, blob = PIN[case]
    p = ARCHIVE/rel if case=='s00_m00_r00' else ROOT/rel
    if not p.is_file() or hashlib.sha256(p.read_bytes()).hexdigest()!=sha:
        raise ValueError('BASS_AUTHENTICATED_INPUT_MISMATCH:'+case)
    with np.load(p,allow_pickle=False) as f:
        d={k:np.array(f[k],copy=True) for k in f.files}
    names=[str(s) for s in d['diagnostic_names']]
    diag={name:np.array(d['diagnostics'][:,j],copy=True) for j,name in enumerate(names)}
    if len(d['x'])!=193 or d['Y'].shape!=(193,21) or d['x'][0]!=0 or np.any(np.diff(d['x'])<=0):
        raise ValueError('TRAJECTORY_LAYOUT_INVALID')
    net=source.new_native_network()
    if not np.array_equal(d['q'],net.q) or not np.array_equal(d['A'],net.A):
        raise ValueError('CHARGE_OR_MASS_SOURCE_MISMATCH')
    Y=d['Y']; ne=np.empty(193);F=np.empty((193,21));J=np.empty((193,21,21));photo=np.empty(193);rr=np.empty(193);dr=np.empty(193);li=np.empty(193)
    max_saved_ne=0;max_photo_energy=0
    for j in range(193):
        r=net.evaluate(np.ascontiguousarray(Y[j]),float(diag['T_K'][j]),float(diag['uv_scale'][j]),float(diag['nb_m3'][j]))
        rates=net.rates(float(diag['T_K'][j]),float(diag['uv_scale'][j]))
        ne[j]=r['ne']; F[j]=r['dY_dt'];J[j]=r['jac'];
        photo[j]=float(net.q@r['photo_rhs']);rr[j]=float(net.q@r['rr_rhs']);dr[j]=float(net.q@r['dr_rhs'])
        li[j]=rates['photo_s'][13]
        max_saved_ne=max(max_saved_ne,abs(ne[j]-diag['ne_m3'][j]))
        max_photo_energy=max(max_photo_energy,abs(r['photo_absorbed_power_W_m3']-diag['photo_absorbed_W_m3'][j]))
    gamma=K*ne/diag['H']
    ret={'case':case,'git_blob':blob,'sha256':sha,'raw':d,'diagnostics':diag,'ne':ne,'F':F,'J':J,
         'xe':Y@net.q,'gamma':gamma,'photo':photo,'rr':rr,'dr':dr,'li':li,
         'max_ne_diff':max_saved_ne,'max_energy_diff':max_photo_energy,
         'max_gamma_diff':float(np.max(np.abs(gamma-diag['gamma_T_per_H']))),
         'max_A_defect':float(np.max(np.abs(Y@net.A-1)))}
    return ret

def run(pair='m00_r00'):
    source.source_manifest_check()
    case_A,case_B='s00_'+pair,'s04_'+pair
    a,b=read_one(case_A),read_one(case_B)
    x=a['raw']['x'];z=a['raw']['z']
    if not (np.array_equal(x,b['raw']['x']) and np.array_equal(z,b['raw']['z'])):
        raise ValueError('UNMATCHED_CLOCK_GRIDS')
    # Match the native *initial* anisotropic physical targets from the actual BASS states;
    # the shear is the only intended initial matched-pair change.
    sn=[str(v) for v in a['raw']['signed_observable_names']]
    if sn != [str(v) for v in b['raw']['signed_observable_names']]:
        raise ValueError('SIGNED_OBSERVABLE_NAMES_DIFFER')
    ia={name:i for i,name in enumerate(sn)}
    initial_a=a['raw']['signed_observables'][0];initial_b=b['raw']['signed_observables'][0]
    if (abs(initial_a[ia['S_F']])>2e-14 or
        abs(initial_b[ia['S_F']]-0.6)>2e-14 or
        abs(initial_a[ia['P_m_F']]-initial_b[ia['P_m_F']])>2e-14 or
        abs(initial_a[ia['P_r_F']]-initial_b[ia['P_r_F']])>2e-14):
        raise ValueError('INITIAL_MATCHED_SHEAR_NOT_ISOLATED')
    for k in ['T_K','uv_scale']:
        if not np.array_equal(a['diagnostics'][k],b['diagnostics'][k]):raise ValueError('BATH_PROVIDER_DIFFERS:'+k)
    H0=a['diagnostics']['H'];H1=b['diagnostics']['H'];nb0=a['diagnostics']['nb_m3'];nb1=b['diagnostics']['nb_m3']
    ne0=a['ne'];ne1=b['ne'];xe0=a['xe'];xe1=b['xe']
    inv0=1/H0;inv1=1/H1
    mean=lambda u,v:(u+v)*0.5
    # Exact two-factor polarisation, then exact baryon-density/chemical-factor split.
    chemistry=K*mean(inv0,inv1)*mean(nb0,nb1)*(xe1-xe0)
    density=K*mean(inv0,inv1)*mean(xe0,xe1)*(nb1-nb0)
    geometry=K*mean(ne0,ne1)*(inv1-inv0)
    delta_gamma=b['gamma']-a['gamma']
    residual=(chemistry+density+geometry)-delta_gamma
    tau_simps={k:cumulative_simpson(v,x=x,initial=0) for k,v in [('chemistry',chemistry),('density',density),('geometry',geometry),('total',delta_gamma)]}
    tau_traps={k:cumulative_trapezoid(v,x=x,initial=0) for k,v in [('chemistry',chemistry),('density',density),('geometry',geometry),('total',delta_gamma)]}
    tau_saved=b['diagnostics']['tau_T']-a['diagnostics']['tau_T']
    # Frozen-source charge equation in proper time, then two-factor chain rule (no new solution).
    q=a['raw']['q'];f0=a['F']@q;f1=b['F']@q
    ion_kinetic=mean(inv0,inv1)*(f1-f0)
    ion_clock=mean(f0,f1)*(inv1-inv0)
    delta_x_prime= f1*inv1-f0*inv0
    # This is a diagnostic on derivative at stored states, NOT a new integrated history.
    out={
     'schema':'vigilode.bass21.matched_shear_opacity_source.v1',
     'owner':{'repo':'cosmosapjw-quantum/bass','commit':'cc602e209e7deeb511ab2a1f3b2268d55206c43c',
              'AtomicNetwork_sha256':source.EXPECTED['BASS_ATOMIC_NETWORK.py'],'pairs':{c:{'sha256':PIN[c][1],'blob':PIN[c][2]} for c in (case_A,case_B)}},
     'definition':{'baseline':case_A+', Sigma_F=0','perturbed':case_B+', Sigma_F=0.6',
                   'matched_initial_Pm':float(initial_a[ia['P_m_F']]),'matched_initial_Pr':float(initial_a[ia['P_r_F']]),
                   'clock':'same x=ln(a/a_start), same z, proper H','model':'prescribed BASS T/UV test-fluid finite21, not real observed tau',
                   'paired_contrast':'B-A','decomposition':'K*mean(1/H)*mean(nb)*delta(xe) + K*mean(1/H)*mean(xe)*delta(nb) + K*mean(ne)*delta(1/H)',
                   'c_sigma_T_m3_s':K},
     'sources':{'source_T_identical':True,'source_UV_identical':True,'source_initial_Pm_Pr_matched':True,'source_initial_shear_A':float(initial_a[ia['S_F']]),'source_initial_shear_B':float(initial_b[ia['S_F']]),'sample_count_each':193,'AtomicNetwork_evaluate_calls':386,'new_ivps':0,
                'n_b_max_abs_diff':float(np.max(np.abs(nb1-nb0))),
                'max_source_native_gamma_mismatch':max(a['max_gamma_diff'],b['max_gamma_diff']),
                'max_saved_ne_mismatch':max(a['max_ne_diff'],b['max_ne_diff']),
                'max_saved_photo_energy_mismatch':max(a['max_energy_diff'],b['max_energy_diff']),
                'max_baryon_defect':max(a['max_A_defect'],b['max_A_defect']),
                'LiI_photo_rate_max_abs_diff':float(np.max(np.abs(b['li']-a['li']))),
                'max_electron_fraction_abs_diff':float(np.max(np.abs(xe1-xe0)))},
     'check':{'max_pointwise_exact_decomposition_residual':float(np.max(np.abs(residual))),
              'max_derivative_electron_split_residual':float(np.max(np.abs(ion_kinetic+ion_clock-delta_x_prime))),
              'max_integral_decomposition_residual':float(np.max(np.abs(sum(tau_simps[k] for k in ['chemistry','density','geometry'])-tau_simps['total']))),
              'max_simpson_vs_stored_delta_tau':float(np.max(np.abs(tau_simps['total']-tau_saved)))},
     'saved':{'tau_A_end':float(a['diagnostics']['tau_T'][-1]),'tau_B_end':float(b['diagnostics']['tau_T'][-1]),'delta_tau_stored_end':float(tau_saved[-1]),
              'xe_A_end':float(xe0[-1]),'xe_B_end':float(xe1[-1]),'delta_xe_end':float(xe1[-1]-xe0[-1])},
     'contributions_simpson':{k:float(v[-1]) for k,v in tau_simps.items()},
     'contributions_trapezoid':{k:float(v[-1]) for k,v in tau_traps.items()},
     'electron_fraction_source':{'kinetic_dxend':float(ion_kinetic[-1]),'clock_dxend':float(ion_clock[-1]),
         'pointwise_difference_end':float(delta_x_prime[-1]),
         'integral_kinetic_simpson':float(cumulative_simpson(ion_kinetic,x=x,initial=0)[-1]),
         'integral_clock_simpson':float(cumulative_simpson(ion_clock,x=x,initial=0)[-1])},
     'quartile_integrals':[],
     'claimed_scope':{'exact_instantaneous_split':'DERIVED_AND_NUMERICALLY_CHECKED',
                      'integrated_matched_193_source':'NUMERICALLY_CHECKED_POSTPROCESS',
                      'physical_causal_attribution':'NOT_ESTABLISHED',
                      'independent_R6_C9':'NOT_RUN','native_RODAS':'NOT_RUN',
                      'production':'HOLD','REI_cross_owner_physical':'NOT_ADMITTED'},
     'DAG':{'VigilODE':'7/23 unchanged','REI_central':'28/88 unchanged','REI_working':'33/88 unchanged','successor':'0/12 admitted','new_auxiliary':'1/1 scoped conditional evidence'}
    }
    for l,r in [(0,64),(64,128),(128,192)]:
        out['quartile_integrals'].append({'index_lo':l,'index_hi':r,'z_hi':float(z[l]),'z_lo':float(z[r]),
          **{k:float(v[r]-v[l]) for k,v in tau_simps.items()}})
    rows=[]
    for i in range(193):
        d={'index':i,'x':float(x[i]),'z':float(z[i]),'ne_A_m3':float(ne0[i]),'ne_B_m3':float(ne1[i]),
           'xe_A':float(xe0[i]),'xe_B':float(xe1[i]),'H_A_s':float(H0[i]),'H_B_s':float(H1[i]),
           'n_b_A_m3':float(nb0[i]),'n_b_B_m3':float(nb1[i]),
           'gamma_A':float(a['gamma'][i]),'gamma_B':float(b['gamma'][i]),'delta_gamma':float(delta_gamma[i]),
           'gamma_delta_chemical':float(chemistry[i]),'gamma_delta_density':float(density[i]),'gamma_delta_geometry':float(geometry[i]),
           'tau_A_saved':float(a['diagnostics']['tau_T'][i]),'tau_B_saved':float(b['diagnostics']['tau_T'][i]),
           'tau_delta_saved':float(tau_saved[i]),'delta_xe_saved':float(xe1[i]-xe0[i]),
           'delta_dxe_dx_kinetic':float(ion_kinetic[i]),'delta_dxe_dx_clock':float(ion_clock[i]),
           'B_LiI_photo_rate_s':float(b['li'][i]),'A_LiI_photo_rate_s':float(a['li'][i]),
           'source_photo_A_per_baryon_s':float(a['photo'][i]),'source_photo_B_per_baryon_s':float(b['photo'][i]),
           'source_RR_A_per_baryon_s':float(a['rr'][i]),'source_RR_B_per_baryon_s':float(b['rr'][i])}
        d.update({f'tau_simpson_{k}':float(v[i]) for k,v in tau_simps.items()})
        rows.append(d)
    return out,rows

def write():
    OUT=ROOT/'results';OUT.mkdir(exist_ok=True)
    all_summary=[]
    for pair in ['m00_r00','m02_r02','m04_r04','m00_r04','m04_r00']:
        summary,rows=run(pair)
        (OUT/f'MATCHED_SHEAR_DECOMPOSITION_{pair}.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2,allow_nan=False)+'\n')
        with (OUT/f'MATCHED_193_EPOCH_DECOMPOSITION_{pair}.csv').open('w',newline='') as f:
            w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
        all_summary.append({'pair':pair,'source_sha256':summary['owner']['pairs'],'stored_delta_tau':summary['saved']['delta_tau_stored_end'],
            'simpson_delta_tau':summary['contributions_simpson']['total'],
            'geometry':summary['contributions_simpson']['geometry'],'ionization':summary['contributions_simpson']['chemistry'],
            'density':summary['contributions_simpson']['density'],
            'geometry_share':summary['contributions_simpson']['geometry']/summary['contributions_simpson']['total'],
            'max_absolute_delta_xe':summary['sources']['max_electron_fraction_abs_diff'],
            'endpoint_delta_xe':summary['saved']['delta_xe_end'],
            'pointwise_reconstruction_error':summary['check']['max_pointwise_exact_decomposition_residual']})
    info={'schema':'vigilode.bass21.matched_shear_five_pairs.v1','source':'original BASS source owner only','n_pairs':5,
          'n_histories':10,'n_epochs_total':1930,'native_evaluate_calls':1930,'new_ivps':0,
          'pairs':all_summary,'n_baseline_original_cases':127,'original_VigilODE_DAG':'7/23 unchanged',
          'original_REI_central':'28/88 unchanged','auxiliary_scoped':'1/1 conditional data only','production':'HOLD'}
    (OUT/'FIVE_PAIRS_SUMMARY.json').write_text(json.dumps(info,ensure_ascii=False,indent=2,allow_nan=False)+'\n')
    print(json.dumps(info,ensure_ascii=False,indent=2,allow_nan=False))

if __name__=='__main__':
    cli=argparse.ArgumentParser();cli.add_argument('--write',action='store_true');o=cli.parse_args()
    if o.write: write()
    else: print(json.dumps(run()[0],indent=2))
