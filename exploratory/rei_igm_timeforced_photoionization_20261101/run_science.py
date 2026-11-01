"""Real original-source time-dependent reduced photon/charge history and 46080-cell response."""
from __future__ import annotations
import csv,gzip,hashlib,io,json,zipfile
from pathlib import Path
import numpy as np
from source_time_ivp import SourceIVP,ND
from prepare_sources import read_sources
ROOT=Path(__file__).resolve().parent
PR320_SHA='48ad5c6f0922ad946a10a89ce17fefd2c7241ac956b6bac194892a42810312a7'
PR320=Path('/mnt/data/REI_PR320_ORIGINAL_TICK160_PHYSICS_20261101.zip')

def read_state_intervals():
    if not (hashlib.sha256(raw).hexdigest() == PR320_SHA): raise ValueError('run_science.py original source numerical contract line 13')
    with zipfile.ZipFile(io.BytesIO(raw)) as z:
        if not (z.testzip() is None): raise ValueError('run_science.py original source numerical contract line 15')
        with gzip.open(io.BytesIO(z.read('NODEWISE_LOCAL_NEUTRAL_HAZARD_PRIMARY_46080_PHYSICAL.csv.gz')),'rt',newline='') as f: rows=list(csv.DictReader(f))
    if not (len(rows) == 46080): raise ValueError('run_science.py original source numerical contract line 17')
    a=lambda k:np.asarray([float(r[k]) for r in rows])
    vals={k:a(k) for k in ['x_HII_lo','x_HII_hi','x_HeII_lo','x_HeII_hi','x_HeIII_lo','x_HeIII_hi','xe_per_H_lo','xe_per_H_hi','T_K_lo','T_K_hi']}
    y=(0.5*(vals['xe_per_H_lo']+vals['xe_per_H_hi'])-0.5*(vals['x_HII_lo']+vals['x_HII_hi'])) / (.5*(vals['x_HeII_lo']+vals['x_HeII_hi'])+vals['x_HeIII_lo']+vals['x_HeIII_hi'])
    if not (abs(np.median(y) - 0.079) < 1e-08 and np.max(np.abs(y - 0.079)) < 5e-08): raise ValueError('run_science.py original source numerical contract line 21')
    return vals

