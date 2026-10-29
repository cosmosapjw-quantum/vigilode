#!/usr/bin/env python3
"""A02 original-global-origin exact reference; reuses frozen A01 proof functions."""
import importlib.util
import json
import struct
import sys
from fractions import Fraction as F
from pathlib import Path
ROOT=Path(__file__).resolve().parents[5]
TESTS=ROOT/'crates/rodas5p-integrators/tests'
def load(name):
    spec=importlib.util.spec_from_file_location(name,TESTS/(name+'.py'))
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module
u=load('packet_unequal_fraction_oracle')
def bits(v):return F.from_float(struct.unpack('>d',bytes.fromhex(v['bits'].removeprefix('0x')))[0])
def q(x):return F.from_float(x)
def pair(z):return q(z[0]),q(z[1])
def within(point,initial,xi,weights,width,chi,t,refs):
    if refs is not None:
        boxes=[]
        for order,limit in [(40,F(1,2**120)),(48,F(1,2**160))]:
            modes,rho,_,_=refs[order]
            s=sum(weights,F(0));lip=max(abs(width*x) for x in xi)+6*abs(chi)*s**3
            eps=u.exp_plus(lip*t)*t*rho
            assert eps<=limit
            values=u.evaluate(modes,t)
            boxes.append(([u.box(v,eps) for v in values],u.box(u.sum_weighted(values,weights),s*eps),eps))
        assert boxes[1][2]<=boxes[0][2]
        modes,mean,_=boxes[1]
    else:
        modes=[]
        for z,x in zip(initial,xi):
            phase=(width*x+(chi*(z[0]*z[0]+z[1]*z[1]) if len(initial)==1 else 0))*t
            assert abs(phase)<=1
            modes.append(u.trig_orbit(z,phase))
        mean=((F(0),F(0)),(F(0),F(0)))
        mean=(tuple(sum(weights[j]*modes[j][0][k] for j in range(len(weights))) for k in range(2)),tuple(sum(weights[j]*modes[j][1][k] for j in range(len(weights))) for k in range(2)))
    assert all(u.disk_contains(z,r,b) for z,r,b in zip(point['modes'],point['mode_radii'],modes))
    assert u.disk_contains(point['mean'],point['mean_radius'],mean)
    return modes

def verify(native,manifest):
    count=0;negative=False;temporal=False
    for row in native['fixtures']:
        if row.get('failure') or row.get('setup_refusal'):
            if row['id']=='F_SUBNORMAL':continue
            raise AssertionError(row['failure'])
        fixture=manifest['fixtures'][row['id']];m=manifest['models'][fixture['model_id']]
        initial=[tuple(map(bits,z)) for z in m['initial_centers']]
        weights=list(map(bits,m['weights']));xi=list(map(bits,m['xi']));width=bits(m['width']);chi=bits(m['chi']);origin=q(row['t_start'])
        if row['id']=='F_UNEQUAL_U':
            eta=F(1,2**30);initial=[u.add(z,p) for z,p in zip(initial,[(eta,F(0)),(F(0),-eta),(-eta,F(0))])]
        if row['id'] in ('F_INHERITED','F_STATIONARY_SMALL_W'):
            radii=list(map(bits,fixture['initial_component_radii']))
            shifts=[(radii[0],F(0)),(F(0),-radii[1])]+([(-radii[2],F(0))] if len(radii)==3 else [])
            initial=[u.add(z,p) for z,p in zip(initial,shifts)]
        refs=None
        if row['id'].startswith('F_UNEQUAL'):
            refs={order:u.reference(initial,xi,weights,width,chi,F(1,32),order) for order in (40,48)}
            assert all(refs[40][0][j]==refs[48][0][j][:41] for j in range(3))
        for point in row['observations']+row['historical']:
            t=q(point['time'])-origin
            modes=within(point,initial,xi,weights,width,chi,t,refs);count+=len(weights)+1
            if row['id']=='F_OFFSET' and point['time']==float.fromhex('0x1.ccccccccccccdp-1'):
                exact=q(point['time'])-origin;lo,hi=map(q,point['offset']);assert lo<=exact<=hi and lo<hi
                rounded=q(point['local_time']);assert exact!=rounded
                assert not rounded<=exact<=rounded # mutation discarding D rejected
                temporal=True
            if row['id']=='F_UNEQUAL_P' and point['time']==0.03125:
                center=list(point['modes'][0]);center[0]+=1
                assert not u.disk_contains(center,point['mode_radii'][0],modes[0])
                assert not u.disk_contains(point['modes'][0],0.0,modes[0]);negative=True
    assert negative and temporal
    return {'status':'PASS_SCOPED','disk_checks':count,'negative_endpoint_controls':'REJECTED','temporal_binding_mutation':'REJECTED','production':'HOLD'}
