"""BASS21 193-sample source-owned *saved history* chemistry/opacity replay.

This script NEVER calls solve_ivp, advances the 62-state system, or mutates the
source arrays. It loads the byte-exact 193-epoch BASS trajectory at its
original owner commit; re-evaluates BASS21 AtomicNetwork at each frozen epoch.
Three different meanings are kept distinct: a saved source RHS, a sampled
postprocessing quadrature, and the original BDF cumulative tau state.
"""
from __future__ import annotations
import argparse
import csv
import hashlib
import json
import math
import platform
import sys
from pathlib import Path
import numpy as np
from scipy.integrate import cumulative_trapezoid, cumulative_simpson

REPLAY=Path(__file__).resolve().parents[1]
BASE=REPLAY.parent
sys.path.insert(0,str(BASE/'rei_pr269'/'code'))
import native_full_rate as src

BASS_COMMIT='cc602e209e7deeb511ab2a1f3b2268d55206c43c'
SOURCE_GIT_BLOB='57a5375c5183f83d3eb3dd094cac31a212c39de3'
SOURCE_SHA256='37d1bab173e81449cf474b21a678077edd0150cf0b2cb386d014764580678086'
SOURCE_BYTES=254301
PARENT_SHA256='ba3f07a4f881ae8b02fd7b9690242ee29cfd1ae9f2b885b2601487010408508b'
ORIGINAL_TAU_FINAL=0.06934076449489847
# 2.99792458e10 cm s^-1 * 6.6524587051e-25 cm^2 / 1e6 cm^3 m^-3
THOMSON_M3_PER_S=2.99792458e10*6.6524587051e-25/1e6
SPECIES={'HI':1,'He3I':7,'He3II':8,'He4I':10,'He4II':11,'Li6I':13,'Li6II':14,'Li6III':15,'Li7I':17,'Li7II':18,'Li7III':19}

class SourceError(RuntimeError):
    pass

def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def verify_sources():
    p=BASE/'original'/'trajectory.npz'
    if not p.exists() or p.stat().st_size!=SOURCE_BYTES or sha256(p)!=SOURCE_SHA256:
        raise SourceError('BYTE_IDENTITY_FAILED: original trajectory differs')
    a=BASE/'original'/'PR118_SOURCE_FULL.zip'
    if not a.exists() or sha256(a)!=PARENT_SHA256:
        raise SourceError('SOURCE_ARCHIVE_IDENTITY_FAILED: prior PR118 archive differs')
    native_hashes=src.source_manifest_check()
    return p,native_hashes

