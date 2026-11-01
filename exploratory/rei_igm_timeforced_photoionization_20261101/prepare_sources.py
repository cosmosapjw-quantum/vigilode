"""Exact source-identified PR332/PR342 inputs; no guessed REI rates.

The piecewise-linear *between-knot* forcing is a scoped model interpolation,
not the original continuous original-owner forcing or REI 26-event time evolution.
"""
from __future__ import annotations
import hashlib,json,zipfile,io,csv,gzip
from pathlib import Path
import numpy as np
ROOT=Path(__file__).resolve().parent
PR332_SHA='83d94b5383ac7c1f36d89486e83bf8b4a56dd1993a25af3ae1f8fed06a695ba9'
PR342_SHA='88f7e5bb1c6c44c0ca0c7f2ef2c81ac70098cc7ed773096868b1e7b4db25c035'
STATE_SHA='cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01'
MAIN_TAR_SHA='a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b'
P332=Path('/mnt/data/REI_PR332_GLOBAL_PHOTON_46080_SOURCE_20261101.zip')
P342=Path('/mnt/data/REI_PR342_FIVE_CLOCK_SOURCE_20261101.zip')

def read_sources(path332=P332,path342=P342):
    b332=Path(path332).read_bytes();b342=Path(path342).read_bytes()
    if not (hashlib.sha256(b332).hexdigest() == PR332_SHA): raise ValueError('PR332 exact source mismatch')
    if not (hashlib.sha256(b342).hexdigest() == PR342_SHA): raise ValueError('PR342 original time sample mismatch')
    with zipfile.ZipFile(io.BytesIO(b332)) as za, zipfile.ZipFile(io.BytesIO(b342)) as zb:
        if not (za.testzip() is None and zb.testzip() is None): raise ValueError('prepare_sources.py original source numerical contract line 23')
        a=json.loads(za.read('physical_output/RESULT.json'))
        b=json.loads(zb.read('RESULT.json'))
        if not (a['state_array_sha256'] == STATE_SHA == b['original_accept_state_array_sha256']): raise ValueError('prepare_sources.py original source numerical contract line 26')
        if not (b['original_source_archive_sha256'] == MAIN_TAR_SHA): raise ValueError('prepare_sources.py original source numerical contract line 27')
        rows=sorted([r for r in a['group_rows'] if r['case']=='accepted_at_tick160'],key=lambda x:x['group'])
        if not (len(rows) == 4 and [r['group'] for r in rows] == list(range(4))): raise ValueError('prepare_sources.py original source numerical contract line 29')
        S=np.array([rows[0]['source_sum_'+v] for v in ['HI','HeI','HeII']],float)
        if not (np.all(S > 0)): raise ValueError('prepare_sources.py original source numerical contract line 31')
        p=np.array([r['species_fraction'] for r in rows],float)
        e=np.array([r['external_fraction'] for r in rows],float)
        J=np.array([r['current_photons_per_s'] for r in rows],float)
        h=float(rows[0]['time_s'])/160.0
        if not (np.allclose(p.sum(axis=1) + e, 1, atol=2e-14, rtol=0)): raise ValueError('prepare_sources.py original source numerical contract line 36')
        by_site={}
        for r in b['source_group_rows']:
            by_site.setdefault(r['site'],[]).append(r)
        if not (len(by_site) == 5): raise ValueError('prepare_sources.py original source numerical contract line 40')
        order=sorted(by_site,key=lambda k:by_site[k][0]['time_s'])
        first=[by_site[order[0]][g] for g in range(4)]
        u0=first[0]['time_s']
        if not (abs(u0 - rows[0]['time_s']) < 1e-09): raise ValueError('prepare_sources.py original source numerical contract line 44')
        first_hi=np.array([r['original_current_hi_per_s'] for r in first])
        if not (np.all(first_hi > 0)): raise ValueError('prepare_sources.py original source numerical contract line 46')
        sites=[]
        for s in order:
            r=sorted(by_site[s],key=lambda x:x['g'])
            if not ([z['g'] for z in r] == list(range(4))): raise ValueError('prepare_sources.py original source numerical contract line 50')
            t=(r[0]['time_s']-u0)/h
            if abs(t-0.5)<1e-10:t=0.5
            if abs(t-1.0)<1e-10:t=1.0
            ratios=np.array([x['original_current_hi_per_s'] for x in r])/first_hi
            current=J*ratios
            weights=np.array([x['p0'] for x in r],dtype=float)
            weights=weights / weights.sum(axis=1)[:,None]
            # exact original PR332 baseline for comparison at u=0
            if s==order[0]:
                weights=np.column_stack((p,e))
            if not (np.max(np.abs(weights.sum(axis=1) - 1)) < 3e-14): raise ValueError('prepare_sources.py original source numerical contract line 61')
            if not (np.all(weights >= 0) and np.all(current >= 0)): raise ValueError('prepare_sources.py original source numerical contract line 62')
            sites.append({'name':s,'u':float(t),'t_s':r[0]['time_s'],
                          'J_per_s':current.tolist(),'weights':weights.tolist(),
                          'upper_J_original_per_s':[x['original_current_hi_per_s'] for x in r],
                          'redshift':r[0]['z']})
        if not (abs(sites[0]['u']) < 1e-15 and abs(sites[-1]['u'] - 1) < 3e-12): raise ValueError('prepare_sources.py original source numerical contract line 67')
        fixture={'schema':'VIGILODE_REI_ORIGINAL_46K_FIVE_SOURCE_TIME_INTERPOLANT_V1',
                 'source_owner':'cosmosapjw-quantum/rei_bianchi','original_owner_first_tick':160,
                 'source_pr332_artifact_sha256':PR332_SHA,
                 'source_pr342_artifact_sha256':PR342_SHA,
                 'original_state_array_sha256':STATE_SHA,'original_failed_run_tar_sha256':MAIN_TAR_SHA,
                 'model':'REI original frozen 3 neutral species, 4 photon groups; piecewise-linear source-time interpolation, reduced photoionization ONLY',
                 'interpolation':'linear in clock u for original J_hi time ratios and original frozen-state 4-channel p weights; base J from PR332 actual point source',
                 'N':46080,'h_seconds':h,'S0_particles':S.tolist(),
                 'ionization_potentials_eV':[13.598434599702,24.587389011,54.4177655282],
                 'sites':sites,'parent_possibility':'original tick160 representative, not full correlated interval',
                 'canonical_REI_delta':0,'DAG_vigilode':'7/23','DAG_rei':'28/88'}
        with gzip.open(io.BytesIO(za.read('physical_output/ORIGINAL_26EVENT_GLOBAL_PHOTON_JACOBIAN_46080.csv.gz')),'rt',encoding='utf-8',newline='') as fd:
            d=list(csv.DictReader(fd))
        if not (len(d) == 46080): raise ValueError('prepare_sources.py original source numerical contract line 81')
        cellHI=np.asarray([float(q['original_photo_HI_events_per_s']) for q in d]);cellHe=np.asarray([float(q['original_photo_HeI_events_per_s']) for q in d]);
        if not (np.all(cellHI >= 0) and np.all(cellHe >= 0)): raise ValueError('prepare_sources.py original source numerical contract line 83')
        total0=(J[:,None]*p).sum(axis=0)
        if not (abs(np.sum(cellHI, dtype=np.longdouble) / total0[0] - 1) < 2e-13): raise ValueError('prepare_sources.py original source numerical contract line 85')
        if not (abs(np.sum(cellHe, dtype=np.longdouble) / total0[1] - 1) < 2e-13): raise ValueError('prepare_sources.py original source numerical contract line 86')
        return fixture,cellHI,cellHe,d

if __name__=='__main__':
    f,hi,he,_=read_sources()
    out=ROOT/'five_clock_pr342_original_source.json'
    out.write_text(json.dumps(f,indent=2,sort_keys=True)+'\n')
    print(json.dumps({'fixture_sha256':hashlib.sha256(out.read_bytes()).hexdigest(),
      'times':[(s['name'],s['u']) for s in f['sites']],
      'initial_HI_rate':float(np.sum(hi,dtype=np.longdouble)),
      'initial_HeI_rate':float(np.sum(he,dtype=np.longdouble)),'nodes':len(hi)},indent=2))
