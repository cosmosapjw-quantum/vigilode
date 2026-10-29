"""BASS original 21-species source-owned *stored endpoint* physical diagnostics.

No new IVP. All state vectors copied from a pinned original BASS pilot
endpoint CSV. No REI-17 equivalence or empirical observation claim.
The original AtomicNetwork is run against the PR269 verified binary64
projection of the pinned BASS original atomic coefficients.
"""
from __future__ import annotations
import sys, json, hashlib, math, csv
from pathlib import Path
import numpy as np

ROOT=Path(__file__).resolve().parents[1]
PROJECT=ROOT.parent
sys.path.insert(0,str(PROJECT/'rei_pr269'/'code'))
import native_full_rate as bsource

GAMMA_T_PER_NE_M3_S=(2.99792458e10*6.6524587051e-25)/1e6
IDX={"H1":(1,2),"D":(3,4),"T":(5,6),"He3":(7,8,9),"He4":(10,11,12),"Li6":(13,14,15,16),"Li7":(17,18,19,20)}

def build():
    bsource.source_manifest_check()
    net=bsource.new_native_network()
    owner=json.loads((ROOT/'source'/'SELECTED_BASS_ENDPOINTS.json').read_text())
    assert owner['source_git_blob']=='a49aa5276b4256d6bf0ad443b6b830effe2ab738'
    all_results=[]; tmp={}
    for row in owner['cases']:
        Y=np.asarray(row['Y'],float); assert Y.shape==(21,) and np.all(Y>=0)
        T=float(row['T_K']);uv=float(row['uv']);nb=float(row['nb_m3']);ne_saved=float(row['ne_m3']);z=float(row['z'])
        T_expect,uv_expect=bsource.original_bass_bath_at_z(z)
        assert abs(T-T_expect)<1e-8 and abs(uv-uv_expect)<1e-14
        ev=net.evaluate(Y,T,uv,nb);r=net.rates(T,uv);ne=ev['ne']
        q=net.q; A=net.A
        assert abs(ne-ne_saved)<1e-12
        assert abs(float(A@Y)-1)<1e-12
        phi=nb*sum(Y[i]*r['photo_s'][i] for i in net.photo_indices)
        rrx=nb*ne*sum(Y[i]*r['rr_m3_s'][i] for i in net.recombination_indices)
        drx=nb*ne*sum(Y[i]*r['dr_m3_s'][i] for i in net.recombination_indices)
        photo_e=nb*float(q@ev['photo_rhs'])
        rr_e=nb*float(q@ev['rr_rhs'])
        dr_e=nb*float(q@ev['dr_rhs'])
        decay_e=nb*float(q@ev['decay_rhs'])
        assert np.isclose(photo_e,phi,rtol=1e-12,atol=1e-20)
        assert np.isclose(rr_e,-rrx,rtol=1e-12,atol=1e-20)
        assert np.isclose(dr_e,-drx,rtol=1e-12,atol=1e-20)
        total_source=ev['electron_source_m3_s']
        assert np.isclose(total_source, photo_e+rr_e+dr_e+decay_e, rtol=1e-12,atol=1e-20)
        # Native BASS output gamma_T_per_H already specifies the achieved
        # mean H of the stored geometry. No FLRW replacement is made.
        gammaT=float(row['gamma_T_per_H'])
        H=(GAMMA_T_PER_NE_M3_S*ne)/gammaT
        tauprime=GAMMA_T_PER_NE_M3_S*ne/H
        assert abs(tauprime/gammaT-1)<2e-14
        # Charge pool; lithium contribution is not the total electron density.
        iso_e={k:float(q[list(v)]@Y[list(v)]) for k,v in IDX.items()}
        iso_inventory={k:float(np.sum(Y[list(v)])) for k,v in IDX.items()}
        frac_li6=iso_e['Li6']/float(q@Y)
        # The time rate here is purely chemical at fixed n_b; total d n_e/dt
        # must also include changing baryon density.
        jac=np.asarray(ev['jac'],float)
        tr=np.max(abs(net.A @ jac))
        calc={"case_id":row['id'],"z":z,"T_K":T,"uv_scale":uv,"nb_m3":nb,"ne_m3":ne,"x_e":float(q@Y),"H_per_s_reconstructed":H,"tau_T_stored":float(row['tau_T']),"dtau_dx_stored":gammaT,"dtau_dx_recomputed":tauprime,"chemical_dne_dt_fixed_nb_m3_s":float(total_source),"photo_electron_source_m3_s":photo_e,"RR_electron_sink_m3_s":rr_e,"DR_electron_sink_m3_s":dr_e,"decay_electron_source_m3_s":decay_e,"photo_events_per_m3_s":phi,"RR_events_per_m3_s":rrx,"DR_events_per_m3_s":drx,"photo_absorbed_W_m3":float(ev['photo_absorbed_power_W_m3']),"primary_kinetic_W_m3":float(ev['primary_kinetic_power_W_m3']),"unresolved_core_W_m3":float(ev['unresolved_core_power_W_m3']),"uv_energy_J_m3":float(ev['uv_energy_density_J_m3']),"LiI_photo_rate_s":float(r['photo_s'][13]),"H1I_photo_rate_s":float(r['photo_s'][1]),"He4I_photo_rate_s":float(r['photo_s'][10]),"He4II_photo_rate_s":float(r['photo_s'][11]),"LiII_photo_rate_s":float(r['photo_s'][14]),"LiIII_photo_rate_s":float(r['photo_s'][15]),"li1s_event_rate_m3_s":float(ev['li1s_photo_event_rate_m3_s']),"Li6_electron_per_baryon":iso_e['Li6'],"Li7_electron_per_baryon":iso_e['Li7'],"Li6_electron_fraction_of_all":frac_li6,"isotope_electron_per_baryon":iso_e,"isotope_inventory_per_baryon":iso_inventory,"baryon_total":float(net.A@Y),"baryon_RHS_residual_per_s":float(net.A @ ev['dY_dt']),"max_baryon_J_residual_per_s":float(tr),"state_min_Y":float(Y.min())}
        all_results.append(calc)
        tmp[row['id']]={'Y':Y, 'J':jac, 'F':np.asarray(ev['dY_dt']), 'rates':r}
    return owner,net,all_results,tmp