def compute():
    path, source_hashes=verify_sources()
    with np.load(path,allow_pickle=False) as loaded:
        source={name:loaded[name].copy() for name in loaded.files}
    n=193
    x=source['x'];z=source['z'];Y=source['Y'];diag=source['diagnostics']
    names=[str(s) for s in source['diagnostic_names']]
    nd={s:i for i,s in enumerate(names)}
    fields={s:diag[:,j] for s,j in nd.items()}
    assert x.shape==(n,) and Y.shape==(n,21) and diag.shape==(n,25)
    assert x[0]==0 and np.all(np.diff(x)>0) and np.all(np.diff(z)<0)
    assert np.all(np.isfinite(diag)) and np.all(np.isfinite(Y)) and np.min(Y)>=0
    net=src.new_native_network()
    if not (np.array_equal(net.q,source['q']) and np.array_equal(net.A,source['A'])):
        raise SourceError('SOURCE_SPECIES_MISMATCH')
    assert all(str(a)==str(b) for a,b in zip(net.species_names,source['species_names'])) if hasattr(net,'species_names') else True
    # BASS native compact33/full62 state source. No fresh IVP/lift.
    state_replay_error=float(np.max(np.abs(source['raw_state62'][:,40:61]*source['isotope_scales']-Y)))
    z_replay_error=float(np.max(np.abs(21*np.exp(-x)-1-z)))
    A=net.A;q=net.q
    mass=Y@A;xe=Y@q
    photo_e=np.empty(n);rr_e=np.empty(n);dr_e=np.empty(n);decay_e=np.empty(n)
    net_chem=np.empty(n);ne_eval=np.empty(n);F=np.empty((n,21));J=np.empty((n,21,21))
    photo_energy=np.empty(n);primary_energy=np.empty(n);core_energy=np.empty(n);uv_energy=np.empty(n)
    li1s_event=np.empty(n);rate={nm:np.empty(n) for nm in SPECIES}
    max_A_F=0.;max_A_J=0.;max_F_def=0.;max_J_def=0.;max_evt=0.;max_e_src=0.
    T=fields['T_K'];uv=fields['uv_scale'];nb=fields['nb_m3'];H=fields['H'];ne=fields['ne_m3']
    for i in range(n):
        y=Y[i]
        ev=net.evaluate(y,float(T[i]),float(uv[i]),float(nb[i]))
        r=net.rates(float(T[i]),float(uv[i]))
        ne_eval[i]=ev['ne']
        F[i]=ev['dY_dt'];J[i]=ev['jac']
        photo_e[i]=nb[i]*float(q@ev['photo_rhs'])
        rr_e[i]=nb[i]*float(q@ev['rr_rhs'])
        dr_e[i]=nb[i]*float(q@ev['dr_rhs'])
        decay_e[i]=nb[i]*float(q@ev['decay_rhs'])
        net_chem[i]=ev['electron_source_m3_s']
        photo_energy[i]=ev['photo_absorbed_power_W_m3']
        primary_energy[i]=ev['primary_kinetic_power_W_m3']
        core_energy[i]=ev['unresolved_core_power_W_m3']
        uv_energy[i]=ev['uv_energy_density_J_m3']
        li1s_event[i]=ev['li1s_photo_event_rate_m3_s']
        for nm,j in SPECIES.items(): rate[nm][i]=r['photo_s'][j]
        max_A_F=max(max_A_F,abs(A@F[i]))
        max_A_J=max(max_A_J,float(np.max(np.abs(A@J[i]))))
        max_F_def=max(max_F_def,float(np.max(np.abs((r['G0']+r['D']+ev['ne']*r['Gne'])@y-F[i]))))
        J_exp=r['G0']+r['D']+ev['ne']*r['Gne']+nb[i]*np.outer(r['Gne']@y,q)
        max_J_def=max(max_J_def,float(np.max(np.abs(J_exp-J[i]))))
        max_evt=max(max_evt,abs(photo_e[i]-nb[i]*sum(y[j]*r['photo_s'][j] for j in net.photo_indices)))
        max_e_src=max(max_e_src,abs(net_chem[i]-photo_e[i]-rr_e[i]-dr_e[i]-decay_e[i]))
    tau_x=THOMSON_M3_PER_S*ne_eval/H
    tau_native=fields['gamma_T_per_H'];tau_stored=fields['tau_T']
    tau_trap=cumulative_trapezoid(tau_x,x=x,initial=0.)
    tau_simp=cumulative_simpson(tau_x,x=x,initial=0.)
    elapsed_trap=cumulative_trapezoid(1/H,x=x,initial=0.)
    elapsed_simp=cumulative_simpson(1/H,x=x,initial=0.)
    dx_e_chem=(q@F.T)/H
    xe_trap=xe[0]+cumulative_trapezoid(dx_e_chem,x=x,initial=0)
    xe_simp=xe[0]+cumulative_simpson(dx_e_chem,x=x,initial=0)
    # Rates integrated over original proper clock; model-predicted chemical
    # event exposure per baryon, NOT net generated astrophysical photon counts.
    events={}
    for nm,arr in [('photo',photo_e),('rr',-rr_e),('dr',-dr_e)]:
        events[nm]=float(cumulative_simpson(arr/(nb*H),x=x,initial=0)[-1])
    photon_power_total=float(cumulative_simpson(photo_energy/H,x=x,initial=0)[-1])
    native_energy_diff={ 'photo_absorbed_W_m3':float(np.max(np.abs(photo_energy-fields['photo_absorbed_W_m3']))), 'primary_photoelectron_W_m3':float(np.max(np.abs(primary_energy-fields['primary_photoelectron_W_m3']))), 'unresolved_core_W_m3':float(np.max(np.abs(core_energy-fields['unresolved_core_W_m3']))), 'uv_energy_J_m3':float(np.max(np.abs(uv_energy-fields['uv_energy_J_m3']))), 'li1s_events_m3_s':float(np.max(np.abs(li1s_event-fields['li1s_events_m3_s'])))}
    ratio=lambda a,b: float(a/b) if b!=0 else None
    d_tau=np.abs(tau_simp-tau_stored);m=int(np.argmax(d_tau))
    dx_e_simp_err=float(xe_simp[-1]-xe[-1])
    out={
      'schema':'vigilode.bass21.saved193.original-source-replay.v1',
      'run_kind':'POSTPROCESS_FROZEN_SAVED_HISTORY_NO_IVP',
      'source':{'repo':'cosmosapjw-quantum/bass','commit':BASS_COMMIT,'trajectory_git_blob':SOURCE_GIT_BLOB,
                'trajectory_bytes':SOURCE_BYTES,'trajectory_sha256':SOURCE_SHA256,
                'original_RESULT_json_git_blob':'4ee306f3af3ef5391d6309794cccfa1459067441',
                'parent_PR118_sha256':PARENT_SHA256,
                'original_BASS_network_sha256':source_hashes['BASS_ATOMIC_NETWORK.py'],
                'original_BASS_model_sha256':source_hashes['BASS_ATOMIC_MODEL.json'],
                'coefficient_status':'129/129 binary64 projected; raw original coefficient JSON byte identity NOT_VERIFIED'},
      'clock':{'x':'ln(a/a_start)','a_start':1/21,'x_start':float(x[0]),'x_end':float(x[-1]),
               'z_start':float(z[0]),'z_end':float(z[-1]),'proper_time':'elapsed_s; dt/dx=1/H',
               'H':'BASS stored geometry mean expansion H s^-1; no FLRW substitution',
               'tau':'cumulative from initial x=0; NOT past light cone from today',
               'original_method':'BASS original solve_ivp BDF, t_eval=193, rtol=2e-8, geometry_atol=2e-11, atomic_atol=2e-10, tau_atol=2e-11, max_step=0.025',
               'source_xz_max_abs':z_replay_error},
      'counts':{'source_epochs':n,'AtomicNetwork_evaluate_calls':n,'native_F_21':n,
                'native_J_21x21':n,'new_IVPs':0},
      'source_invariants':{'max_abs_Y_raw62_scales':state_replay_error,
                           'min_Y':float(Y.min()),'max_baryon_defect':float(np.max(np.abs(mass-1))),
                           'max_ne_diff_m3':float(np.max(np.abs(ne_eval-ne))),
                           'max_xe_diff':float(np.max(np.abs(ne_eval/nb-xe))),
                           'max_H_negative':int(np.sum(H<=0)),
                           'max_A_F_per_s':max_A_F,'max_A_J_per_s':max_A_J,
                           'max_F_defect_per_s':max_F_def,'max_J_defect_per_s':max_J_def,
                           'max_photo_event_charge_m3_s':max_evt,
                           'max_e_source_decomposition_m3_s':max_e_src,
                           'max_direct_native_energetic_diagnostic_diff':native_energy_diff,
                           'array_layout_note':'np.load Y is Fortran-strided. Native evaluate on row-wise C-contiguous copies reproduces original ne exactly; direct Fortran-strided reductions can differ by ~8.5e-14 m^-3 due binary64 accumulation order.'},
      'tau':{'thomson_csigma_m3_per_s':THOMSON_M3_PER_S,
             'native_gamma_max_abs_diff':float(np.max(np.abs(tau_x-tau_native))),
             'native_gamma_max_rel_diff':float(np.max(np.abs(tau_x-tau_native)/tau_native)),
             'stored_final':float(tau_stored[-1]),'trap_final':float(tau_trap[-1]),
             'simpson_final':float(tau_simp[-1]),
             'trap_endpoint_signed_error':float(tau_trap[-1]-tau_stored[-1]),
             'simpson_endpoint_signed_error':float(tau_simp[-1]-tau_stored[-1]),
             'simpson_endpoint_relative_error':ratio(abs(tau_simp[-1]-tau_stored[-1]),tau_stored[-1]),
             'trap_max_abs_error_all_epochs':float(np.max(np.abs(tau_trap-tau_stored))),
             'simpson_max_abs_error_all_epochs':float(np.max(d_tau)),
             'simpson_max_err_at_index':m,'simpson_max_err_at_redshift':float(z[m]),
             'min_stored_delta_tau':float(np.min(np.diff(tau_stored))),
             'note':'Postprocess composite Simpson and trapezoid on 193 fixed x epochs do not replay original adaptive BDF quadrature; discrepancy is combined sampling, interpolation and IVP errors, not uniquely attributable to one cause.'},
      'proper_time':{'stored_elapsed_final_s':float(fields['elapsed_s'][-1]),
                    'simpson_final_s':float(elapsed_simp[-1]),
                    'trap_final_s':float(elapsed_trap[-1]),
                    'simpson_end_rel_err':ratio(elapsed_simp[-1]-fields['elapsed_s'][-1],fields['elapsed_s'][-1])},
      'chemistry':{'xe_start':float(xe[0]),'xe_end':float(xe[-1]),
                   'xe_simpson_final':float(xe_simp[-1]),
                   'xe_simpson_endpoint_error':dx_e_simp_err,
                   'peak_electron_fraction':float(np.max(xe)),
                   'first_xe_half_index':int(np.searchsorted(xe,0.5)) if float(np.max(xe))>=0.5 else None,
                   'net_source_start_m3_s':float(net_chem[0]),
                   'net_source_end_m3_s':float(net_chem[-1]),
                   'proper_photo_event_exposure_per_baryon':events['photo'],
                   'proper_RR_event_exposure_per_baryon':events['rr'],
                   'proper_DR_event_exposure_per_baryon':events['dr'],
                   'power_exposure_definition':'integral of local photo absorbed W/m3 against proper time; not comoving conserved heating',
                   'photo_power_integrated_dt_J_per_m3_of_epoch_local_volume':photon_power_total},
      'model_limits':{'prescribed_temperature_and_UV':True,'full_photon_transfer':False,
                      'photo_absorbed_power_is_thermal_heating':False,
                      'REI17_P19_as_same_owner':False,'independent_R6_C9':'NOT_RUN',
                      'native_RODAS_transaction':'NOT_RUN','production':'HOLD'}
    }
    rows=[]
    for i in range(n):
        row={'index':i,'x':x[i],'z':z[i],'elapsed_s':fields['elapsed_s'][i],
             'H_s':H[i],'nb_m3':nb[i],'ne_m3':ne[i],'x_e':xe[i],
             'baryon_sum':mass[i],'T_K':T[i],'UV_scale':uv[i],
             'tau_T_stored':tau_stored[i],'dtau_dx_stored':tau_native[i],
             'dtau_dx_native_FJ_same_source':tau_x[i],
             'tau_T_trapezoid_193':tau_trap[i],'tau_T_simpson_193':tau_simp[i],
             'simpson_minus_saved_tau':tau_simp[i]-tau_stored[i],
             'photo_e_m3_s':photo_e[i],'RR_e_m3_s':rr_e[i],
             'DR_e_m3_s':dr_e[i],'decay_e_m3_s':decay_e[i],
             'chemical_e_fixed_nb_m3_s':net_chem[i],
             'chemical_dxe_dx':dx_e_chem[i],
             'photo_absorbed_W_m3':photo_energy[i],
             'primary_photoelectron_W_m3':primary_energy[i],
             'unresolved_core_W_m3':core_energy[i],
             'UV_energy_J_m3':uv_energy[i],'Li1s_photo_events_m3_s':li1s_event[i]}
        row.update({'Gamma_'+k+'_s':rate[k][i] for k in SPECIES})
        rows.append(row)
    arrays={'x':x,'z':z,'Y_original':Y,'F_proper_s':F,'J_proper_s':J,
            'photo_e_m3_s':photo_e,'RR_e_m3_s':rr_e,'DR_e_m3_s':dr_e,
            'chemical_e_m3_s':net_chem,'tau_x':tau_x,'tau_stored':tau_stored,
            'tau_trapezoid':tau_trap,'tau_simpson':tau_simp,
            'xe':xe,'xe_simpson':xe_simp,'gamma_rates_s':np.column_stack([rate[s] for s in SPECIES]),
            'species_rate_names':np.asarray(list(SPECIES))}
    return out,rows,arrays

def write():
    summary,rows,arrays=compute()
    out=REPLAY/'results';out.mkdir(parents=True,exist_ok=True)
    (out/'SUMMARY.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2,allow_nan=False)+'\n')
    with (out/'NATIVE_BASS_193_EPOCH_HISTORY.csv').open('w',newline='') as fh:
        w=csv.DictWriter(fh,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
    np.savez_compressed(out/'ACTUAL_FJ_ALL_193_EPOCHS.npz',**arrays)
    print(json.dumps({'scientific_claim':summary['schema'],'source_SHA256':SOURCE_SHA256,
          'counts':summary['counts'],'tau':summary['tau'],
          'proper_time':summary['proper_time'],'chemistry':summary['chemistry'],
          'invariants':summary['source_invariants']},indent=2,allow_nan=False))

if __name__=='__main__':
    ap=argparse.ArgumentParser();ap.add_argument('--write',action='store_true');args=ap.parse_args()
    if args.write: write()
    else:print(json.dumps(compute()[0],indent=2))
