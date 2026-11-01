#!/usr/bin/env python3
"""Extract ONLY original verified PR332 46k-source physical constants; do not transform owners."""
from __future__ import annotations
from pathlib import Path
import os
from zipfile import ZipFile
import json,hashlib
HERE=Path(__file__).resolve().parent
UP=Path(os.environ.get('REI_PR332_ZIP', str(HERE/'upstream/REI_PR332_GLOBAL_PHOTON_46080_SOURCE_20261101.zip')))
SHA='83d94b5383ac7c1f36d89486e83bf8b4a56dd1993a25af3ae1f8fed06a695ba9'
assert hashlib.sha256(UP.read_bytes()).hexdigest()==SHA,(hashlib.sha256(UP.read_bytes()).hexdigest())
with ZipFile(UP) as z:
    assert z.testzip() is None
    j=json.loads(z.read('physical_output/RESULT.json'))
    assert j['unique_original_physical_nodes']==46080
    assert j['state_array_sha256']=='cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01'
    groups=sorted([g for g in j['group_rows'] if g['case']=='accepted_at_tick160'],key=lambda x:x['group'])
    assert len(groups)==4
    s0=[groups[0]['source_sum_'+s] for s in ['HI','HeI','HeII']]
    for g in groups:
        assert [g['source_sum_'+s] for s in ['HI','HeI','HeII']]==s0
    h=groups[0]['time_s']/160
    data={
    'schema':'VIGILODE_REI_ORIGINAL_3MOMENT_FROZEN_PHOTON_SOURCE_V1',
    'repo_owner':'cosmosapjw-quantum/rei_bianchi',
    'original_source_26event_accepted_tick':160,'original_first_interval_ticks':131072,
    'source_artifact_sha256':SHA,
    'source_pr332_exact_head':'2e3305c8ba9bff5d00412c5d79d9c1aa1e6f6e9a',
    'source_original_frozen_archive_sha256':'a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b',
    'original_state_array_sha256':j['state_array_sha256'],
    'source_case':'accepted_at_tick160',
    'nodes':46080,'t0_s':groups[0]['time_s'],'dt_s':h,
    'global_neutral_particle_counts_S0':s0,
    'groups':[{ 'g':g['group'],'original_photon_current_per_s':g['current_photons_per_s'],
                'baseline_species_absorption_fraction':g['species_fraction'],
                'baseline_external_escape_fraction':g['external_fraction'] } for g in groups],
    'model_scope':'source_frozen_only_fixed_four_group_photon_forcing_no_recombination_no_heating_thermal_root',
    'owner_native_ivps':0,'rei_central_dag':'28/88','vigilode_dag':'7/23'}
    for g in data['groups']:
        assert abs(sum(g['baseline_species_absorption_fraction'])+g['baseline_external_escape_fraction']-1)<3e-14
    out=HERE/'frozen_pr332_source.json'
    out.write_text(json.dumps(data,indent=2,sort_keys=True)+'\n')
    print(out, hashlib.sha256(out.read_bytes()).hexdigest())
