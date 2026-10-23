#!/usr/bin/env python3
"""Independent one-pass archive audit; never executes a solver or imports its analyzer."""
from pathlib import Path
import hashlib, json, math, statistics, tarfile, subprocess
import numpy as np
ROOT=Path(__file__).resolve().parents[3]
NODE=ROOT/'research/production_hardening_20261023'
B=NODE/'evidence/local_bench'

def sha(data): return hashlib.sha256(data).hexdigest()
def exact(case,i,t):
    y0=.05+.01*(i%8) if case=='logistic' else float(i%2) if case=='nonnormal' else 1.-i%2 if case in ('rapid','damped') else 1./(1+i%8)
    if case=='logistic': return 1./(1.+(1./y0-1)*math.exp(-(.25+.1*(i%8))*t))
    if case=='nonnormal': return math.exp(-2*t) if i%2 else 30*(math.exp(-t)-math.exp(-2*t))
    if case in ('rapid','damped'):
        w=(100+5*((i//2)%4)) if case=='rapid' else 10+(i//2)%4
        return (math.exp(-3*t) if case=='damped' else 1)*(math.sin(w*t) if i%2 else math.cos(w*t))
    rate=.5+.05*(i%8) if case=='large-vector' else (.25,1.,3.,.5)[i%4]
    return y0*math.exp(-rate*t)+(math.sin(t) if case=='driven' else 0.)

def main():
    if not (B/'SUMMARY.json').is_file(): raise RuntimeError('Wait for the original analyzer to finish; no concurrent duplicate replay.')
    plan=json.loads((B/'PLAN.json').read_text()); summary=json.loads((B/'SUMMARY.json').read_text()); campaign=json.loads((B/'CAMPAIGN.json').read_text()); receipts=json.loads((B/'RECEIPTS.json').read_text()); manifest=json.loads((B/'RAW_MANIFEST.json').read_text())
    problems=[]
    def ck(ok,msg):
        if not ok: problems.append(msg)
    configs={c['id']:c for c in plan['configurations']}
    ck(len(configs)==len(plan['configurations'])==72,'configuration count/identity')
    expected_keys={(i,phase,rep) for i in configs for phase,rep in [('warmup',-1),('timed',0),('timed',1),('timed',2)]}
    keys=[(r['id'],r['phase'],r['repeat']) for r in receipts]
    ck(len(keys)==len(set(keys))==288 and set(keys)==expected_keys,'exact unique configuration/round coverage')
    by_stdout={r['stdout']:r for r in receipts}; man={e['path']:e for e in manifest}
    ck(len(man)==len(manifest)==576,'raw manifest coverage')
    archive_bytes=(B/'RAW_OUTPUTS.tar.gz').read_bytes(); ck(sha(archive_bytes)==campaign['raw_archive_sha256'],'whole archive hash')
    del archive_bytes
    records=[]; raw_checked=[]; membernames=[]
    with tarfile.open(B/'RAW_OUTPUTS.tar.gz','r|gz') as tar:
        for member in tar:
            if not member.isfile(): continue
            data=tar.extractfile(member).read(); membernames.append(member.name); e=man.get(member.name)
            ok=e is not None and len(data)==e['bytes'] and sha(data)==e['sha256']; ck(ok,'raw file '+member.name); raw_checked.append(ok)
            if member.name not in by_stdout: continue
            receipt=by_stdout[member.name]; cfg=configs[receipt['id']]; r=json.loads(data); local=[]
            def rc(ok,msg):
                if not ok: local.append(msg)
            rc(receipt['status']=='COMPLETED' and receipt['exit_code']==0,'process status')
            rc(sha(data)==receipt['stdout_sha256'],'stdout receipt hash')
            rc(receipt['stderr_sha256']==man[receipt['stderr']]['sha256'],'stderr receipt hash')
            rc(r['source_commit']==plan['source_commit']==campaign['source_commit']==summary['source_commit'],'source identity')
            rc(all(r[k]==cfg[k] for k in ('case','dimension','mode','observations','rtol','atol')),'configuration fields')
            rc(r['success'] is True,'solver success')
            n=cfg['dimension']; m=cfg['observations']; case=cfg['case']; end=10. if case=='logistic' else 2. if case=='rapid' else 4.
            expected_times=[end*j/(m-1) for j in range(m)]
            rc(r['times']==r['requested_times']==expected_times and r['end']==end,'saved/requested time grid')
            states=np.asarray(r['states'],dtype='<f8'); rc(states.shape==(m,n) and np.isfinite(states).all(),'finite state shape')
            ref=np.asarray([[exact(case,i,t) for i in range(min(n,8))] for t in expected_times]); ref=np.tile(ref,(1,(n+7)//8))[:,:n] if n>8 else ref
            ini=ref[0]; error=np.abs(states-ref); abs_err=float(error.max()); norm_err=float((error/np.maximum(1.,np.maximum(np.abs(ini),np.abs(ref)))).max())
            rc(np.array_equal(states[0],ini),'exact initial state')
            for k,value in [('max_abs_error',abs_err),('max_normalized_error',norm_err)]:rc(abs(r[k]-value)<=max(5e-14,1e-8*value),'reported '+k)
            rc(r['final_time']==end and r['final_state']==r['states'][-1],'final state/time')
            work=r['work']; count=work['counters']; a=work['attempts']
            rc(a==work['accepted_steps']+work['rejected_steps'],'attempt accounting')
            rc(count['rhs_calls']==count['rhs_evaluations']==work['rhs_completed']==1+6*a and work['fsal_reuses']==a-1,'FSAL accounting')
            for key,value in count.items():
                if any(t in key for t in ('jacobian','jvp','newton','factorization','linear_','nonlinear_','matvec','preconditioner')):rc(value==0,'unexpected implicit work:'+key)
            rc(math.isfinite(r['elapsed_seconds']) and r['elapsed_seconds']>0,'timer')
            row={'id':cfg['id'],'phase':receipt['phase'],'repeat':receipt['repeat'],'passed':not local,'problems':local,'abs_error':abs_err,'error':norm_err,'seconds':r['elapsed_seconds'],'work':work,'state_hash':sha(states.tobytes()),'endpoint_hash':sha(states[-1].tobytes())}
            records.append(row)
            if local:problems.append({'record':member.name,'problems':local})
    ck(len(membernames)==len(set(membernames))==576 and set(membernames)==set(man),'archive exact member coverage')
    ck(len(records)==288 and all(r['passed'] for r in records),'all raw replays')
    groups={i:[r for r in records if r['id']==i] for i in configs}; rows={}
    for i,rs in groups.items():
        timed=[r for r in rs if r['phase']=='timed']; med=statistics.median(r['seconds'] for r in timed)
        row={'id':i,**configs[i],'seconds':med,'error':max(r['error'] for r in timed),'work':timed[0]['work'],'endpoint_hash':timed[0]['endpoint_hash'],'states_identical':len({r['state_hash'] for r in rs})==1,'work_identical':all(r['work']==timed[0]['work'] for r in rs)}; rows[i]=row
        published=next(x for x in summary['configuration_summaries'] if x['id']==i)
        ck(published['eligible'] and math.isclose(published['median_seconds'],med,rel_tol=1e-14) and math.isclose(published['max_normalized_error'],row['error'],abs_tol=5e-14),f'summary config {i}')
    density=[]
    for item in summary['density_invariance']:
        d=rows[item['dense_config']]; s=rows[item['sparse_config']]
        ok=d['work']==s['work'] and d['endpoint_hash']==s['endpoint_hash'] and all(x['states_identical'] and x['work_identical'] for x in [d,s]); ck(ok and item['passed'],'density pair '+str(item))
        density.append({'dense_config':d['id'],'sparse_config':s['id'],'passed':ok})
    caps=[]
    for item in summary['common_error_cap_pairs']:
        group=[r for r in rows.values() if r['case']==item['case'] and r['dimension']==item['dimension'] and r['observations']>5 and r['error']<=item['error_cap']]
        best={}
        for mode in ['landing','dense']:
            candidates=[r for r in group if r['mode']==mode]; best[mode]=min(candidates,key=lambda r:r['seconds']) if candidates else None
        ok=item['selected_config_ids']=={k:v['id'] if v else None for k,v in best.items()}
        if all(best.values()):ok=ok and item['status']=='COMPARABLE' and math.isclose(item['landing_over_dense_median'],best['landing']['seconds']/best['dense']['seconds'],rel_tol=1e-14)
        else:ok=ok and item['status']=='INSUFFICIENT_SAMPLED_TOLERANCES'
        ck(ok,'cap selection '+str(item));caps.append({'case':item['case'],'n':item['dimension'],'cap':item['error_cap'],'passed':ok,**item})
    result={'schema':'vigilode.final-benchmark-independent-evidence.v1','source_commit':plan['source_commit'],'source_tree':plan['source_tree'],'method':'Read-only one-pass raw archive hash and numerical replay using separate NumPy/math exact formulas; no solver/analyzer import or solver rerun.','raw_files_checked':len(raw_checked),'raw_hashes_passed':sum(raw_checked),'records_replayed':len(records),'record_passes':sum(r['passed'] for r in records),'unique_round_coverage':len(keys)==len(set(keys))==288 and set(keys)==expected_keys,'all_repeated_states_identical':all(r['states_identical'] for r in rows.values()),'all_repeated_work_identical':all(r['work_identical'] for r in rows.values()),'density_pairs':density,'cap_selections':caps,'problems':problems,'decision':'PASS_SCOPED' if not problems else 'FAIL'}
    (NODE/'evidence/FINAL_RAW_REPLAY_AUDIT.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps({k:v for k,v in result.items() if k not in ['density_pairs','cap_selections']},indent=2))
if __name__=='__main__':main()
