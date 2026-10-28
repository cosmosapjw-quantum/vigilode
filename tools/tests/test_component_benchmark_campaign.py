"""Frozen campaign shape and configured-arm binding, without solver execution."""
import json
import math
import sys
from pathlib import Path
from types import SimpleNamespace

import pytest

from component_benchmark import campaign, configured_adapter
from component_benchmark.cases import build_cases
from component_benchmark.registry import inventory

ROOT=Path(__file__).resolve().parents[2]


def test_expanded_78_cases_keep_anchors_and_fixed_pr101_grid():
    anchors={c['case_id']:c for c in build_cases()}
    expanded=campaign.expanded_cases()
    ids=[c['case_id'] for c in expanded]
    assert len(anchors)==34 and len(expanded)==78 and len(ids)==len(set(ids))
    assert {c['case_id']:c for c in expanded if c['case_id'] in anchors}==anchors
    for variant in (c for c in expanded if 'base_case_id' in c):
        base=anchors[variant['base_case_id']]
        assert variant['equation_form']==base['equation_form']
        assert variant['initial_state']==base['initial_state']
        assert variant['parameters']==base['parameters']
        assert variant['mass_matrix']==base['mass_matrix']
        assert variant['boundary_conditions']==base['boundary_conditions']
        assert variant['output_times']==base['output_times']
    pr=anchors['ode-pr101']
    assert pr['parameters']=={'lambda':-1e6,'beta':10.0}
    assert pr['initial_state']==[0.0] and pr['t_span']==[0.0,1.0]
    assert pr['rtol']==1e-8 and pr['atol']==[1e-11]
    assert len(pr['output_times'])==101
    assert pr['output_times']==[i/100 for i in range(101)]
    assert ids.count('ode-pr101')==1
    high=next(c for c in expanded if c['case_id']=='ode-rotation-1e4')
    assert high['t_span'][-1]==1.0
    assert len(high['output_times'])>=math.ceil(9*10000/(2*math.pi))


def test_all_configured_component_ids_exist_and_aliases_bind_real_base(tmp_path):
    cfg=campaign.configurations(ROOT,tmp_path)
    known={r['component_id'] for r in inventory(ROOT)}
    # The campaign retains its frozen78 cases while the integrated source
    # registry adds ten modules (eight integration + two new implementation).
    # Preserve the historical map and the earlier phase-one receipt unchanged.
    historical = json.loads((ROOT/'research/component_benchmark_20261018/COMPONENT_COVERAGE_MAP.json').read_text())
    historical_ids = {row['component_id'] for row in historical['current_ids']}
    added_modules = {
        'rodas5p-cli:src::triage', 'rodas5p-cli:src::triage_arms',
        'rodas5p-integrators:src::banded_exact_tests',
        'rodas5p-integrators:src::exact_band_cache',
        'rodas5p-integrators:src::global_budget',
        'rodas5p-integrators:src::pr101_verified',
        'rodas5p-integrators:src::rodas5p_fast_dense',
        'rodas5p-integrators:src::rodas5p_lagged',
        'rodas5p-integrators:src::stage_target_policy',
        'rodas5p-integrators:src::transaction',
    }
    assert len(historical_ids)==246 and len(cfg['cases'])==78
    assert known == historical_ids | added_modules
    assert not historical_ids & added_modules
    assert all(set(s['components'])<=known for s in cfg['solvers'].values())
    assert all(s['measurement_modes']==['timing'] for s in cfg['solvers'].values() if 'measurement_modes' in s)
    aliases={sid:s for sid,s in cfg['solvers'].items() if 'alias_of' in s}
    assert len(aliases)==9
    for sid,spec in aliases.items():
        assert spec['alias_of'] in cfg['solvers']
        assert sid!=spec['alias_of']
        assert spec['configuration']
        assert spec['executable_artifact']==cfg['solvers'][spec['alias_of']]['command'][0]
        assert spec['command'][spec['command'].index('--solver-id')+1]==spec['alias_of']
        assert json.loads(spec['command'][spec['command'].index('--solver-config-json')+1])==spec['configuration']
    assert cfg['solvers']['native-bdf1']['components'][-1].endswith('#BDF1')
    assert cfg['solvers']['native-radau3']['components'][-1].endswith('#RadauIIA3')


