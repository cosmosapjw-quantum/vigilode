"""Fake P1 artifacts for the read-only P2 prerequisite generator."""
import copy
import json
from pathlib import Path

import pytest

from research.component_benchmark_20261018 import prepare_p2
from tools.component_benchmark.cases import build_cases
from tools.component_benchmark.protocol import digest, file_digest, physical_input_digest
from tools.component_benchmark.parity import frozen_points


def put(path,value):
    path.parent.mkdir(parents=True,exist_ok=True)
    path.write_text(json.dumps(value,sort_keys=True,allow_nan=False))


def fixture(tmp_path,monkeypatch):
    root=tmp_path/'repo'; root.mkdir()
    p1=root/'p1';frozen=p1/'frozen';frozen.mkdir(parents=True)
    cases={c['case_id']:c for c in build_cases()}
    selected=[cases['ode-scalar-decay'],cases['dae-index1-inconsistent'],cases['dae-pendulum-index3']]
    solvers={'native-rodas5p-fast':{'command':['/fake/native']},'scipy-radau':{'command':['/fake/scipy']}}
    screen={'cases':selected,'solvers':solvers,'contrasts':[]}
    allocation={'cases':selected[:2],'solvers':{'native-rodas5p-fast':solvers['native-rodas5p-fast']},'contrasts':[]}
    put(frozen/'screening_config.json',screen);put(frozen/'allocation_config.json',allocation)
    binding={'fake':'source'};source_hash=digest(binding)
    monkeypatch.setattr(prepare_p2,'source_binding',lambda root:binding)
    monkeypatch.setitem(prepare_p2.probe_receipt_valid.__globals__,'_bound_files',
                        lambda spec:{'status':'BOUND','files':{}})
    put(p1/'references/plan.json',{'source_binding':binding})
    states=[[1.0] for _ in selected[0]['output_times']]
    ref={'status':'OK','reference_converged':True,'times':selected[0]['output_times'],
         'states':states,'uncertainty_abs':[[0.0] for _ in states]}
    put(p1/'references.json',{'ode-scalar-decay':ref})
    put(p1/'references.json.provenance.json',{'ode-scalar-decay':{'receipt_sha256':'a'*64}})
    jobs=[];physical_inputs={}
    for case in selected:
        physical=physical_input_digest(case)
        physical_inputs[physical]={}
        for sid,spec in solvers.items():
            req={'case':case,'solver_id':sid,'probes':frozen_points(case)}
            adapter_binding={'status':'BOUND','files':{}}
            identity={'request':req,'solver_spec':spec,'source_binding_sha256':source_hash,
                      'adapter_binding':adapter_binding,'physical_input_sha256':physical}
            job={'job_id':digest(identity),'request':req,'solver_spec':spec,'solver_id':sid,
                 'physical_input_sha256':physical,'case_sha256':digest(case),
                 'source_binding_sha256':source_hash,'adapter_binding':adapter_binding}
            jobs.append(job)
            folder=p1/'probes/jobs'/job['job_id'];folder.mkdir(parents=True)
            parity={'status':'PASS_DIAGNOSTIC','case_sha256':digest(case),
                    'probes_sha256':digest(req['probes'])}
            put(folder/'request.json',req)
            put(folder/'adapter.stdout',{'status':'OK','case_id':case['case_id'],'probes':[]})
            put(folder/'oracle.stdout',parity)
            receipt={'job':job,'adapter_binding':adapter_binding,'source_binding_sha256':source_hash,
                     'adapter_exit_code':0,'oracle_exit_code':0,'adapter_status':'OK','parity':parity,
                     'files_sha256':{name:file_digest(folder/name) for name in ('request.json','adapter.stdout','oracle.stdout')}}
            receipt['receipt_sha256']=digest(receipt)
            put(folder/'receipt.json',receipt)
            physical_inputs[physical][sid]={'adapter_status':'OK','parity':parity,'job_id':job['job_id']}
    put(p1/'probes/plan.json',{'source_binding_sha256':source_hash,
        'config_sha256':file_digest(frozen/'screening_config.json'),
        'selected_adapters':list(solvers),'jobs':jobs})
    put(p1/'probes/mapping.json',{'source_binding_sha256':source_hash,
                                 'physical_inputs':physical_inputs})
    return root,p1,selected,solvers


def reference_ok(case,ref,provenance,root,source_hash):
    return case['case_id']=='ode-scalar-decay' and ref is not None and provenance is not None


