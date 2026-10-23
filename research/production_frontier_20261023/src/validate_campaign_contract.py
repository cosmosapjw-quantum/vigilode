#!/usr/bin/env python3
"""Validate the local qualification plan and exact-rational error-budget examples.
This does not execute or certify an ODE benchmark, nor validate physical constants.
"""
import argparse, copy, hashlib, itertools, json, math
from fractions import Fraction as F
from pathlib import Path

ROOT=Path(__file__).resolve().parents[3]
ALLOWED={'VODE/DVODE','diffsol','Julia'}
REQUIRED_FAMILIES={'nonstiff','stiff','stiff_order_reduction','affine_forcing','long_time_phase','nonlinear_rotation','noncommuting','nonnormal','switching','failure_contract','events','dense_output','restart','pde_fixed_grid','pde_nonlinear','amr','mass_capability','clock_extremes'}

def validate(plan):
    errors=[]
    def require(condition,message):
        if not condition:errors.append(message)
    require(plan.get('schema')=='vigilode.frontier.production-validation.v1','unexpected schema')
    require(plan.get('status')=='PLANNED_NOT_RUN','a plan must not assert executed evidence')
    require(plan.get('ncp') is False,'NCP must remain disabled')
    r=plan.get('resources',{})
    require(r.get('cloud_resources_allowed') is False,'cloud resource creation must remain disabled')
    require(r.get('threads')==[1,2] and r.get('single_thread_required') is True,'require small-machine 1/2-thread cohorts')
    require(r.get('concurrent_timed_solves')==1,'timed solves must be serialized')
    require(0<r.get('campaign_wall_seconds',0)<=7200,'finite local wall budget <=2h required')
    require(0<r.get('per_process_wall_seconds',0)<=r.get('campaign_wall_seconds',0),'per-process wall budget invalid')
    require(0<r.get('solver_peak_rss_bytes',0)<=2**31,'small-machine RSS bound required')
    require(sum(x.get('wall_seconds',0) for x in plan.get('execution_phases',[]))<=r.get('campaign_wall_seconds',0),'phase budgets exceed total')
    require(set(plan.get('comparators',[]))==ALLOWED,'external comparator families changed')
    require(plan.get('comparator_contract',{}).get('missing_dependency_status')=='NOT_RUN','missing comparator may not be a fabricated pass')
    families=plan.get('families',[])
    require(len({x['id'] for x in families})==len(families),'duplicate case IDs')
    require({x.get('family') for x in families}==REQUIRED_FAMILIES,'required production family missing/unregistered')
    require(all(x.get('execute_cartesian_product') is False and x.get('status')=='PLANNED_NOT_RUN' for x in families),'families must remain finite-budget planned subsets')
    fractions=plan.get('global_error_budget_fractions',{})
    require(all(math.isfinite(x) and x>=0 for x in fractions.values()),'nonnegative finite budget fractions required')
    require(abs(sum(fractions.values())-1)<=1e-15,'error budget fractions must sum to one')
    mappings=plan.get('old_blocker_mapping',[])
    keys=[(x['source'],x['old_id']) for x in mappings]
    require(len(set(keys))==len(keys),'duplicate legacy blocker mapping')
    for source,expected_hash in plan.get('inherited_source_sha256',{}).items():
        p=ROOT/source
        require(p.exists(),f'missing inherited source {source}')
        if not p.exists():continue
        require(hashlib.sha256(p.read_bytes()).hexdigest()==expected_hash,f'inherited source hash drift {source}')
        d=json.loads(p.read_text());items=d.get('items',d.get('tasks',[]));actual={x['id'] for x in items}
        mapped={x['old_id'] for x in mappings if x['source']==source}
        require(actual==mapped,f'incomplete legacy mapping {source}: {sorted(actual ^ mapped)}')
        inherited={x['id']:x['status'] for x in items}
        require(all(x['inherited_status']==inherited[x['old_id']] for x in mappings if x['source']==source),'inherited status changed without source evidence')
    acceptance=plan.get('acceptance',{})
    require(acceptance.get('contract_violation_count_max')==0,'contract failure cannot be averaged away')
    require(acceptance.get('reference_uncertainty_fraction_of_error_cap_max',1)<=.1,'reference uncertainty budget too large')
    require(acceptance.get('minimum_refinement_levels',0)>=3,'three-level convergence separation required')
    require(plan.get('performance_decisions',{}).get('accuracy_eligible_only') is True,'speed eligibility must require accuracy')
    require(plan.get('record_contract',{}).get('preserve_all_attempts') is True,'failed/rejected work must be retained')
    required={'binary_sha256','source_commit','source_tree','observed_states','reference_uncertainty','accepted_time','failure_reason'}
    require(required<=set(plan.get('record_contract',{}).get('required_fields',[])),'run evidence fields missing')
    return errors

