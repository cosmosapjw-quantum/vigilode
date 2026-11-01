"""Independent SciPy Radau/DOP853 full one-original-tick frozen REI photoionization."""
from __future__ import annotations
from pathlib import Path
import os
import json,hashlib,csv,gzip,io
from zipfile import ZipFile
import numpy as np
from scipy.integrate import solve_ivp
from rei_moments import MomentSource
HERE=Path(__file__).resolve().parent
UP=Path(os.environ.get('REI_PR332_ZIP', str(HERE/'upstream/REI_PR332_GLOBAL_PHOTON_46080_SOURCE_20261101.zip')))
SHA='83d94b5383ac7c1f36d89486e83bf8b4a56dd1993a25af3ae1f8fed06a695ba9'

def fmt(x):return format(float(x),'.17g')

def main():
    model=MomentSource()
    t=np.linspace(0,1,9)
    opts={'rtol':3e-12,'atol':2e-14,'t_eval':t}
    a=solve_ivp(model.rhs_aug,(0,1),np.r_[np.ones(3),np.zeros(3)],method='Radau',jac=model.jac_aug,**opts)
    b=solve_ivp(model.rhs_aug,(0,1),np.r_[np.ones(3),np.zeros(3)],method='DOP853',**opts)
    assert a.success and b.success
    assert np.all(a.y[:3]>0) and np.all(b.y[:3]>0) and np.all(a.y[3:]>=0) and np.all(b.y[3:]>=0)
    discrepancy=float(np.max(np.abs(a.y-b.y)))
    assert discrepancy<1e-11,discrepancy
    out=HERE/'generated';out.mkdir(exist_ok=True)
    rows=[]
    for i,x in enumerate(t):
        y=a.y[:,i];w=y[:3];r,p,pe=model.rates(w);cnt=model.totals(y)
        rows.append({'u':fmt(x),'proper_t_elapsed_s':fmt(x*model.h),
                     **{f'S_{s}_over_initial':fmt(w[k]) for k,s in enumerate(('HI','HeI','HeII'))},
                     **{f'absorbed_{s}_cumulative_particles':fmt(cnt[k]) for k,s in enumerate(('HI','HeI','HeII'))},
                     **{f'photo_{s}_per_s':fmt(r[k]) for k,s in enumerate(('HI','HeI','HeII'))},
                     'total_initial_source_current_per_s':fmt(np.sum(model.J)),
                     'escape_per_s':fmt(float(model.J@pe)),
                     'electron_produced_cumulative_particles':fmt(np.sum(cnt)),
                     'photon_normalization_error':fmt(model.normalize_source(x,w))})
    with (out/'SOURCE_FROZEN_PHYSICAL_9_TIMES.csv').open('w',newline='') as z:
        writer=csv.DictWriter(z,fieldnames=rows[0].keys());writer.writeheader();writer.writerows(rows)

    rawhash=hashlib.sha256(UP.read_bytes()).hexdigest();assert rawhash==SHA
    with ZipFile(UP) as z:
        assert z.testzip() is None
        filename='physical_output/ORIGINAL_26EVENT_GLOBAL_PHOTON_JACOBIAN_46080.csv.gz'
        # source actual 46k per-cell photons per second, exact file identity stored in PR332 JSON
        src=json.loads(z.read('physical_output/RESULT.json'))
        expected=src['nodewise_csv_sha256']
        compressed=z.read(filename)
        assert hashlib.sha256(compressed).hexdigest()==expected
        rows_in=list(csv.DictReader(io.TextIOWrapper(gzip.GzipFile(fileobj=io.BytesIO(compressed)),encoding='utf-8')))
    assert len(rows_in)==46080
    r0,_,_=model.rates(np.ones(3)); C=model.totals(a.y[:,-1]);assert np.all(C>=0),C
    events=[];summed=np.zeros(3)
    for row in rows_in:
        rr=np.array([float(row['original_photo_'+s+'_events_per_s']) for s in ('HI','HeI','HeII')]);
        # For HI and HeI, the spatial share of group absorption remains constant because
        # their local loss hazard depends only on global moments and frozen forcing.
        n=rr[:2]/r0[:2]*C[:2]
        # HeII source is numerically subnormal in the frozen original group 3; it is
        # NOT meaningfully calibrated as a physical HeII absorption experiment.
        if rr[2]!=0.0: raise ValueError('unresolved original HeII channel cannot be dropped')
        d_e=float(n.sum())
        summed[:2]+=n
        events.append({'node':row['node'],'parent_T_K':row['source_tick160_T_K'],
             'parent_x_HII':row['source_tick160_xHII'],
             'original_photo_HI_events_per_s':fmt(rr[0]),
             'original_photo_HeI_events_per_s':fmt(rr[1]),
             'photo_HI_absorptions_one_tick':fmt(n[0]),
             'photo_HeI_absorptions_one_tick':fmt(n[1]),
             'new_electrons_source_only_one_tick':fmt(d_e)})
    assert np.max(np.abs(summed[:2]-C[:2])/C[:2])<4e-13,(summed,C)
    with (out/'SOURCE_46080_CELL_PHOTOELECTRONS.csv.gz').open('wb') as stream, gzip.GzipFile(fileobj=stream,mode='wb',filename='',mtime=0,compresslevel=9) as payload, io.TextIOWrapper(payload,encoding='utf-8',newline='') as gz:
        writer=csv.DictWriter(gz,fieldnames=events[0].keys());writer.writeheader();writer.writerows(events)
    # Independent photon accounting: total source photons = absorbed species + escaped.
    from scipy.integrate import quad
    # use high-order DOP853 dense output for independent source escape integral
    dense=solve_ivp(model.rhs_aug,(0,1),np.r_[np.ones(3),np.zeros(3)],method='DOP853',rtol=3e-12,atol=2e-14,dense_output=True)
    n_escape=quad(lambda u:float(model.J@model.rates(dense.sol(u)[:3])[2])*model.h,0,1,epsrel=2e-11)[0]
    photon_resid=float((n_escape+sum(C)-float(np.sum(model.J)*model.h))/float(np.sum(model.J)*model.h))
    assert abs(photon_resid)<2e-11,photon_resid
    output={'schema':'VIGILODE_REI_PHYSICAL_3MOMENT_PHOTO_SOURCE_ONE_TICK_V1',
            'frozen_original_input_source_sha256':SHA,'original_46k_cells':46080,
            'original_one_tick_seconds':model.h,'source_groups':4,
            'model':'original REI explicit photoionization photon competition only, fixed forcing and no other 26-event physics',
            'two_independent_solves':{'Radau':{'success':a.success,'nfev':a.nfev,'njev':a.njev},'DOP853':{'success':b.success,'nfev':b.nfev},'max_state_difference':discrepancy},
            'final_dimensionless_neutral_moments':a.y[:3,-1].tolist(),
            'global_absorbed_photons_particles':C.tolist(),
            'global_source_photoelectrons_particles':float(sum(C)),
            'fractional_HI_depletion':float(a.y[3,-1]),
            'fractional_HeI_depletion':float(a.y[4,-1]),
            'photon_identity_relative_residual':photon_resid,
            'spatial_source_replay_relative_error':float(np.max(np.abs(summed[:2]-C[:2])/C[:2])),
            'frozen_rate_linear_photons':(r0*model.h).tolist(),
            'nonlinear_feedback_relative_to_frozen_rate':[float((C[i]-r0[i]*model.h)/(r0[i]*model.h)) if r0[i]*model.h>1e-300 else None for i in range(3)],
            'source_rows':46080,'time_rows':len(rows),
            'canonical_rei_delta':0,'vigilode_official_delta':0,
            'original_REI_26event_first_interval':'BLOCKED_MINIMUM_STEP',
            'original_tick160_161_full_parent_error':2.1245050576368385e-4,
            'not_full_original_source':True,'new_full_owner_ivps':0,
            'no_bianchi_geometry_coupling':True,
            'source_zero_subnormal_note':'original fourth group current is smallest positive float64, numerically unresolved; not interpreted as measured zero physical flux'}
    (out/'RESULTS.json').write_text(json.dumps(output,indent=2,sort_keys=True)+'\n')
    print(json.dumps(output,indent=2))
    return output
if __name__=='__main__':main()
