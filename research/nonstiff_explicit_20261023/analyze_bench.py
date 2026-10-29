#!/usr/bin/env python3
"""Re-score complete saved states and compare cost at common achieved error."""
import argparse, gzip, hashlib, json, math, pathlib, statistics, tarfile
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

def exact(case,n,t):
    i=np.arange(n)
    initial=1./(1.+i%8)
    rates=np.array([.25,1.,3.,.5])[i%4]
    if case=='logistic':
        initial=.05+.01*(i%8);rates=.25+.1*(i%8)
        return initial,1./(1.+(1./initial-1.)*np.exp(-rates*t))
    if case=='mixed-scales':initial=10.**(i%12-8)
    if case=='large-vector':rates=.5+.05*(i%8)
    if case=='moderate-stiff':rates=np.array([1.,30.,300.])[i%3]
    if case=='stiff-control':rates=np.array([1.,1000.,100000.])[i%3]
    if case=='driven':return initial,np.sin(t)+initial*np.exp(-rates*t)
    if case=='nonnormal':
        initial=(i%2).astype(float);out=np.empty(n)
        out[0::2]=30.*(np.exp(-t)-np.exp(-2.*t));out[1::2]=np.exp(-2.*t)
        return initial,out
    if case in ('harmonic','rapid','damped'):
        initial=(1-i%2).astype(float);out=np.empty(n)
        j=np.arange(n//2)%4
        w=(100.+5.*j) if case=='rapid' else (10.+j) if case=='damped' else (1.+.25*j)
        envelope=np.exp(-3.*t) if case=='damped' else 1.
        out[0::2]=envelope*np.cos(w*t);out[1::2]=envelope*np.sin(w*t)
        return initial,out
    return initial,initial*np.exp(-rates*t)

def checked_record(record):
    errors=[]
    try:
        case=record['case'];n=record['dimension'];times=record['times']
        states=np.asarray(record['states'],dtype=float);stored=np.asarray(record['reference'],dtype=float)
        if states.shape!=(len(times),n) or stored.shape!=states.shape or not np.isfinite(states).all() or not np.isfinite(stored).all() or not np.isfinite(times).all():
            return {'valid':False,'reasons':['shape or nonfinite state'],'max_abs_error':None,'max_normalized_error':None}
        if not math.isfinite(record['elapsed_seconds']) or record['elapsed_seconds']<=0:errors.append('nonpositive or nonfinite elapsed time')
        if not times:return {'valid':False,'reasons':['no outputs'],'max_abs_error':None,'max_normalized_error':None}
        references=np.array([exact(case,n,t)[1] for t in times]);initial=exact(case,n,0.)[0]
        scales=np.maximum(1.,np.maximum(np.abs(initial)[None,:],np.abs(references)))
        ref_disagreement=float(np.max(np.abs(references-stored)/scales))
        if ref_disagreement>5e-14:errors.append('stored analytic reference disagrees with independent Python expression')
        error=np.abs(states-references);absolute=float(error.max());normalized=float((error/scales).max())
        if record.get('success') and times!=record.get('requested_times'):errors.append('successful output times differ from request')
        if record.get('success') and len(times)!=5:errors.append('successful record must include five observations')
        if record.get('max_normalized_error') is not None and abs(normalized-record['max_normalized_error'])>1e-13*max(1.,normalized):
            errors.append('reported error disagrees with replay')
        if record.get('mode')=='dopri5':
            work=record.get('work',{}).get('counters',{})
            forbidden=['jacobian_builds','jvp_calls','jvp_vectors','linear_solves','direct_factorizations','nonlinear_solves','nonlinear_iterations']
            if any(work.get(k,0)!=0 for k in forbidden):errors.append('explicit path charged a forbidden derivative or solve operation')
        return {'valid':not errors,'reasons':errors,'max_abs_error':absolute,'max_normalized_error':normalized,
                'reference_disagreement':ref_disagreement}
    except Exception as error:return {'valid':False,'reasons':[repr(error)],'max_abs_error':None,'max_normalized_error':None}

def main():
    ap=argparse.ArgumentParser();ap.add_argument('campaign');args=ap.parse_args();folder=pathlib.Path(args.campaign)
    plan=json.loads((folder/'PLAN.json').read_text());receipts=json.loads((folder/'RECEIPTS.json').read_text())
    outputs=[];unparsed=[]
    for receipt in receipts:
        p=folder/receipt['stdout']
        try:record=json.loads(p.read_text())
        except Exception as error:
            unparsed.append({'receipt':receipt,'reason':repr(error)});continue
        replay=checked_record(record)
        expected=next(c for c in plan['configurations'] if c['id']==receipt['id'])
        if record.get('source_commit')!=plan['source_commit'] or any(record.get(k)!=expected[k] for k in ('case','dimension','rtol','atol','mode')):
            replay['valid']=False;replay['reasons'].append('source/configuration identity mismatch')
        if receipt.get('stdout_sha256')!=hashlib.sha256(p.read_bytes()).hexdigest():
            replay['valid']=False;replay['reasons'].append('raw stdout hash mismatch')
        outputs.append({'receipt':receipt,'record':record,'replay':replay})
    rows=[]
    for cfg in plan['configurations']:
        runs=[r for r in outputs if r['receipt']['id']==cfg['id']]
        timed=[r for r in runs if r['receipt']['phase']=='timed'];warmups=[r for r in runs if r['receipt']['phase']=='warmup']
        success=len(timed)==plan['timed_repeats'] and len(warmups)==1 and all(r['receipt']['status']=='COMPLETED' and r['record'].get('success',False) and r['replay']['valid'] for r in runs)
        timing=[r['record']['elapsed_seconds'] for r in timed]
        numeric=[r['replay'] for r in timed if r['replay']['max_normalized_error'] is not None]
        row={**cfg,'all_runs_successful':success,'timed_runs':len(timed),'warmups':len(warmups),
             'seconds':timing,'median_seconds':statistics.median(timing) if timing else None,
             'min_seconds':min(timing) if timing else None,'max_seconds':max(timing) if timing else None,
             'max_abs_error':max((x['max_abs_error'] for x in numeric),default=None),
             'max_normalized_error':max((x['max_normalized_error'] for x in numeric),default=None),
             'work':timed[-1]['record'].get('work') if timed else None,
             'adapter':timed[-1]['record'].get('adapter') if timed else None,
             'failure_messages':[r['record'].get('message') for r in runs if not r['record'].get('success',False)],
             'replay_failures':[r['replay']['reasons'] for r in runs if not r['replay']['valid']]}
        rows.append(row)
    comparisons=[]
    for case,n in dict.fromkeys((r['case'],r['dimension']) for r in rows):
        for threshold in (1e-6,1e-8):
            eligible=[r for r in rows if r['case']==case and r['dimension']==n and r['all_runs_successful'] and r['max_normalized_error']<=threshold]
            explicit=min((r for r in eligible if r['mode']=='dopri5'),key=lambda x:x['median_seconds'],default=None)
            for mode in dict.fromkeys(r['mode'] for r in rows if r['mode']!='dopri5'):
                implicit=min((r for r in eligible if r['mode']==mode),key=lambda x:x['median_seconds'],default=None)
                if not any(r['case']==case and r['dimension']==n and r['mode']==mode for r in rows):continue
                comparisons.append({'case':case,'dimension':n,'normalized_error_threshold':threshold,'comparator':mode,
                    'status':'COMPARABLE' if explicit and implicit else 'INSUFFICIENT_ELIGIBLE_CONFIGURATIONS',
                    'explicit_config':explicit['id'] if explicit else None,'comparator_config':implicit['id'] if implicit else None,
                    'explicit_error':explicit['max_normalized_error'] if explicit else None,
                    'comparator_error':implicit['max_normalized_error'] if implicit else None,
                    'explicit_seconds':explicit['median_seconds'] if explicit else None,
                    'comparator_seconds':implicit['median_seconds'] if implicit else None,
                    'comparator_over_explicit_time':implicit['median_seconds']/explicit['median_seconds'] if explicit and implicit else None,
                    'observed_ratio_envelope':[implicit['min_seconds']/explicit['max_seconds'],implicit['max_seconds']/explicit['min_seconds']] if explicit and implicit else None})
    replay_summary={'parsed_records':len(outputs),'successful_records':sum(r['record'].get('success',False) for r in outputs),
                    'valid_replays':sum(r['replay']['valid'] for r in outputs),'invalid_replays':sum(not r['replay']['valid'] for r in outputs),
                    'max_reference_disagreement':max((r['replay'].get('reference_disagreement',0.) for r in outputs),default=None)}
    summary={'schema':'vigilode.nonstiff-summary.v1','source_commit':plan['source_commit'],'rows':rows,
             'common_error_comparisons':comparisons,'replay':replay_summary,'unparsed':unparsed,
             'interpretation':'Finite sampled-state error and measured whole solve costs, not uniform-in-time certified accuracy. Error thresholds fixed independently of requested tolerances. Ratios use only successful replay-verified runs; min/max ratio envelopes are not confidence intervals.'}
    (folder/'SUMMARY.json').write_text(json.dumps(summary,indent=2)+'\n')
    (folder/'REPLAY.json').write_text(json.dumps([{'id':r['receipt']['id'],'phase':r['receipt']['phase'],'repeat':r['receipt']['repeat'],'stdout':r['receipt']['stdout'],**r['replay']} for r in outputs],indent=2)+'\n')
    labels=list(dict.fromkeys((r['case'],r['dimension']) for r in rows));columns=3;nr=math.ceil(len(labels)/columns)
    fig,axes=plt.subplots(nr,columns,figsize=(13,3.5*nr),squeeze=False);axes=axes.ravel()
    colors={'dopri5':'#0072b2','rodas-dense':'#444444','rodas-mf':'#009e73','vode':'#d55e00','diffsol':'#cc79a7','julia':'#e69f00'}
    for ax,(case,n) in zip(axes,labels):
        for mode in dict.fromkeys(r['mode'] for r in rows):
            data=sorted([r for r in rows if r['case']==case and r['dimension']==n and r['mode']==mode and r['all_runs_successful']],key=lambda x:x['max_normalized_error'])
            if not data:continue
            x=[max(r['max_normalized_error'],1e-17) for r in data];y=[r['median_seconds'] for r in data]
            ax.errorbar(x,y,yerr=[[r['median_seconds']-r['min_seconds'] for r in data],[r['max_seconds']-r['median_seconds'] for r in data]],fmt='o-',capsize=3,label=mode,color=colors.get(mode))
        ax.set_xscale('log');ax.set_yscale('log');ax.set_title(f'{case}, n={n}');ax.grid(True,which='both',alpha=.2)
        ax.set_xlabel('Max normalized sampled error');ax.set_ylabel('Complete solve seconds')
    for ax in axes[len(labels):]:ax.axis('off')
    handles={}
    for ax in axes:
        for h,label in zip(*ax.get_legend_handles_labels()):handles[label]=h
    fig.legend(handles.values(),handles.keys(),loc='lower center',ncol=3)
    fig.suptitle('Local explicit / protected implicit work–precision\nMedian of timed repetitions; bars span observed minimum–maximum',y=.995)
    fig.tight_layout(rect=(0,.045,1,.95))
    for ext in ('png','pdf','svg'):fig.savefig(folder/f'WORK_PRECISION.{ext}',dpi=150)
    lines=['# Local nonstiff campaign results','','Each error is re-scored from every saved state against an independent analytic expression. All output rows and failures remain in the raw archive. Native methods land on five observation times; VODE may interpolate its internal steps to the same requested observations.','',
           '| Case | n | Method | rtol | All runs pass | Max normalized error | Median seconds | RHS attempts |',
           '|---|---:|---|---:|---|---:|---:|---:|']
    for r in rows:
        work=r['work'] or {};rhs=work.get('counters',{}).get('rhs_calls',work.get('rhs_attempted'))
        lines.append(f"| {r['case']} | {r['dimension']} | {r['mode']} | {r['rtol']:.0e} | {r['all_runs_successful']} | {r['max_normalized_error']} | {r['median_seconds']} | {rhs} |")
    lines+=['','![Measured work–precision](WORK_PRECISION.png)','','Common-error comparisons are in SUMMARY.json. A ratio larger than one means the comparator took longer than DOPRI5 on these samples. The comparison includes setup and callbacks; it does not isolate mathematical-stage cost or compiler/language overhead. VODE callbacks run in Python/NumPy and its imports are excluded. Three repetitions with randomized order provide a local observation, not a universal speed guarantee or confidence interval. Stiff controls identify the explicit stability penalty and are retained regardless of performance.']
    (folder/'RESULTS.md').write_text('\n'.join(lines)+'\n')
    paths=sorted([*folder.glob('*.stdout.json'),*folder.glob('*.stderr.log')]);entries=[]
    archive=folder/'RAW_OUTPUTS.tar.gz'
    with archive.open('wb') as raw:
        with gzip.GzipFile(filename='',mode='wb',fileobj=raw,mtime=0) as gz:
            with tarfile.open(mode='w',fileobj=gz,format=tarfile.USTAR_FORMAT) as tar:
                for p in paths:
                    b=p.read_bytes();entries.append({'path':p.name,'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()})
                    info=tarfile.TarInfo(p.name);info.size=len(b);info.mode=0o644;info.mtime=0
                    with p.open('rb') as content:tar.addfile(info,content)
    with tarfile.open(archive,'r:gz') as tar:
        for e in entries:assert hashlib.sha256(tar.extractfile(e['path']).read()).hexdigest()==e['sha256']
    (folder/'RAW_MANIFEST.json').write_text(json.dumps({'archive':archive.name,'sha256':hashlib.sha256(archive.read_bytes()).hexdigest(),'files':entries,'verified':True},indent=2)+'\n')
    (folder/'.gitignore').write_text('# Exact raw bytes are preserved in RAW_OUTPUTS.tar.gz.\n*.stdout.json\n*.stderr.log\n')
    print(json.dumps({'configurations':len(rows),'all_runs_successful':sum(r['all_runs_successful'] for r in rows),'replay':replay_summary,'archive_bytes':archive.stat().st_size}))

if __name__=='__main__':main()
