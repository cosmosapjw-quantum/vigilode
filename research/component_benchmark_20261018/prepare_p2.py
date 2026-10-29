#!/usr/bin/env python3
"""Pure P1 evidence validation and P2 config preparation; runs no numerical job."""
from __future__ import annotations

import argparse
import copy
import importlib.util
import json
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT / 'tools') not in sys.path:
    sys.path.insert(0, str(ROOT / 'tools'))
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from component_benchmark.analysis import reference_gate
from component_benchmark.models import rhs
from component_benchmark.protocol import digest, file_digest, physical_input_digest, strict_load
from component_benchmark.runner import atomic_json, source_binding
from component_benchmark.probe_campaign import _receipt_valid as probe_receipt_valid, frozen_points

ADAPTERS = {
    'native-':'native-rodas5p-fast',
    'diffsol-':'diffsol-bdf',
    'scipy-':'scipy-radau',
    'vode-':'scipy-radau',
    'julia-':'julia-rodas5p',
    'hairer-':'hairer-radau5',
}


def reference_validator(root):
    """Load the existing CLI validation routine without changing frozen source."""
    path=Path(root)/'tools/component_benchmark.py'
    spec=importlib.util.spec_from_file_location('_frozen_component_cli',path)
    module=importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.validated_reference


def adapter_for(solver_id):
    for prefix,adapter in ADAPTERS.items():
        if solver_id.startswith(prefix):return adapter
    return None


def initial_algebraic_residual(case):
    matrix=case.get('mass_matrix')
    if not isinstance(matrix,list):return None
    rows=[i for i,row in enumerate(matrix) if all(value==0 for value in row)]
    if len(rows)!=1:return None
    return rhs(case,case['t_span'][0],case['initial_state'])[rows[0]]


def _read(path, files):
    path=Path(path)
    if not path.is_file():return None
    files[str(path.resolve())]=file_digest(path)
    try:
        return strict_load(path.read_text())
    except (UnicodeError, ValueError, TypeError):
        return None


def _probe(case, adapter, probes_dir, plan, mapping, source_hash, files, validate_probe):
    physical=physical_input_digest(case)
    cell=(mapping.get('physical_inputs') or {}).get(physical,{}).get(adapter)
    jobs=[j for j in plan.get('jobs',[]) if j.get('physical_input_sha256')==physical and j.get('solver_id')==adapter]
    if len(jobs)!=1 or not isinstance(cell,dict):return False,'PROBE_PLAN_OR_MAPPING_MISSING',None
    job=jobs[0]; folder=probes_dir/'jobs'/job['job_id']
    receipt=_read(folder/'receipt.json',files)
    if not isinstance(receipt,dict):return False,'PROBE_RAW_RECEIPT_MISSING',None
    try:
        valid=validate_probe(folder,receipt,job)
    except (OSError,ValueError,KeyError,TypeError):
        valid=False
    if not valid:return False,'PROBE_RAW_RECEIPT_INVALID',receipt.get('receipt_sha256')
    request=_read(folder/'request.json',files)
    adapter_output=_read(folder/'adapter.stdout',files)
    oracle_output=_read(folder/'oracle.stdout',files)
    parity=receipt.get('parity')
    try:
        identity_ok=(job.get('source_binding_sha256')==source_hash and receipt.get('source_binding_sha256')==source_hash
            and job.get('physical_input_sha256')==physical
            and job.get('case_sha256')==digest(job['request']['case'])
            and physical_input_digest(job['request']['case'])==physical
            and job.get('request',{}).get('probes')==frozen_points(job['request']['case'])
            and request==job.get('request')
            and cell.get('job_id')==job['job_id']
            and cell.get('adapter_status')==receipt.get('adapter_status')
            and cell.get('parity')==parity)
    except (KeyError,TypeError,ValueError):
        identity_ok=False
    if not identity_ok:
        return False,'PROBE_IDENTITY_OR_MAPPING_MISMATCH',receipt.get('receipt_sha256')
    if (receipt.get('adapter_exit_code')!=0 or receipt.get('oracle_exit_code')!=0
        or receipt.get('adapter_status')!='OK' or not isinstance(adapter_output,dict)
        or adapter_output.get('status')!='OK'):
        return False,'PROBE_ADAPTER_NOT_OK',receipt.get('receipt_sha256')
    if (not isinstance(oracle_output,dict) or oracle_output!=parity
        or not isinstance(parity,dict) or parity.get('status')!='PASS_DIAGNOSTIC'
        or parity.get('case_sha256')!=job['case_sha256']
        or parity.get('probes_sha256')!=digest(job['request']['probes'])):
        return False,'PROBE_PARITY_NOT_PASS_DIAGNOSTIC',receipt.get('receipt_sha256')
    return True,'PROBE_PASS_DIAGNOSTIC',receipt['receipt_sha256']