def test_pass_refusal_and_typed_capability_are_distinct(tmp_path,monkeypatch):
    root,p1,cases,solvers=fixture(tmp_path,monkeypatch)
    out=tmp_path/'p2'
    summary=prepare_p2.prepare(root,p1,out,validate_reference=reference_ok)
    assert summary['gate_count']==6
    screen=json.loads((out/'screening_config_gated.json').read_text())
    alloc=json.loads((out/'allocation_config_gated.json').read_text())
    gate=screen['case_gates']
    assert all(g['status']=='PASS' and g['reason']=='REFERENCE_AND_MODEL_PROBE_PREREQUISITES_VALID'
               for g in gate['ode-scalar-decay'].values())
    assert all(g['status']=='PASS' and g['reason']=='EXPECTED_REFUSAL_ONLY'
               for g in gate['dae-index1-inconsistent'].values())
    assert all(g['status']=='HOLD' and g['reason']=='TYPED_UNSUPPORTED_CAPABILITY'
               for g in gate['dae-pendulum-index3'].values())
    assert set(alloc['case_gates'])=={'ode-scalar-decay','dae-index1-inconsistent'}
    evidence=json.loads((out/'prerequisite_evidence.json').read_text())['entries']
    refusal=[e for e in evidence if e['case_id']=='dae-index1-inconsistent']
    assert all(e['initial_algebraic_residual']==-1.0 and e['reference_gate'] is None for e in refusal)
    assert all(len(g['evidence_sha256'])==64 for gs in gate.values() for g in gs.values())


def test_corrupted_raw_probe_hash_holds_only_affected_adapter(tmp_path,monkeypatch):
    root,p1,cases,solvers=fixture(tmp_path,monkeypatch)
    plan=json.loads((p1/'probes/plan.json').read_text())
    job=next(j for j in plan['jobs'] if j['request']['case']['case_id']=='ode-scalar-decay'
             and j['solver_id']=='native-rodas5p-fast')
    (p1/'probes/jobs'/job['job_id']/'adapter.stdout').write_text('{"status":"WRONG"}')
    prepare_p2.prepare(root,p1,tmp_path/'p2',validate_reference=reference_ok)
    gate=json.loads((tmp_path/'p2/screening_config_gated.json').read_text())['case_gates']['ode-scalar-decay']
    assert gate['native-rodas5p-fast']['status']=='HOLD'
    assert gate['native-rodas5p-fast']['reason']=='PROBE_RAW_RECEIPT_INVALID'
    assert gate['scipy-radau']['status']=='PASS'


def test_missing_reference_holds_standard_but_not_refusal(tmp_path,monkeypatch):
    root,p1,cases,solvers=fixture(tmp_path,monkeypatch)
    (p1/'references.json').unlink()
    prepare_p2.prepare(root,p1,tmp_path/'p2',validate_reference=reference_ok)
    gates=json.loads((tmp_path/'p2/screening_config_gated.json').read_text())['case_gates']
    assert all(x['status']=='HOLD' for x in gates['ode-scalar-decay'].values())
    assert all(x['reason']=='EXPECTED_REFUSAL_ONLY' for x in gates['dae-index1-inconsistent'].values())


def test_source_mismatch_holds_without_launch(tmp_path,monkeypatch):
    root,p1,cases,solvers=fixture(tmp_path,monkeypatch)
    plan=json.loads((p1/'probes/plan.json').read_text());plan['source_binding_sha256']='0'*64
    put(p1/'probes/plan.json',plan)
    prepare_p2.prepare(root,p1,tmp_path/'p2',validate_reference=reference_ok)
    gates=json.loads((tmp_path/'p2/screening_config_gated.json').read_text())['case_gates']
    assert gates['ode-scalar-decay']['native-rodas5p-fast']['status']=='HOLD'
    assert gates['dae-index1-inconsistent']['native-rodas5p-fast']['status']=='HOLD'


def test_reference_uncertainty_above_frozen_five_percent_holds(tmp_path,monkeypatch):
    root,p1,cases,solvers=fixture(tmp_path,monkeypatch)
    path=p1/'references.json'; refs=json.loads(path.read_text())
    case=cases[0];budget=case['atol'][0]+case['rtol']
    refs['ode-scalar-decay']['uncertainty_abs']=[[.051*budget] for _ in case['output_times']]
    put(path,refs)
    prepare_p2.prepare(root,p1,tmp_path/'p2',validate_reference=reference_ok)
    gate=json.loads((tmp_path/'p2/screening_config_gated.json').read_text())['case_gates']['ode-scalar-decay']
    assert all(x['reason']=='REFERENCE_CONVERGENCE_OR_UNCERTAINTY_HOLD' for x in gate.values())


def test_mapping_identity_mismatch_holds_only_affected_adapter(tmp_path,monkeypatch):
    root,p1,cases,solvers=fixture(tmp_path,monkeypatch)
    path=p1/'probes/mapping.json';mapping=json.loads(path.read_text())
    key=physical_input_digest(cases[0]);mapping['physical_inputs'][key]['native-rodas5p-fast']['job_id']='0'*64
    put(path,mapping)
    prepare_p2.prepare(root,p1,tmp_path/'p2',validate_reference=reference_ok)
    gate=json.loads((tmp_path/'p2/screening_config_gated.json').read_text())['case_gates']['ode-scalar-decay']
    assert gate['native-rodas5p-fast']['reason']=='PROBE_IDENTITY_OR_MAPPING_MISMATCH'
    assert gate['scipy-radau']['status']=='PASS'
