#!/usr/bin/env python3
"""Measured work-precision plots and common achieved-error threshold comparison."""
import argparse,json,pathlib,statistics
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

def summarized_work(work):
    attempts=work.get('attempts',[])
    if isinstance(attempts,int):
        return {k:work.get(k) for k in ['attempts','accepted_steps','rejected_steps','state_reuses','counters']}
    result={'attempts':len(attempts),'accepted_steps':sum(a.get('accepted',False) for a in attempts),
            'fallback_attempts':sum(a.get('fallback_attempted',False) for a in attempts),'counters':work.get('counters'),
            'candidate_work':{},'fallback_work':{},'fallback_callback_work':{},'fallback_validation_work':{}}
    result['rejected_steps']=result['attempts']-result['accepted_steps']
    result['counter_semantics']='fallback_callback_work is diagnostic attempted/completed work already represented in fallback_work; do not add it again'
    for name in ['candidate_work','fallback_work','fallback_callback_work','fallback_validation_work']:
        for a in attempts:
            for key,value in a.get(name,{}).items():
                if isinstance(value,(int,float)):
                    # Peak fields are maxima; attempted/completed/call counters are sums.
                    if key.startswith(('peak_','maximum_','planned_storage')):
                        result[name][key]=max(result[name].get(key,0),value)
                    else:result[name][key]=result[name].get(key,0)+value
    return result