def verify_invalid(invalid,manifest):
    outcomes=[]
    for row in invalid:
        if row['id'] not in ('SUBNORMAL_TIME','SUBNORMAL_RADIUS'):
            continue
        name=row['id'];fixture=manifest['fixtures']['F_SUBNORMAL' if name=='SUBNORMAL_TIME' else 'F_INHERITED']
        assert row['model_id']==fixture['model_id']
        assert row['original_fixture']==fixture
        assert row.get('work') is not None, 'setup/attempt work must survive refusal'
        m=manifest['models'][fixture['model_id']]
        initial=[tuple(map(bits,z)) for z in m['initial_centers']]
        weights=list(map(bits,m['weights']));xi=list(map(bits,m['xi']))
        width=bits(m['width']);chi=bits(m['chi']);origin=q(row['t_start'])
        radii=[F.from_float(struct.unpack('>d',bytes.fromhex(x.removeprefix('0x')))[0]) for x in row['initial_radius_bits']]
        if name=='SUBNORMAL_TIME':
            assert len(initial)==1 and initial[0]==(F(1,2**1074),F(0))
            assert width==chi==0 and xi==[F(0)] and radii==[F(0)]
            assert origin==0
            assert list(map(q,row['requested_times']))==[F(0),F(1,2**1074)]
            cfg=row['effective_config']
            assert all(q(cfg[k])==F(1,2**1074) for k in ('t_end','initial_h','min_h','max_h'))
        else:
            assert radii==[F(1,2**1074)]*2, 'do not reuse default inherited radii'
            assert chi==0 and origin==bits(manifest['configs'][fixture['config_id']]['t_start'])
            assert list(map(q,row['requested_times']))==list(map(bits,fixture['requested_times']))
            # Exact rational perturbations, formed before products/rotation.
            initial=[u.add(initial[0],(radii[0],F(0))),u.add(initial[1],(F(0),-radii[1]))]
        checked=0
        for point in row.get('observations',[]):
            t=q(point['time'])-origin
            within(point,initial,xi,weights,width,chi,t,None)
            checked+=len(initial)+1
        refused='setup_refusal' in row or row.get('result')!='Ok(())'
        if not refused:
            assert len(row.get('observations',[]))==len(row['requested_times'])
        outcomes.append({'id':name,'status':'TYPED_REFUSAL' if refused else 'INCLUSION_PASS',
                         'disk_checks':checked,'exact_input':'BINARY64_FRACTION',
                         'refusal_counted_as_inclusion':False})
    assert {x['id'] for x in outcomes}=={'SUBNORMAL_TIME','SUBNORMAL_RADIUS'}
    return outcomes

if __name__=='__main__':
    native=json.loads(Path(sys.argv[1]).read_text())
    invalid=json.loads(Path(sys.argv[2]).read_text())
    manifest=json.loads((Path(__file__).parent.parent/'supplement_02/FROZEN_ACCEPTANCE_INPUTS_V2.json').read_text())
    result=verify(native,manifest)
    result['invalid_arms']=verify_invalid(invalid,manifest)
    result['successful_invalid_disk_checks']=sum(x['disk_checks'] for x in result['invalid_arms'])
    result['typed_invalid_refusals']=sum(x['status']=='TYPED_REFUSAL' for x in result['invalid_arms'])
    Path(sys.argv[3]).write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