def exact_checks():
    # Densities: old cell volumes(1,2), new volumes(1/2,1/2,2).
    w=[F(1),F(2)];v=[F(1,2),F(1,2),F(2)]
    R=[[F(1),F(0)],[F(1),F(0)],[F(0),F(1)]]
    C=[[F(1,2),F(1,2),F(0)],[F(0),F(0),F(1)]]
    apply=lambda A,x:[sum(a*b for a,b in zip(row,x)) for row in A]
    norm=lambda weights,x:sum(weight*abs(value) for weight,value in zip(weights,x))
    assert [sum(v[i]*R[i][j] for i in range(3)) for j in range(2)]==w
    assert [sum(w[i]*C[i][j] for i in range(2)) for j in range(3)]==v
    contraction_cases=0
    for x in itertools.product([F(-2),F(-1),F(0),F(1),F(2)],repeat=2):
        assert norm(v,apply(R,x))<=norm(w,x);contraction_cases+=1
    for x in itertools.product([F(-1),F(0),F(1)],repeat=3):
        assert norm(w,apply(C,x))<=norm(v,x);contraction_cases+=1
    a=F(11,10);eta=F(1,1000);jump=F(1,2000)
    trajectories=[]
    for signs in itertools.product([-1,1],repeat=5):
        e=[F(1,100),F(-1,200)];weights=w[:];bound=norm(weights,e)
        for step in range(4):
            e=[a*x for x in e];e[0]+=signs[step]*eta/weights[0];bound=a*bound+eta
            if step==1:
                e=apply(R,e);weights=v[:];e[0]+=signs[4]*jump/weights[0];bound+=jump
            assert norm(weights,e)<=bound
        trajectories.append({'error':str(norm(weights,e)),'bound':str(bound)})
    # A mass-conserving consistency jump can have zero mass error but nonzero state error.
    consistency=[F(1,100),F(-1,100),F(0)]
    assert sum(x*y for x,y in zip(v,consistency))==0
    consistency_norm=norm(v,consistency);assert consistency_norm>0
    return {'arithmetic':'exact fractions','weighted_positive_remap_cases':contraction_cases,'error_recurrence_sign_trajectories':len(trajectories),'maximum_observed_error':str(max(F(x['error']) for x in trajectories)),'common_bound':trajectories[0]['bound'],'zero_mass_nonzero_consistency_defect':str(consistency_norm),'status':'PASS','scope':'finite algebraic regression examples; theorem assumptions are proved separately, physical PDE constants are not certified'}

def mutation_checks(plan):
    tests=[]
    changes=[('ncp',lambda d:d.update(ncp=True)),('missing_class',lambda d:d['families'].pop()),('budget',lambda d:d['global_error_budget_fractions'].update(remap=.5)),('comparator',lambda d:d['comparators'].append('unapproved')),('legacy_mapping',lambda d:d['old_blocker_mapping'].pop()),('lost_failure',lambda d:d['record_contract'].update(preserve_all_attempts=False))]
    for name,change in changes:
        d=copy.deepcopy(plan);change(d);assert validate(d),name;tests.append(name)
    return tests

def main():
    p=argparse.ArgumentParser();p.add_argument('--plan',default=str(ROOT/'research/production_frontier_20261023/BENCHMARK_PLAN.json'));p.add_argument('--receipt');a=p.parse_args()
    source=Path(a.plan);plan=json.loads(source.read_text());errors=validate(plan)
    result={'schema':'vigilode.frontier.campaign-contract-check.v1','plan_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'errors':errors,'status':'PASS' if not errors else 'FAIL','benchmark_execution':'NOT_RUN'}
    if not errors:result.update(exact_error_budget_checks=exact_checks(),negative_mutations_rejected=mutation_checks(plan))
    text=json.dumps(result,indent=2)+'\n'
    if a.receipt:Path(a.receipt).write_text(text)
    print(text,end='')
    if errors:raise SystemExit(1)

if __name__=='__main__':main()