def run():
    f,HI,He,oldrows=read_sources();original=read_state_intervals()
    mod_dyn=SourceIVP(frozen=False);mod_const=SourceIVP(frozen=True)
    r_dyn=mod_dyn.solve(method='Radau',rtol=1e-12,atol=1e-14)
    r_const=mod_const.solve(method='Radau',rtol=1e-12,atol=1e-14)
    o_dyn=mod_dyn.solve(method='DOP853',rtol=1e-12,atol=1e-14)
    o_const=mod_const.solve(method='DOP853',rtol=1e-12,atol=1e-14)
    if not (np.allclose(r_dyn.t, r_const.t, rtol=0, atol=2e-14)): raise ValueError('run_science.py original source numerical contract line 31')
    if not (np.max(np.abs(o_dyn.y - r_dyn.y)) < 3e-12): raise ValueError('run_science.py original source numerical contract line 32')
    if not (np.max(np.abs(o_const.y - r_const.y)) < 3e-12): raise ValueError('run_science.py original source numerical contract line 33')
    err_inv_dyn=float(np.max(np.abs(mod_dyn.invariants(r_dyn.y.T))))
    err_inv_frozen=float(np.max(np.abs(mod_const.invariants(r_const.y.T))))
    if not (max(err_inv_dyn, err_inv_frozen) < 8e-13): raise ValueError('run_science.py original source numerical contract line 36')
    yD=r_dyn.y[:,-1];yC=r_const.y[:,-1]
    if not ((yD[:3] > 0).all() and (yC[:3] > 0).all()): raise ValueError('run_science.py original source numerical contract line 38')
    # H and HeI spatial source response: same spatial neutral probability for all t.
    Hsum=float(np.sum(HI,dtype=np.longdouble));Hesum=float(np.sum(He,dtype=np.longdouble))
    HIshare=HI/Hsum;Heshare=He/Hesum
    photonsD_H=yD[3]*mod_dyn.S[0]*HIshare;photonsD_He=yD[4]*mod_dyn.S[1]*Heshare
    photonsC_H=yC[3]*mod_dyn.S[0]*HIshare;photonsC_He=yC[4]*mod_dyn.S[1]*Heshare
    if not (abs(np.sum(photonsD_H, dtype=np.longdouble) / (yD[3] * mod_dyn.S[0]) - 1) < 5e-14): raise ValueError('run_science.py original source numerical contract line 44')
    if not (abs(np.sum(photonsD_He, dtype=np.longdouble) / (yD[4] * mod_dyn.S[1]) - 1) < 5e-14): raise ValueError('run_science.py original source numerical contract line 45')
    # Source-authentic accepted-interval outer range of electron response;
    # this is not a jointly certified original 26-event IVP solution.
    hionD=1-yD[0];heionD=1-yD[1];hionC=1-yC[0];heionC=1-yC[1]
    delta_eD_lo=(1-original['x_HII_hi'])*hionD+.079*np.maximum(0,1-original['x_HeII_hi']-original['x_HeIII_hi'])*heionD
    delta_eD_hi=(1-original['x_HII_lo'])*hionD+.079*np.maximum(0,1-original['x_HeII_lo']-original['x_HeIII_lo'])*heionD
    delta_eC_lo=(1-original['x_HII_hi'])*hionC+.079*np.maximum(0,1-original['x_HeII_hi']-original['x_HeIII_hi'])*heionC
    delta_eC_hi=(1-original['x_HII_lo'])*hionC+.079*np.maximum(0,1-original['x_HeII_lo']-original['x_HeIII_lo'])*heionC
    if not (np.all(delta_eD_lo >= 0) and np.all(delta_eD_hi >= delta_eD_lo)): raise ValueError('run_science.py original source numerical contract line 53')
    if not (np.all(delta_eC_lo >= 0) and np.all(delta_eC_hi >= delta_eC_lo)): raise ValueError('run_science.py original source numerical contract line 54')
    def quant(a):return {str(k):float(np.percentile(a,k)) for k in (0,5,25,50,75,95,100)}
    out=ROOT/'results';out.mkdir(parents=True,exist_ok=True)
    with (out/'ORIGINAL_REI_46080_CELL_SOURCE_TIME_RESPONSE.csv').open('w',newline='') as fd:
        w=csv.writer(fd)
        w.writerow(['node','source_HI_absorbed_photons_timevarying','source_HeI_absorbed_photons_timevarying','source_photoelectrons_timevarying','source_HI_absorbed_photons_frozen','source_HeI_absorbed_photons_frozen','source_photoelectrons_difference_timevarying_minus_frozen','delta_xe_per_H_timevarying_lower','delta_xe_per_H_timevarying_upper','delta_xe_per_H_frozen_lower','delta_xe_per_H_frozen_upper','source_parent_T_K_mid'])
        temp=.5*(original['T_K_lo']+original['T_K_hi'])
        for i in range(46080):
            w.writerow([i,*[format(float(x[i]),'.17g') for x in (photonsD_H,photonsD_He,photonsD_H+photonsD_He,photonsC_H,photonsC_He,(photonsD_H+photonsD_He)-(photonsC_H+photonsC_He),delta_eD_lo,delta_eD_hi,delta_eC_lo,delta_eC_hi,temp)]])
    with (out/'ORIGINAL_REI_GLOBAL_IONIZATION_HISTORY.csv').open('w',newline='') as fd:
        w=csv.writer(fd);w.writerow(['u','proper_time_s_since_tick160','w_HI_timevarying','w_HeI_timevarying','w_HeII_timevarying','cum_HI_photons_over_S0_HI','cum_HeI_photons_over_S0_HeI','cum_HeII_photons_over_S0_HeII','emitted_photons_per_initial_HI','escaped_photons_per_initial_HI','ionization_binding_eV_per_initial_HI','w_HI_frozen','w_HeI_frozen','binding_eV_per_HI_frozen'])
        for k,u in enumerate(r_dyn.t):
            a=r_dyn.y[:,k];b=r_const.y[:,k]
            w.writerow([format(float(u),'.17g'),format(float(u*mod_dyn.h),'.17g'),*[format(float(x),'.17g') for x in (a[0],a[1],a[2],a[3],a[4],a[5],sum(a[10:14]),sum(a[6:10]),a[14],b[0],b[1],b[14])]])
    # original point-controlled source photon ledger and spatial original counts.
    record={
      'schema':'VIGILODE_REI_ORIGINAL_TIME_DEPENDENT_FOUR_GROUP_PHOTON_SOURCE_20261101_V1',
      'owner_science':'cosmosapjw-quantum/rei_bianchi', 'owner_method':'cosmosapjw-quantum/vigilode',
      'scope':'source-frozen 3-moment photoionization-only; 5 original forcing clock sites; linear inter-knot source interpolant',
      'source_pr332_SHA256':f['source_pr332_artifact_sha256'],
      'source_pr342_SHA256':f['source_pr342_artifact_sha256'],
      'source_pr320_SHA256':PR320_SHA,'source_cell_count':46080,
      'forcing_clock_sites':len(f['sites']),'proper_tick_s':mod_dyn.h,
      'original_HI_rate_particles_per_s':Hsum,'original_HeI_rate_particles_per_s':Hesum,
      'radiative_photons_and_species':[{'case':'timevarying','final_neutral_ratios':yD[:3].tolist(),'absorbed_photons_by_species_particles':(yD[3:6]*mod_dyn.S).tolist(),'absorbed_photoelectrons_particles':float(np.dot(yD[3:6],mod_dyn.S)),'emitted_photons_per_original_neutral_HI':float(sum(yD[10:14])),'escaped_photons_per_original_neutral_HI':float(sum(yD[6:10])),'ionization_binding_eV_per_original_neutral_HI':float(yD[14])},
      {'case':'frozen','final_neutral_ratios':yC[:3].tolist(),'absorbed_photons_by_species_particles':(yC[3:6]*mod_dyn.S).tolist(),'absorbed_photoelectrons_particles':float(np.dot(yC[3:6],mod_dyn.S)),'emitted_photons_per_original_neutral_HI':float(sum(yC[10:14])),'escaped_photons_per_original_neutral_HI':float(sum(yC[6:10])),'ionization_binding_eV_per_original_neutral_HI':float(yC[14])}],
      'differences':{'HI_ionization_fraction':float(hionD-hionC),'HeI_ionization_fraction':float(heionD-heionC),'ionization_binding_eV_per_original_neutral_HI':float(yD[14]-yC[14]),'photoelectrons_particles':float(np.dot(yD[3:6]-yC[3:6],mod_dyn.S)),'spatial_photoelectron_change_quantiles_particles':quant((photonsD_H+photonsD_He)-(photonsC_H+photonsC_He))},
      'spatial_physical_electron_delta_per_H_bounds_timevarying_quantiles':{'lower':quant(delta_eD_lo),'upper':quant(delta_eD_hi)},
      'independent_reference_max_discrepancy':{'forced_Radau_vs_DOP853':float(np.max(abs(o_dyn.y-r_dyn.y))),'frozen_Radau_vs_DOP853':float(np.max(abs(o_const.y-r_const.y)))},
      'conservation_max_residual':{'timevarying':err_inv_dyn,'frozen':err_inv_frozen},
      'runtime_python':{'Radau_nfev_forced':r_dyn.nfev,'DOP853_nfev_forced':o_dyn.nfev,'Radau_nfev_frozen':r_const.nfev,'DOP853_nfev_frozen':o_const.nfev},
      'spatial_new_rows':46080,'time_history_rows':int(len(r_dyn.t)),
      'no_original_REI_26_event_IVP':True,'interpolation_approximation':True,
      'original_F05_gate':'2.1245050576368385e-4 > 2e-4; HOLD',
      'VIGILODE_DAG':'7/23','VIGILODE_SUCCESSOR':'0/12','REI_CENTRAL_DAG':'28/88','REI_WORKING_DAG':'33/88','canonical_delta':0,'PRODUCTION':'HOLD'}
    (out/'SCIENCE_RESULT.json').write_text(json.dumps(record,sort_keys=True,indent=2,ensure_ascii=False)+'\n')
    print(json.dumps({'source_cells':46080,'global_history_rows':record['time_history_rows'],'differences':record['differences'],
      'conservation':record['conservation_max_residual'],'source_identity':record['source_pr342_SHA256']},indent=2,ensure_ascii=False))
    return record

if __name__=='__main__':run()