def main():
    ap=argparse.ArgumentParser();ap.add_argument('campaign');args=ap.parse_args()
    folder=pathlib.Path(args.campaign);rows=[];unparsed=[]
    for path in sorted(folder.glob('*.stdout.json')):
        try: record=json.loads(path.read_text())
        except Exception as error:unparsed.append({'file':path.name,'error':str(error)});continue
        runs=record.get('runs',[])
        if not runs or record.get('case') is None:
            unparsed.append({'file':path.name,'error':'No completed benchmark record','record':record})
            continue
        success=len(runs)==3 and all(r.get('success',False) for r in runs) and record.get('warmup',{}).get('success',False)
        row={k:record.get(k) for k in ['case','dimension','mode','threads','rtol','atol','refinement','max_step','source_commit']}
        row.update(file=path.name,all_repeats_success=success)
        if runs:
            row['median_seconds']=statistics.median(r['elapsed_seconds'] for r in runs)
            row['min_seconds']=min(r['elapsed_seconds'] for r in runs)
            row['max_seconds']=max(r['elapsed_seconds'] for r in runs)
            errors=[r['max_abs_error'] for r in runs if 'max_abs_error'in r]
            row['max_abs_error']=max(errors) if errors else None
            row['max_reference_scaled_wrms']=max((r.get('max_reference_scaled_wrms',0) for r in runs),default=None)
            row['work']=summarized_work(runs[-1].get('work',{}))
            row['failure_messages']=[r.get('error',r.get('message')) for r in runs if not r.get('success',False)]
        rows.append(row)
    matched=[]
    cases=list(dict.fromkeys(r['case'] for r in rows if not r.get('refinement')))
    for case in cases:
        for target in [1e-6,1e-8]:
            eligible=[r for r in rows if r['case']==case and not r['refinement'] and r['all_repeats_success'] and r['max_abs_error']<=target]
            base=[r for r in eligible if r['mode']=='sequential']
            baseline=min(base,key=lambda x:x['median_seconds']) if base else None
            for workers in [1,2]:
                candidates=[r for r in eligible if r['mode']=='coupled' and r['threads']==workers]
                candidate=min(candidates,key=lambda x:x['median_seconds']) if candidates else None
                matched.append({'case':case,'max_abs_error_threshold':target,'threads':workers,
                    'status':'COMPARABLE' if baseline and candidate else 'INSUFFICIENT_ELIGIBLE_CONFIGURATIONS',
                    'baseline_file':baseline['file'] if baseline else None,'candidate_file':candidate['file'] if candidate else None,
                    'baseline_seconds':baseline['median_seconds'] if baseline else None,'candidate_seconds':candidate['median_seconds'] if candidate else None,
                    'baseline_error':baseline['max_abs_error'] if baseline else None,'candidate_error':candidate['max_abs_error'] if candidate else None,
                    'speedup':baseline['median_seconds']/candidate['median_seconds'] if baseline and candidate else None})
    refinements=[]
    import math
    for mode in ['sequential','coupled']:
        data=sorted([r for r in rows if r['refinement'] and r['mode']==mode and r['all_repeats_success']],key=lambda r:-r['max_step'])
        for a,b in zip(data,data[1:]):
            if a['max_abs_error']>0 and b['max_abs_error']>0:
                refinements.append({'mode':mode,'h_coarse':a['max_step'],'h_fine':b['max_step'],
                    'error_coarse':a['max_abs_error'],'error_fine':b['max_abs_error'],
                    'observed_slope':math.log(a['max_abs_error']/b['max_abs_error'])/math.log(a['max_step']/b['max_step'])})
    summary={'schema':'vigilode.local-adaptive-summary.v1','rows':rows,'unparsed_outputs':unparsed,'common_error_threshold_comparisons':matched,
             'refinement_slopes':refinements,'interpretation':'Common achieved-error threshold, not identical error or asymptotic speed guarantee. Timing includes complete solve; the change includes adaptive coupling and small-block Newton factorization.'}
    (folder/'SUMMARY.json').write_text(json.dumps(summary,indent=2)+'\n')
    fig,axes=plt.subplots(3,3,figsize=(12,10));axes=axes.ravel()
    colors={'sequential':'#333333','coupled1':'#0072b2','coupled2':'#d55e00'}
    for ax,case in zip(axes,cases):
        for mode,workers in [('sequential',1),('coupled',1),('coupled',2)]:
            data=sorted([r for r in rows if r['case']==case and r['mode']==mode and r['threads']==workers and not r['refinement'] and r['all_repeats_success']],key=lambda r:r['max_abs_error'])
            if data:
                key=mode if mode=='sequential' else mode+str(workers)
                ax.errorbar([r['max_abs_error'] for r in data],[r['median_seconds'] for r in data],
                    yerr=[[r['median_seconds']-r['min_seconds'] for r in data],[r['max_seconds']-r['median_seconds'] for r in data]],
                    marker='o',capsize=3,label='raw sequential' if mode=='sequential' else f'coupled, {workers} worker(s)',color=colors[key])
        ax.set(xscale='log',yscale='log',title=case,xlabel='Maximum absolute error',ylabel='Full solve seconds');ax.grid(True,which='both',alpha=.25)
    for ax in axes[len(cases):]:ax.axis('off')
    handles,labels=axes[0].get_legend_handles_labels();fig.legend(handles,labels,loc='lower center',ncol=3)
    fig.suptitle('Local raw-U adaptive trajectories: measured work versus accuracy\nMedian of 3 timed solves; bars span minimum to maximum')
    fig.tight_layout(rect=[0,.05,1,.94])
    for extension in ['png','pdf','svg']:fig.savefig(folder/f'WORK_PRECISION.{extension}',dpi=160)
    lines=['# Local adaptive trajectory results','','All configurations, including failures, are retained in the campaign receipts. Timings use one local sandbox and cannot be extrapolated to NCP.','','| Case | Path | Workers | rtol | All repeats passed | Max absolute error | Median seconds | Fallback attempts |','|---|---|---:|---:|---|---:|---:|---:|']
    for r in rows:
        if r['refinement']:continue
        lines.append(f"| {r['case']} | {r['mode']} | {r['threads']} | {r['rtol']:.0e} | {r['all_repeats_success']} | {r.get('max_abs_error')} | {r.get('median_seconds',float('nan')):.6g} | {r.get('work',{}).get('fallback_attempts','n/a')} |")
    lines += ['','The machine-readable comparison uses common maximum-absolute-error thresholds of 1e-6 and 1e-8. It selects the fastest measured configuration satisfying each threshold for both methods. This establishes a shared accuracy requirement, not exactly equal errors. Failed and ineligible configurations are excluded from timing ratios but remain in the table.','','![Measured work-precision](WORK_PRECISION.png)']
    (folder/'RESULTS.md').write_text('\n'.join(lines)+'\n')

if __name__=='__main__':main()
