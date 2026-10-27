#!/usr/bin/env python3
"""Independent exact-real original-stage residual check plus descriptive timings."""
from fractions import Fraction as Q
from pathlib import Path
import json, math, statistics, hashlib, datetime, sys

ROOT=Path(__file__).resolve().parent
FINAL=len(sys.argv)>1 and sys.argv[1]=='final'
PREFIX='FINAL_' if FINAL else ''
q=lambda x:Q.from_float(float(x))

def exact_oracle():
    path=ROOT/('EXACT_ORACLE_CORRECTION3_INPUT.json' if FINAL else 'EXACT_ORACLE_INPUT.json')
    data=json.loads(path.read_text())
    gamma=q(data['gamma']);a=[[q(v) for v in row] for row in data['a']];c=[[q(v) for v in row] for row in data['c']]
    rows=[]; total_components=0
    for cell in data['cases']:
        n=cell['n'];d=q(cell['d']);k=q(cell['kappa']);h=q(cell['h']);hg=h*gamma
        y=list(map(q,cell['y']));b=list(map(q,cell['forcing']));u=[list(map(q,row)) for row in cell['stages']]
        exact_maxima=[];ratios=[]
        for i in range(8):
            state=[y[z]+sum((a[i][j]*u[j][z] for j in range(i)),Q()) for z in range(n)]
            mean_state=sum(state,Q())/n;mean_u=sum(u[i],Q())/n
            maximum=Q()
            for z in range(n):
                fy=b[z]-d*state[z]+k*mean_state
                ju=-d*u[i][z]+k*mean_u
                mix=sum((c[i][j]*u[j][z] for j in range(i)),Q())
                residual=u[i][z]-hg*ju-gamma*mix-hg*fy
                maximum=max(maximum,abs(residual));total_components+=1
            bound=q(cell['bounds'][i]);assert maximum<=bound,(cell,i,float(maximum),float(bound))
            exact_maxima.append(float(maximum));ratios.append(float(bound/maximum) if maximum else None)
        rows.append({'n':n,'d':cell['d'],'kappa':cell['kappa'],'h':cell['h'],'maximum_input_magnitude':max(abs(float(v)) for v in y+b),'exact_residual_maxima':exact_maxima,'native_outward_bounds':cell['bounds'],'bound_over_exact_ratio':ratios,'pass':True})
    # Independent check of half-ULP radii against exact rational midpoints of
    # neighboring representable values, covering every finite normal exponent.
    radius_checks=0
    for exponent in (range(1,2047) if not FINAL else []):
        for mantissa in [0,1,1<<51,(1<<52)-1]:
            import struct
            z=struct.unpack('>d',struct.pack('>Q',(exponent<<52)|mantissa))[0]
            if exponent<=1:r=Q(1,1<<1074)
            else:r=Q(2)**(exponent-1023-53)
            up=math.nextafter(z,math.inf);down=math.nextafter(z,-math.inf)
            upper_gap=(q(up)-q(z)) if math.isfinite(up) else q(z)-q(down)
            lower_gap=q(z)-q(down)
            assert r>=max(upper_gap,lower_gap)/2
            radius_checks+=1
    if FINAL:
        radius_checks=json.loads((ROOT/'EXACT_ORACLE_RESULTS.json').read_text())['half_ulp_exponent_checks']
    result={'status':'pass_scoped','half_ulp_checks_reused':FINAL,'method':'Python Fraction exact rational evaluation of every component of the full original raw-U equations; no factorized residual identity is used by this oracle','input_sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'cells':len(rows),'stage_bounds':8*len(rows),'original_space_components':total_components,'half_ulp_exponent_checks':radius_checks,'rows':rows}
    (ROOT/(PREFIX+'EXACT_ORACLE_RESULTS.json')).write_text(json.dumps(result,indent=2)+'\n')
    return {k:v for k,v in result.items() if k!='rows'}

def timings():
    rows=[]
    for path in sorted(ROOT.glob(PREFIX+'BENCH_N*_W*.json')):
        data=json.loads(path.read_text());comparisons=[]
        for treatment in data['timing']:
            blocks={}
            for sample in treatment['samples']:
                by_arm=blocks.setdefault(sample['block'],{});by_arm.setdefault(sample['arm'],[]).append(sample['seconds_per_trajectory'])
            ratios=[statistics.mean(v['pr91'])/statistics.mean(v[treatment['candidate']]) for _,v in sorted(blocks.items())]
            comparisons.append({'candidate':treatment['candidate'],'paired_block_speedups':ratios,'median_speedup':statistics.median(ratios),'observed_range':[min(ratios),max(ratios)],'loops_per_batch':treatment['loops_per_batch']})
        rows.append({'dimension':data['dimension'],'threads':data['threads'],'steps':data['steps'],'blocks':data['blocks'],'diagnostics':data['diagnostics'],'comparisons':comparisons,'raw':path.name,'raw_sha256':hashlib.sha256(path.read_bytes()).hexdigest()})
    result={'status':'pass_scoped','scope':'full 64-step trajectories; five descriptive paired blocks, observed range not confidence interval','rows':rows}
    (ROOT/(PREFIX+'BENCHMARK_SUMMARY.json')).write_text(json.dumps(result,indent=2)+'\n')
    return [{'n':r['dimension'],'threads':r['threads'],'results':r['comparisons'],'max_grid_error':max(d['maximum_common_grid_error'] for d in r['diagnostics'])} for r in rows]

if __name__=='__main__':
    started=datetime.datetime.now(datetime.timezone.utc).isoformat()
    result={'started_utc':started,'exact_oracle':exact_oracle(),'timings':timings(),'finished_utc':datetime.datetime.now(datetime.timezone.utc).isoformat()}
    (ROOT/(PREFIX+'ANALYSIS_RECEIPT.json')).write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))
