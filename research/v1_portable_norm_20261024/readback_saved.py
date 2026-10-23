"""Read original S2 bytes and outputs only; never run a compiler/solver/benchmark."""
from pathlib import Path
from fractions import Fraction as Q
from collections import Counter
import csv,hashlib,json,math,struct,sys,time

started=time.monotonic(); root=Path(sys.argv[1]); output=Path(sys.argv[2])
manifest=json.loads((root/'MANIFEST.json').read_text())
for item in manifest['files']:
    b=(root/item['path']).read_bytes()
    assert len(b)==item['bytes'],item['path']
    assert hashlib.sha256(b).hexdigest()==item['sha256'],item['path']
    assert hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest()==item['git_blob'],item['path']
original_budget=json.loads((root/'BUDGET.json').read_text())
for entry in original_budget['entries']:
    assert hashlib.sha256((root/entry['log']).read_bytes()).hexdigest()==entry['sha256']
assert math.isclose(sum(e['wall_seconds'] for e in original_budget['entries']),original_budget['total_seconds'],rel_tol=0,abs_tol=1e-12)
assert original_budget['closed'] and original_budget['total_seconds']<original_budget['cap_seconds']
assert '12 passed; 0 failed' in (root/'evidence/GREEN_EXEC.log').read_text()

def number(h):
    x=struct.unpack('>d',bytes.fromhex(h))[0]
    assert math.isfinite(x)
    return Q.from_float(x)

inputs=(root/'evidence/ORACLE_INPUT.txt').read_text().splitlines()
outputs=(root/'evidence/ORACLE_OUTPUT.txt').read_text().splitlines()
saved=json.loads((root/'evidence/ORACLE_EXACT.json').read_text())
assert len(inputs)==len(outputs)==len(saved)==741
counts=Counter();max_width=Q(0)
for i,(line,answer,record) in enumerate(zip(inputs,outputs,saved)):
    mode,*hexbits=line.split(); values=[number(h) for h in hexbits]
    if mode=='S':
        assert len(values)==3
        exact=abs(sum(values,Q(0)))
    else:
        assert mode=='N' and len(values)==63
        e,s=values[:30],values[30:]; assert all(x>0 for x in s)
        # Fixed row geometry, independently expressed by explicit dependent indices.
        full=[*e[:23],-e[23],e[23],-(e[24]+e[25]),e[24],e[25],
              -(e[26]+e[27]+e[28]),e[26],e[27],e[28],e[29]]
        assert len(full)==33
        exact=sum((x/y)**2 for x,y in zip(full,s))/33
    lh,hh,exponent,status=answer.split()
    lo=number(lh)*Q(2)**int(exponent);hi=number(hh)*Q(2)**int(exponent)
    assert 0<=lo<=exact<=hi,(i,'enclosure')
    assert record['case']==i and record['kind']==mode and record['enclosed']
    assert exact==Q(int(record['exact_num']),int(record['exact_den']))
    assert status==record['decision']
    if mode=='N':
        expected='Accept' if hi<=1 else 'Reject' if lo>1 else 'Indeterminate'
        assert status==expected,(i,'decision')
        if status=='Accept': assert exact<=1
        if status=='Reject': assert exact>1
        if exact:max_width=max(max_width,(hi-lo)/exact)
    else:assert status=='S'
    counts[status]+=1
summary=json.loads((root/'ORACLE_SUMMARY.json').read_text())
assert dict(counts)==summary['counts'] and float(max_width)==summary['max_relative_width']
raw=list(csv.DictReader((root/'evidence/KERNEL_COST.log').open()))
assert len(raw)==36 and len({(r['case'],r['block'],r['arm']) for r in raw})==36
from statistics import median
cost=json.loads((root/'COST_SUMMARY.json').read_text())
for result in cost['rows']:
    c=str(result['case']);r=[x for x in raw if x['case']==c]
    assert len(r)==12
    for x in r:assert int(x['repeats'])==4000 and int(x['total_ns'])>0 and math.isfinite(float(x['checksum']))
    values={arm:[int(x['total_ns'])/4000 for x in r if x['arm']==arm] for arm in ['0','1']}
    assert median(values['0'])==result['rust_interval_ns_median']
    assert median(values['1'])==result['c99_point_ns_median']
    ratios=[]
    for b in range(6):
        y={x['arm']:int(x['total_ns']) for x in r if int(x['block'])==b}
        ratios.append(y['1']/y['0'])
    assert math.isclose(median(ratios),result['paired_C_over_Rust'],rel_tol=1e-15)
result={'status':'PASS_SAVED_S2_READBACK','verified_manifest_payloads':len(manifest['files']),
        'verified_original_budget_logs':len(original_budget['entries']),
        'verified_fraction_records':len(inputs),'counts':dict(counts),
        'verified_cost_cells':len(raw),'verified_cost_pair_medians':3,
        'source_rust_blob':next(x['git_blob'] for x in manifest['files'] if x['path']=='source/portable_norm.rs'),
        'original_validation_seconds':original_budget['total_seconds'],
        'readback_wall_seconds':time.monotonic()-started,
        'new_compiler_invocations':0,'new_Rust_test_runs':0,'new_native_ODE_runs':0,
        'new_timing_samples':0,'NCP_submissions':0,
        'scope':'saved-artifact and arithmetic readback, same author; not fresh personnel review'}
output.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