def test_freeze_allocation_census_is_separate_and_bounded(tmp_path,monkeypatch,capsys):
    monkeypatch.setattr(campaign,'source_binding',lambda root:{})
    monkeypatch.setattr(campaign,'write_reports',lambda rows,path:None)
    out=tmp_path/'frozen'
    monkeypatch.setattr(sys,'argv',['campaign','freeze','--root',str(ROOT),'--env-root',str(tmp_path),'--output',str(out)])
    campaign.main()
    info=json.loads(capsys.readouterr().out)
    screening=json.loads((out/'screening_config.json').read_text())
    allocation=json.loads((out/'allocation_config.json').read_text())
    reference=json.loads((out/'reference_config.json').read_text())
    assert info['status']=='FROZEN_INPUTS' and info['cases']==78
    assert len(screening['cases'])==78
    assert {c['case_id'] for c in allocation['cases']}=={
        'ode-scalar-decay','ode-robertson','ode-pr101','ode-periodic-heat','ode-sabr-scalar-fixture'}
    assert all(sid.startswith(('native-','diffsol-')) for sid in allocation['solvers'])
    assert all(spec['measurement_modes']==['allocation'] for spec in allocation['solvers'].values())
    assert {c['case_id'] for c in reference['cases']}=={
        c['case_id'] for c in build_cases()}|{'ode-scalar-decay@rtol1e-10','ode-rotation-1@rtol1e-10'}


def _wrapper_request(tmp_path, arm='native-rodas5p-fast@pi', config=None):
    p=tmp_path/'request.json'
    p.write_text(json.dumps({'case':{'case_id':'fake'},'solver_id':arm,
                             'solver_config':config or {}}))
    return p


def test_configured_adapter_passes_bound_solver_and_config_to_fake(tmp_path,monkeypatch,capsys):
    req=_wrapper_request(tmp_path)
    seen={}
    def fake_run(argv,**kwargs):
        seen.update(json.loads(Path(argv[argv.index('--request')+1]).read_text()))
        seen['argv']=argv
        return SimpleNamespace(stdout=json.dumps({'status':'OK','requested_solver_config':{'controller':'pi'},
                                                   'times':[0.0],'states':[[1.0]]}),stderr='',returncode=0)
    monkeypatch.setattr(configured_adapter.subprocess,'run',fake_run)
    monkeypatch.setattr(sys,'argv',['configured_adapter','--adapter','/fake/native','--solver-id','native-rodas5p-fast',
                                    '--solver-config-json','{"controller":"pi"}','--request',str(req)])
    with pytest.raises(SystemExit) as exit_info: configured_adapter.main()
    assert exit_info.value.code==0
    response=json.loads(capsys.readouterr().out)
    assert seen['solver_id']=='native-rodas5p-fast' and seen['solver_config']=={'controller':'pi'}
    assert '--measurement-mode' in seen['argv'] and 'timing' in seen['argv']
    assert response['benchmark_arm_id']=='native-rodas5p-fast@pi'
    assert response['underlying_solver_id']=='native-rodas5p-fast'
    assert response['configuration_binding']=={'controller':'pi'}


def test_configured_adapter_rejects_conflicting_request_before_fake_launch(tmp_path,monkeypatch):
    req=_wrapper_request(tmp_path,config={'controller':'predictive'})
    def forbidden(*args,**kwargs): raise AssertionError('fake adapter must not launch')
    monkeypatch.setattr(configured_adapter.subprocess,'run',forbidden)
    monkeypatch.setattr(sys,'argv',['configured_adapter','--adapter','/fake/native','--solver-id','native-rodas5p-fast',
                                    '--solver-config-json','{"controller":"pi"}','--request',str(req)])
    with pytest.raises(ValueError,match='conflicting frozen solver configuration'):
        configured_adapter.main()


def test_configured_adapter_rejects_unconfirmed_config(tmp_path,monkeypatch):
    req=_wrapper_request(tmp_path)
    monkeypatch.setattr(configured_adapter.subprocess,'run',lambda *a,**k:SimpleNamespace(
        stdout=json.dumps({'status':'OK','requested_solver_config':{'controller':'predictive'}}),stderr='',returncode=0))
    monkeypatch.setattr(sys,'argv',['configured_adapter','--adapter','/fake/native','--solver-id','native-rodas5p-fast',
                                    '--solver-config-json','{"controller":"pi"}','--request',str(req)])
    with pytest.raises(ValueError,match='did not confirm requested configuration'):
        configured_adapter.main()
