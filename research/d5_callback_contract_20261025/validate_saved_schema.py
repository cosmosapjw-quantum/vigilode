#!/usr/bin/env python3
"""Additive complete-field/coverage review for the unchanged saved D5 corpus.

This closes the concrete zip-shortening risk in the original numerical oracle.
It never evaluates the model, callback, derivative oracle or old campaigns.
"""
import copy,hashlib,itertools,json,math
from collections import Counter
from pathlib import Path

root=Path(__file__).resolve().parent
path=root/'evidence/points.jsonl'
def unique_object(pairs):
    result={}
    for key,value in pairs:
        if key in result:raise ValueError('duplicate JSON key '+key)
        result[key]=value
    return result
rows=[json.loads(line,object_pairs_hook=unique_object) for line in path.read_text().splitlines()]

def vec(x,n):
    assert type(x) is list and len(x)==n,'incomplete vector'
    assert all(type(v) in (int,float) and math.isfinite(v) for v in x),'invalid finite value'
    return len(x)
def matrix(x):
    assert type(x) is list and len(x)==2,'incomplete matrix'
    return sum(vec(row,2) for row in x)
def integer_key(key,n):
    assert type(key) is list and len(key)==n and all(type(i) is int for i in key)
    return tuple(key)
def validate(items):
    wanted={'point':729,'fd':49,'finite_wrong_j':1,'input_refusal':7,'fd_refusal':7,'summary':1}
    assert Counter(r['kind'] for r in items)==wanted
    assert len(items)==794 and items[-1]['kind']=='summary'
    points=[];stencils=[];ids=[];names=[]
    fields={'rhs':0,'jacobian':0,'jvp':0,'mass_residual':0,'fd_estimate':0}
    for row in items:
        kind=row['kind']
        if kind=='point':
            points.append(integer_key(row['i'],3));vec(row['y'],2)
            fields['rhs']+=vec(row['rhs'],2)
            fields['jacobian']+=matrix(row['jac'])
            fields['jvp']+=vec(row['jv'],2)
            vec([row['mass_residual']],1);fields['mass_residual']+=1
        elif kind=='fd':
            assert type(row['it']) is int and type(row['power']) is int
            stencils.append((row['it'],row['power']));vec(row['y'],2)
            fields['fd_estimate']+=vec(row['estimate'],2)
        elif kind=='finite_wrong_j':
            vec(row['y'],2);vec(row['rhs'],2);matrix(row['jac']);assert row['admitted'] is True
        elif kind=='input_refusal':
            assert type(row['id']) is int;ids.append(row['id'])
            assert type(row['y_bits']) is list and len(row['y_bits'])==2
            assert all(type(x) is int and 0<=x<2**64 for x in [row['t_bits'],*row['y_bits']])
        elif kind=='fd_refusal':
            names.append(row['name'])
            assert all(type(x) is int and 0<=x<2**64 for x in [row['t_bits'],row['eps_bits']])
    assert len(points)==len(set(points))==729
    assert set(points)==set(itertools.product(range(9),repeat=3))
    assert len(stencils)==len(set(stencils))==49
    assert set(stencils)==set(itertools.product(range(1,8),range(3,10)))
    assert len(ids)==len(set(ids))==7 and set(ids)==set(range(7))
    assert len(names)==len(set(names))==7 and set(names)=={'left','right','collapse','zero','negative','infinite','nan'}
    assert fields=={'rhs':1458,'jacobian':2916,'jvp':1458,'mass_residual':729,'fd_estimate':98}
    return fields

fields=validate(rows)
# Three concrete malformed-output controls exercise the exact review gap.
# These are temporary schema objects, not new scientific input or callback runs.
controls=[]
for name in ['short_rhs','short_fd_estimate','wrong_grid_index']:
    damaged=copy.deepcopy(rows)
    if name=='short_rhs':next(r for r in damaged if r['kind']=='point')['rhs']=[]
    elif name=='short_fd_estimate':next(r for r in damaged if r['kind']=='fd')['estimate']=[0.0]
    else:next(r for r in damaged if r['kind']=='point')['i']=[99,0,0]
    try:validate(damaged)
    except AssertionError:controls.append({'case':name,'result':'REJECTED'})
    else:raise AssertionError('malformed schema admitted: '+name)

result={'schema':'vigilode.d5.saved_schema.v1','status':'PASS_COMPLETE_FIELDS_AND_COVERAGE','input_sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'input_bytes':path.stat().st_size,'input_rows':len(rows),'actual_numeric_field_counts':fields,'actual_point_value_count':sum(fields[k] for k in ['rhs','jacobian','jvp','mass_residual']),'point_key_coverage':'EXACT_9x9x9','stencil_key_coverage':'EXACT_7x7','input_refusal_ids':'EXACT_0_TO_6','fd_refusal_names':'EXACT_FROZEN_7','duplicate_JSON_keys':0,'malformed_schema_controls':controls,'finding_closed':'original oracle zip() could truncate a malformed short array; full saved shapes, identities and measured field counts are separately proven here','original_oracle_and_result_preserved':True,'model_evaluations':0,'callback_evaluations':0,'old_science_replays':0,'script_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
out=root/'SCHEMA_REVIEW.json'
if out.exists():raise RuntimeError('schema review already exists')
out.write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
