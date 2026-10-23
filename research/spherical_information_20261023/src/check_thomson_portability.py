#!/usr/bin/env python3
"""Loader-only provenance checks, plus AST identity of existing scientific kernels."""
from pathlib import Path
import ast, hashlib, importlib.util, json, tempfile
BASE=Path(__file__).resolve().parents[1]
CURRENT=BASE/'src/thomson_demo.py'
OLD=BASE/'evidence/thomson_demo_pre_portable_loader.py'

def functions(path):
    return {n.name:ast.dump(n,include_attributes=False) for n in ast.parse(path.read_text()).body if isinstance(n,ast.FunctionDef)}
old,new=functions(OLD),functions(CURRENT)
names=['grid','quadrupoles','scalar_scatter','scalar_exact','lift','polarized_scatter','polarized_exact','lowrank_exact','local_coeff','from_coeff','entropic_distance','timer']
unchanged={name:old[name]==new[name] for name in names}
assert all(unchanged.values())
old_results=json.loads((BASE/'results/thomson_before_portable_loader.json').read_text())
assert hashlib.sha256(OLD.read_bytes()).hexdigest()==old_results['implementation']['source_sha256']
spec=importlib.util.spec_from_file_location('thomson_portability',CURRENT)
mod=importlib.util.module_from_spec(spec);spec.loader.exec_module(mod)
with tempfile.TemporaryDirectory(prefix='thomson-oracle-mismatch-') as td:
    bad=Path(td)/'oracle.py'
    bad.write_text('raise RuntimeError("this must never execute")\n')
    mismatch=mod.fvm_reference_checks(oracle=bad)
    assert mismatch['status']=='unmatched'
    assert 'cases' not in mismatch
loader=json.loads((BASE/'results/thomson_loader_results.json').read_text())
assert loader['existing_fvm_reference']['actual_checkout_head'] is None
assert loader['existing_fvm_reference']['source_commit_basis']=='bundled_origin_manifest'
assert loader['status']=='pass_scoped'
record={'schema':'spherical-information.thomson-portability-checks.v1','status':'pass_scoped','source_sha256':hashlib.sha256(CURRENT.read_bytes()).hexdigest(),'preserved_scientific_source_sha256':hashlib.sha256(OLD.read_bytes()).hexdigest(),'scientific_function_AST_unchanged':unchanged,'mismatched_source_rejected_before_import':mismatch,'bundled_loader_execution':'results/thomson_loader_results.json','scientific_results_reused':'results/thomson_before_portable_loader.json'}
(BASE/'results/thomson_portability_checks.json').write_text(json.dumps(record,indent=2)+'\n')
print(json.dumps({'status':record['status'],'unchanged_scientific_functions':len(unchanged),'mismatch_result':mismatch['status']}))
