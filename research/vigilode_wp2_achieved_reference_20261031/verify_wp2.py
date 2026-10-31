#!/usr/bin/env python3
"""Fail-closed structural/scientific gates for WP2 source-scoped certificate.

This verifier checks the recorded exact-rational posterior result's provenance,
completeness and claimed inequalities. The direct generator+certificate code is
still the proof-relevant dependency; JSON inspection alone is not a theorem.
"""
import argparse,hashlib,json,math,struct
from pathlib import Path

CASES=('asymmetric_2','asymmetric_2_refined','coupled_3_cubic','linear_exchange_control')
PARENT_SWEEP_SHA='9dae90079893170edddcf5cf8af193e53b43ab5bcf0c436cb81ce7aa0f8790b0'
PARENT_SELECTION_SHA='56746487ac5c5c8702b53ef0ffd47694c040ab4e0adcfea1ec25aa3b8e9ba909'
PRIOR_ALPHA_IDS={'asymmetric_2':(1,1),'asymmetric_2_refined':(0,0),'coupled_3_cubic':(1,2),'linear_exchange_control':(0,0)}
PRIOR_CALLS={'asymmetric_2':(396,297),'asymmetric_2_refined':(810,602),'coupled_3_cubic':(784,546),'linear_exchange_control':(530,402)}
PRIOR_REF_PANELS={32:(2048,4096,4096,4096),64:(4096,8192,8192,8192)}


def verify(certificate_json,ref_dir,log_path,wp1_summary_path):
    c=json.loads(Path(certificate_json).read_text())
    src=Path(log_path).read_bytes();wp=Path(wp1_summary_path).read_bytes()
    assert hashlib.sha256(src).hexdigest()==PARENT_SWEEP_SHA,'IMMUTABLE_WP1_LOG_IDENTITY'
    assert hashlib.sha256(wp).hexdigest()==PARENT_SELECTION_SHA,'IMMUTABLE_WP1_SELECTION_IDENTITY'
    assert c['source_head']=='72bd52d6936b923bd110222f36df79172b9ced49'
    assert c['source_log_sha256']==PARENT_SWEEP_SHA
    assert c['prior_selected_pair_file_sha256']==PARENT_SELECTION_SHA
    assert set(c['micro_cohorts'])==set(CASES)
    assert c['proof_scope'].startswith('dyadic original source')
    per_micro={32:{},64:{}};panels=0;all_parity=0;total_source_arms=40;verified_bits=0
    for name in CASES:
        rs=c['micro_cohorts'][name]
        assert len(rs)==2 and {x['reference_micro'] for x in rs}=={32,64}
        for record in rs:
            micro=record['reference_micro'];per_micro[micro][name]=record
            idx=CASES.index(name)
            assert record['reference_panels']==PRIOR_REF_PANELS[micro][idx]
            assert record['proof_audited_intervals']==4
            assert record['reference_delta_grid_up']>=0
            assert math.isfinite(record['reference_delta_grid_up'])
            assert 0<=record['total_reference_plus_clock_delta_up']<1e-8
            assert record['total_reference_plus_clock_delta_up']>=record['reference_delta_grid_up']
            assert record['clock_bound_float']>=0
            assert record['max_defect_panel']>=0
            assert len(record['same_alpha'])==5
            assert record['number_certified_same_alpha']==sum(bool(x['guaranteed_1pct']) for x in record['same_alpha'])
            assert record['number_certified_same_alpha']==(1 if name=='linear_exchange_control' else 5)
            selected=record['PREREGISTERED_WP1_SELECTED_PAIR']
            assert (selected['alpha_baseline'],selected['alpha_fast'])==PRIOR_ALPHA_IDS[name]
            assert tuple(selected['source_calls'])==PRIOR_CALLS[name]
            assert selected['certified_1pct'] is True
            assert selected['worst_possible_true_error_ratio']<=1.01
            for lower,upper in (selected['baseline_error_interval'],selected['fast_error_interval']):
                assert 0<lower<=upper
            bin_path=Path(ref_dir)/record['reference_states_name']
            b=bin_path.read_bytes()
            n=3 if name=='coupled_3_cubic' else 2
            assert len(b)==(record['reference_panels']+1)*n*8
            assert hashlib.sha256(b).hexdigest()==record['reference_states_sha256']
            panels+=record['reference_panels'];verified_bits+=len(b)//8
            all_parity+=1
    assert panels==43008,('PANEL_COUNT',panels)
    assert all_parity==8 and total_source_arms==40
    assert sum(v['number_certified_same_alpha'] for v in per_micro[32].values())==16
    assert sum(v['number_certified_same_alpha'] for v in per_micro[64].values())==16
    print(f'WP2_SOURCE_PINNED=PASS SWEEP={PARENT_SWEEP_SHA} WP_SUMMARY={PARENT_SELECTION_SHA}')
    print(f'WP2_CERTIFIED_PANEL_COUNT={panels}/43008_PASS')
    print(f'WP2_REFERENCE_STATES={verified_bits}_BINARY64_BITS_SHA_PASS')
    print(f'WP2_CERTIFIED_PREVIOUS_FROZEN_PAIR={all_parity}/8_PASS')
    print('WP2_SAME_ALPHA_PARITY=16/20_PER_MICRO_PASS_WITH_4_NEGATIVE')
    print('WP2_OFFICIAL_VIGILODE_DAG=7/23 PRODUCTION=HOLD C9_DISTINCT_AUTHOR=NOT_RUN')
    return True

if __name__=='__main__':
    p=argparse.ArgumentParser()
    p.add_argument('--certificate',required=True)
    p.add_argument('--refdir',required=True)
    p.add_argument('--log',required=True)
    p.add_argument('--wp1-summary',required=True)
    a=p.parse_args()
    verify(a.certificate,a.refdir,a.log,a.wp1_summary)
