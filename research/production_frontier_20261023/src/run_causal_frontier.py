#!/usr/bin/env python3
"""Bounded accepted-trajectory campaign; run only during a coordinated quiet window."""
import argparse, csv, hashlib, json, os, pathlib, statistics, subprocess, time

ROOT = pathlib.Path(__file__).resolve().parents[3]
CASES = ['few-mode', 'few-mode-large', 'riccati', 'stiff', 'nonnormal', 'oscillatory', 'broad']
METHODS = ['residual-first', 'causal-forward', 'causal-picard']

def main():
    p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--output',required=True)
    p.add_argument('--repeats',type=int,default=3);p.add_argument('--threads',type=int,choices=[1,2],default=1)
    p.add_argument('--plan-only',action='store_true');p.add_argument('--campaign-seconds',type=float,default=600);a=p.parse_args()
    if a.repeats not in range(1,6) or not (a.campaign_seconds>0):p.error('invalid repeats or campaign budget')
    out=pathlib.Path(a.output).resolve();out.mkdir(parents=True,exist_ok=True)
    binary=pathlib.Path(a.binary).resolve()
    affinity=sorted(os.sched_getaffinity(0)) if hasattr(os,'sched_getaffinity') else []
    chosen=affinity[:a.threads]
    if chosen: os.sched_setaffinity(0,chosen)
    rev=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
    tracked=['Cargo.toml','Cargo.lock','crates/vigilode-coupled/examples/frontier_causal_trajectory.rs']
    for crate in ['vigilode-coupled','rodas5p-integrators','rodas5p-core','rodas5p-krylov']:
        folder=ROOT/'crates'/crate
        tracked.append(str((folder/'Cargo.toml').relative_to(ROOT)))
        tracked.extend(str(f.relative_to(ROOT)) for f in sorted((folder/'src').rglob('*.rs')))
        if (folder/'build.rs').exists():tracked.append(str((folder/'build.rs').relative_to(ROOT)))
    dirty=subprocess.check_output(['git','status','--porcelain','--']+tracked,cwd=ROOT,text=True)
    tree=subprocess.check_output(['git','rev-parse','HEAD^{tree}'],cwd=ROOT,text=True).strip()
    plan={'schema':'vigilode.causal-frontier-campaign.v1','source_commit':rev,'source_tree':tree,'source_status':dirty,'source_clean':not bool(dirty),'total_wall_budget_seconds':a.campaign_seconds,'source_file_sha256':{s:hashlib.sha256((ROOT/s).read_bytes()).hexdigest() for s in tracked},'binary':str(binary),'threads':a.threads,'cpu_affinity':chosen,'cases':CASES,'methods':METHODS,'tolerances':[1e-4,1e-6],'repeats':a.repeats,'absolute_error_cap':1e-5,'per_process_timeout_seconds':120,'timing_contract':'complete solve; fresh-process warmup; same observations; actual error eligibility; failed work retained','matrix_free_scope':'provided frozen-base JVP remains; no state Jacobian assembly by candidate; current-stage JVP zero only causal schedules'}
    if binary.exists():plan['binary_sha256']=hashlib.sha256(binary.read_bytes()).hexdigest()
    (out/'PLAN.json').write_text(json.dumps(plan,indent=2)+'\n')
    if a.plan_only:return
    if dirty:raise SystemExit('Refusing timed campaign: relevant native sources differ from HEAD; commit frozen sources first.')
    rows=[];receipts=[];campaign_start=time.monotonic();deadline=campaign_start+a.campaign_seconds
    env=os.environ.copy();env['VIGILODE_SOURCE_COMMIT']=rev
    env.update({k:'1' for k in ['OPENBLAS_NUM_THREADS','OMP_NUM_THREADS','MKL_NUM_THREADS','BLIS_NUM_THREADS']})
    for case in CASES:
        for tol in plan['tolerances']:
            # rotate deterministic method order across cases/tolerances.
            offset=(CASES.index(case)+plan['tolerances'].index(tol))%len(METHODS)
            for method in METHODS[offset:]+METHODS[:offset]:
                tag=f'{case}_{method}_{tol:g}_t{a.threads}';target=out/(tag+'.json')
                cmd=[str(binary),'--case',case,'--mode',method,'--rtol',str(tol),'--repeats',str(a.repeats),'--threads',str(a.threads)]
                start=time.monotonic()
                remaining=deadline-start
                if remaining<=0:
                    receipts.append({'tag':tag,'command':cmd,'status':'NOT_RUN_TOTAL_BUDGET_EXHAUSTED'})
                    rows.append({'case':case,'method':method,'rtol':tol,'success':False,'status':'NOT_RUN_TOTAL_BUDGET_EXHAUSTED'})
                    continue
                with target.open('w') as so, (out/(tag+'.stderr.log')).open('w') as se:
                    try:
                        r=subprocess.run(cmd,stdout=so,stderr=se,env=env,cwd=ROOT,timeout=min(120,remaining));code=r.returncode
                    except subprocess.TimeoutExpired:code=124
                receipt={'tag':tag,'command':cmd,'exit_code':code,'process_wall_seconds':time.monotonic()-start};receipts.append(receipt)
                if code:rows.append({'case':case,'method':method,'rtol':tol,'success':False,'exit_code':code});continue
                try:d=json.loads(target.read_text())
                except Exception as e:rows.append({'case':case,'method':method,'rtol':tol,'success':False,'parse_error':str(e)});continue
                for r in d['runs']:
                    c=r.get('counters',{});ct=r.get('candidate_totals',{})
                    rows.append({'case':case,'method':method,'rtol':tol,'repeat':r['repeat'],'threads':a.threads,'success':r['success'],'seconds':r['elapsed_seconds'],'max_abs_error':r.get('max_abs_error'),'max_reference_scaled_wrms':r.get('max_reference_scaled_wrms'),'rhs':c.get('rhs_calls'),'jvp':c.get('jvp_calls'),'accepted':c.get('accepted_steps'),'rejected':c.get('rejected_steps'),'stage_jvp':ct.get('stage_jvp'),'fallback_attempts':ct.get('fallback_attempts'),'candidate_factorizations':ct.get('candidate_factorizations'),'newton_updates':ct.get('newton_updates'),'picard_updates':ct.get('picard_updates'),'basis_expansions':ct.get('basis_expansions')})
    (out/'RECEIPTS.json').write_text(json.dumps(receipts,indent=2)+'\n')
    (out/'ROWS.json').write_text(json.dumps(rows,indent=2)+'\n')
    fields=sorted(set().union(*(r.keys() for r in rows)))
    with (out/'ROWS.csv').open('w',newline='') as f:
        writer=csv.DictWriter(f,fieldnames=fields);writer.writeheader();writer.writerows(rows)
    comparisons=[]
    for case in CASES:
        eligible={}
        for method in METHODS:
            options=[]
            for tol in plan['tolerances']:
                rr=[r for r in rows if r['case']==case and r['method']==method and r['rtol']==tol]
                if len(rr)==a.repeats and all(r['success'] and r.get('max_abs_error') is not None and r['max_abs_error']<=plan['absolute_error_cap'] for r in rr):
                    options.append({'method':method,'rtol':tol,'median_seconds':statistics.median(r['seconds'] for r in rr),'worst_max_abs_error':max(r['max_abs_error'] for r in rr),'rhs':rr[0]['rhs'],'jvp':rr[0]['jvp'],'accepted':rr[0]['accepted'],'rejected':rr[0]['rejected'],'fallback_attempts':rr[0]['fallback_attempts']})
            if options:eligible[method]=min(options,key=lambda r:r['median_seconds'])
        baseline=eligible.get('residual-first')
        if baseline:
            for row in eligible.values():row['speed_ratio_baseline_over_method']=baseline['median_seconds']/row['median_seconds']
        comparisons.append({'case':case,'absolute_error_cap':plan['absolute_error_cap'],'eligible_best':eligible,'selection_scope':'best of two preregistered tolerances; no global optimality claim'})
    summary={'schema':'vigilode.causal-frontier-summary.v1','campaign_wall_seconds':time.monotonic()-campaign_start,'total_runs':len(rows),'successful_runs':sum(r['success'] for r in rows),'failed_runs':sum(not r['success'] for r in rows),'comparisons':comparisons,'timing_caveat':'single-host microcampaign; compare repeat spread and work, not universal speed','production_decision':'SPECIALIZED_ONLY; autonomous identity mass supplied JVP; broad production unresolved'}
    (out/'SUMMARY.json').write_text(json.dumps(summary,indent=2)+'\n')
    print(json.dumps(summary,indent=2))

if __name__=='__main__':main()