def write_outputs():
    owner,net,rows,tmp=build()
    out=ROOT/'results';out.mkdir(parents=True,exist_ok=True)
    path=out/'NATIVE_BASS21_ENDPOINT_PHYSICS.json'
    obj={"schema":"vigilode.bass21.saved-endpoint-physics.v1","source_commit":owner['source_commit'],"stored_original_csv_git_blob":owner['source_git_blob'],"original_atomic_network_sha256":bsource.EXPECTED['BASS_ATOMIC_NETWORK.py'],"rate_coeff_source_git_blob":"2eba600a9d7a7359ca55f566f1b841a363a3e5c7","independent_variable":"x=ln(a/a_ref)","physical_model":"BASS original finite21; prescribed T/UV; stored compact33 pilot physical fields","sample_count":len(rows),"new_IVPs":0,"production":"HOLD","rows":rows}
    path.write_text(json.dumps(obj,indent=2,ensure_ascii=False,allow_nan=False)+"\n")
    fields=['case_id','z','T_K','nb_m3','ne_m3','x_e','H_per_s_reconstructed','tau_T_stored','dtau_dx_stored','chemical_dne_dt_fixed_nb_m3_s','photo_electron_source_m3_s','RR_electron_sink_m3_s','DR_electron_sink_m3_s','photo_absorbed_W_m3','primary_kinetic_W_m3','unresolved_core_W_m3','LiI_photo_rate_s','LiII_photo_rate_s','LiIII_photo_rate_s','Li6_electron_fraction_of_all']
    with (out/'ENDPOINT_PHYSICS.csv').open('w',newline='') as fh:
        w=csv.DictWriter(fh,fieldnames=fields);w.writeheader();w.writerows({k:r[k] for k in fields} for r in rows)
    # Actual source-native vector field and Jacobian, not numerical finite differences.
    np.savez_compressed(out/'ACTUAL_SOURCE_J_F_AT_ENDPOINTS.npz',case_id=np.array([r['case_id'] for r in rows]),Y=np.stack([tmp[r['case_id']]['Y'] for r in rows]),J_proper_s=np.stack([tmp[r['case_id']]['J'] for r in rows]),F_proper_s=np.stack([tmp[r['case_id']]['F'] for r in rows]))
    return rows

if __name__=='__main__':
    rows=write_outputs()
    for r in rows:
        print(r['case_id'], 'x_e=',format(r['x_e'],'.12g'),'photo=',format(r['photo_electron_source_m3_s'],'.8g'),'RR=',format(r['RR_electron_sink_m3_s'],'.8g'),'DR=',format(r['DR_electron_sink_m3_s'],'.8g'),'LiI=',format(r['LiI_photo_rate_s'],'.8g'),'H=',format(r['H_per_s_reconstructed'],'.8g'),'tau=',format(r['tau_T_stored'],'.9g'))