def prepare(root,p1,output,validate_reference=None,validate_probe=None):
    root,p1,output=Path(root).resolve(),Path(p1).resolve(),Path(output).resolve()
    output.mkdir(parents=True,exist_ok=True)
    files={}
    screening=_read(p1/'frozen/screening_config.json',files)
    allocation=_read(p1/'frozen/allocation_config.json',files)
    reference_plan=_read(p1/'references/plan.json',files)
    refs=_read(p1/'references.json',files)
    provenance=_read(p1/'references.json.provenance.json',files)
    probe_plan=_read(p1/'probes/plan.json',files)
    mapping=_read(p1/'probes/mapping.json',files)
    if not isinstance(screening,dict) or not isinstance(screening.get('cases'),list) or not isinstance(screening.get('solvers'),dict):
        raise ValueError('P1_SCREENING_CONFIG_MISSING_OR_INVALID')
    if not isinstance(allocation,dict) or not isinstance(allocation.get('cases'),list) or not isinstance(allocation.get('solvers'),dict):
        raise ValueError('P1_ALLOCATION_CONFIG_MISSING_OR_INVALID')
    reference_plan=reference_plan if isinstance(reference_plan,dict) else {}
    refs=refs if isinstance(refs,dict) else {}
    provenance=provenance if isinstance(provenance,dict) else {}
    probe_plan=probe_plan if isinstance(probe_plan,dict) else {}
    mapping=mapping if isinstance(mapping,dict) else {}
    source_hash=digest(source_binding(root))
    reference_source_valid=(digest(reference_plan.get('source_binding'))==source_hash)
    probe_source_valid=(probe_plan.get('source_binding_sha256')==source_hash
        and mapping.get('source_binding_sha256')==source_hash
        and probe_plan.get('config_sha256')==files[str((p1/'frozen/screening_config.json').resolve())])
    if validate_reference is None:validate_reference=reference_validator(root)
    if validate_probe is None:validate_probe=probe_receipt_valid
    # Test seams are explicit. Production always uses the frozen validators.
    cases=screening['cases']; solvers=screening['solvers']
    gate_matrix={}; evidence=[]; probe_cache={}
    for case in cases:
        cid=case['case_id'];physical=physical_input_digest(case)
        try:
            initial_residual=initial_algebraic_residual(case) if case.get('reference_policy')=='expected_failure' else None
        except (KeyError,ValueError,TypeError,ArithmeticError):
            initial_residual=None
        ref=refs.get(cid); prov=provenance.get(cid)
        ref_receipt_sha=prov.get('receipt_sha256') if isinstance(prov,dict) else None
        if isinstance(prov,dict):
            refpath=Path(prov.get('receipt_path',''))
            if not refpath.is_absolute():refpath=root/refpath
            if refpath.is_file():files[str(refpath.resolve())]=file_digest(refpath)
        if reference_source_valid:
            try:
                ref_valid=validate_reference(case,ref,prov,root,source_hash)
            except (OSError,ValueError,KeyError,TypeError):
                ref_valid=False
        else:ref_valid=False
        ref_gate=None
        if (ref_valid and isinstance(ref,dict) and ref.get('status')=='OK'
            and ref.get('reference_converged') is True and ref.get('times')==case['output_times']):
            try:
                ref_gate=reference_gate(case,ref.get('states'),ref.get('states'),ref.get('uncertainty_abs'))
            except (KeyError,TypeError,ValueError,ArithmeticError):
                ref_gate={'status':'HOLD','reason':'INVALID_REFERENCE_NUMERICS'}
        ref_ok=ref_gate is not None and ref_gate.get('status')=='PASS' and ref_gate.get('max_uncertainty_budget_ratio',1) <= .05
        gate_matrix[cid]={}
        for sid in solvers:
            adapter=adapter_for(sid)
            probe_ok=False;probe_reason='PROBE_ADAPTER_MAPPING_MISSING';probe_sha=None
            if probe_source_valid and adapter is not None and adapter in probe_plan.get('selected_adapters',[]):
                cache_key=(physical,adapter,source_hash)
                if cache_key not in probe_cache:
                    probe_cache[cache_key]=_probe(case,adapter,p1/'probes',probe_plan,mapping,source_hash,files,validate_probe)
                probe_ok,probe_reason,probe_sha=probe_cache[cache_key]
            if case.get('reference_policy')=='capability_gap':
                reason='TYPED_UNSUPPORTED_CAPABILITY'
            elif case.get('reference_policy')=='expected_failure':
                if cid!='dae-index1-inconsistent' or initial_residual!=-1.0:
                    reason='EXPECTED_REFUSAL_SOURCE_CASE_DRIFT'
                else:
                    reason='EXPECTED_REFUSAL_ONLY' if probe_ok else probe_reason
            elif not reference_source_valid or not probe_source_valid:
                reason='P1_SOURCE_OR_CONFIG_HASH_MISMATCH'
            elif not ref_valid:reason='REFERENCE_RAW_IDENTITY_HOLD'
            elif not ref_ok:reason='REFERENCE_CONVERGENCE_OR_UNCERTAINTY_HOLD'
            elif not probe_ok:reason=probe_reason
            else:reason='REFERENCE_AND_MODEL_PROBE_PREREQUISITES_VALID'
            status='PASS' if reason in ('REFERENCE_AND_MODEL_PROBE_PREREQUISITES_VALID','EXPECTED_REFUSAL_ONLY') else 'HOLD'
            chain={'case_id':cid,'solver_id':sid,'physical_input_sha256':physical,
                   'source_binding_sha256':source_hash,'reference_receipt_sha256':ref_receipt_sha,
                   'reference_result_sha256':digest(ref) if isinstance(ref,dict) else None,
                   'reference_gate':ref_gate,'probe_adapter':adapter,
                   'probe_receipt_sha256':probe_sha,'probe_reason':probe_reason,
                       'status':status,'reason':reason,
                       'initial_algebraic_residual':initial_residual,
                       'reference_source_valid':reference_source_valid,'probe_source_valid':probe_source_valid}
            evsha=digest(chain)
            gate_matrix[cid][sid]={'status':status,'reason':reason,'evidence_sha256':evsha}
            evidence.append({'evidence_sha256':evsha,**chain})
    for label,config in (('screening',screening),('allocation',allocation)):
        gated=copy.deepcopy(config)
        gated['case_gates']={c['case_id']:{sid:gate_matrix[c['case_id']][sid] for sid in gated['solvers']}
                             for c in gated['cases']}
        atomic_json(output/(label+'_config_gated.json'),gated)
    summary={'schema':'vigilode-component-p2-prerequisites-v1','source_binding_sha256':source_hash,
             'input_files_sha256':files,'case_count':len(cases),'solver_count':len(solvers),
             'gate_count':len(evidence),'gate_status_counts':dict(Counter(e['status'] for e in evidence)),
             'gate_reason_counts':dict(Counter(e['reason'] for e in evidence)),
             'evidence_chain_sha256':digest(evidence),'scientific_admission':'HOLD',
             'scope':'P2 admission prerequisites only; no numerical execution'}
    atomic_json(output/'prerequisite_evidence.json',{'schema':'vigilode-component-p2-gate-evidence-v1','entries':evidence})
    atomic_json(output/'prerequisite_validation.json',summary)
    return summary


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--root',type=Path,default=ROOT)
    parser.add_argument('--p1',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    args.output.mkdir(parents=True,exist_ok=False)
    print(json.dumps(prepare(args.root,args.p1,args.output),sort_keys=True,allow_nan=False))


if __name__=='__main__':main()
