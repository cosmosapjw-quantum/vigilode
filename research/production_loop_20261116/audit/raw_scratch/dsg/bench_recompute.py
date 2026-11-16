import json, math, re, collections
B='/home/cosmosapjw/vigilode/bench/out/'
d=json.load(open(B+'rust_bench_results.json')); jl=json.load(open(B+'julia_bench_results.json'))
rows=d['rows']; jr=jl['results']
out={}
out['matched_status_counts']=dict(collections.Counter(m['status'] for m in d['matched']))
out['matched_basis_counts']=dict(collections.Counter(m.get('basis') for m in d['matched']))
out['matched_with_time_ratio']=sum(1 for m in d['matched'] if m.get('time_ratio') is not None)
out['corpus_problem_n']={p['id']:p['n'] for p in d['corpus']['problems']}
out['grid_points']=d['corpus']['grid_points']
out['rust_arms_present']=sorted({r['arm'] for r in rows})
out['free_stepping_vigilode_rows']=sum(1 for r in rows if r['arm'].startswith('vigilode') and 'dense' in r['arm'])
out['julia_rows']=len(jr); out['julia_rows_with_error']=sum(1 for r in jr if r.get('error_common') is not None)
out['julia_problems']=sorted({r['problem'] for r in jr}); out['rust_problems']=sorted({r['problem'] for r in rows})
out['build_target_features']=d['provenance']['build_target_features']; out['build_git_dirty_context']=d['provenance']['build_git_dirty_context']
# clipped fraction per vigilode row
clip=[]
for r in rows:
    if r['arm'] in ('vigilode','vigilode-banded'):
        n=r['native']; acc=n['counters']['accepted_steps']
        clip.append({'problem':r['problem'],'arm':r['arm'],'rtol':r['rtol'],'accepted':acc,'rejected':n['counters']['rejected_steps'],
                     'output_clipped_steps':n['output_clipped_steps'],'clipped_fraction':n['output_clipped_steps']/acc,
                     'free_steps_lower_bound':acc-n['output_clipped_steps'],'error_common':r['error_common'],'wall_ms':r['wall_ms_median']})
out['vigilode_clipping']=clip
# Julia Rodas5P step counts for comparison
J={(r['problem'],r['rtol'],r['algorithm']):r for r in jr}
cmp=[]
for c in clip:
    j=J.get((c['problem'],c['rtol'],'julia-rodas5p'))
    if j and c['arm']=='vigilode':
        cmp.append({'problem':c['problem'],'rtol':c['rtol'],'vigilode_accepted':c['accepted'],'vigilode_clipped':c['output_clipped_steps'],
                    'julia_rodas5p_accepted':j['naccept'],'ratio':c['accepted']/j['naccept']})
out['vigilode_vs_julia_rodas5p_steps']=cmp
# reproduce three report rows
rep=open(B+'UNIFIED_BENCHMARK_REPORT.md').read()
def fmt_row(r):
    a=r['allocations']; nb=a['heap_bytes']
    fb = f"{nb/(1024*1024):.1f} MB" if nb>=1024*1024 else (f"{nb/1024:.1f} KB" if nb>=1024 else f"{nb} B")
    return {'error':f"{r['error_common']:.3e}",'wall':f"{r['wall_ms_median']:.3f}",'rhs':r['callbacks']['rhs'],'jac':r['callbacks']['jac'],
            'alloc':f"{a['heap_calls']} calls ({fb})"}
R={(r['problem'],r['rtol'],r['arm']):r for r in rows}
checks=[]
for key,label in ((('robertson',1e-4,'vigilode'),'VigilODE (Ours)'),(('hires',1e-6,'rodas'),'Hairer RODAS'),(('brusselator-1d-n100',1e-8,'vigilode-banded'),'VigilODE (Banded)')):
    r=R[key]; f=fmt_row(r)
    # locate report line in the right section
    sec=rep.split(f"## Problem: `{key[0]}`")[1].split('## Problem:')[0]
    exp=-round(math.log10(key[1])); tolsec=sec.split(f"$rtol = 10^{{-{exp}}}$")[1].split('### Tolerance')[0]
    line=[l for l in tolsec.splitlines() if l.startswith(f"| **{label}**")][0]
    cells=[c.strip() for c in line.strip('|').split('|')]
    work=r.get('work') or {}
    ok = (cells[2]==f['error'] and cells[3]==f['wall'] and cells[4]==str(f['rhs']) and cells[5]==str(f['jac']) and cells[7]==f['alloc'])
    checks.append({'key':list(key),'report_cells':cells,'from_raw':f,'raw_steps':{'work':work,'native_acc_rej':[r['native'].get('counters',{}).get('accepted_steps'),r['native'].get('counters',{}).get('rejected_steps')] if isinstance(r.get('native'),dict) else None},'match_error_wall_rhs_jac_alloc':ok})
out['reproduced_rows']=checks
# matched-error interpolation: vigilode (clipped) vs hairer rodas and radau5, wall and steps, per problem
def interp(pts,key,es):
    pts=sorted(pts,key=lambda p:p['E'])
    for a,b in zip(pts,pts[1:]):
        if a['E']<=es<=b['E'] and a['E']<b['E']:
            t=(math.log(es)-math.log(a['E']))/(math.log(b['E'])-math.log(a['E']))
            return math.exp(math.log(a[key])+t*(math.log(b[key])-math.log(a[key])))
    return None
me=[]
for p in out['rust_problems']:
    base_arm='vigilode-banded' if p.startswith('bruss') else 'vigilode'
    for comp in (['rodas-band','radau5-band','cvode-bdf-band','dvode-mf24'] if p.startswith('bruss') else ['rodas','radau5','cvode-bdf-dense','dvode-mf21']):
        bp=[{'E':r['error_common'],'wall':r['wall_ms_median'],'rtol':r['rtol']} for r in rows if r['problem']==p and r['arm']==base_arm and r['error_common']]
        cp=[{'E':r['error_common'],'wall':r['wall_ms_median'],'rtol':r['rtol']} for r in rows if r['problem']==p and r['arm']==comp and r['error_common']]
        if len(bp)<2 or len(cp)<2: me.append({'problem':p,'comparator':comp,'status':'INSUFFICIENT'}); continue
        lo=max(min(x['E'] for x in bp),min(x['E'] for x in cp)); hi=min(max(x['E'] for x in bp),max(x['E'] for x in cp))
        e={'problem':p,'baseline':base_arm,'comparator':comp,'baseline_E_range':[min(x['E'] for x in bp),max(x['E'] for x in bp)],'comparator_E_range':[min(x['E'] for x in cp),max(x['E'] for x in cp)],'n_tol_baseline':len(bp)}
        if lo<=hi:
            g=math.sqrt(lo*hi); wb,wc=interp(bp,'wall',g),interp(cp,'wall',g)
            e.update({'overlap':[lo,hi],'E_star':g,'comparator_wall_over_vigilode_wall':(wc/wb if wb and wc else None)})
        else: e['status']='NO_OVERLAP'
        # same-tolerance ratios as the report implies
        e['same_tol_wall_ratio_comp_over_vig']={str(x['rtol']):next(y['wall'] for y in cp if y['rtol']==x['rtol'])/x['wall'] for x in bp if any(y['rtol']==x['rtol'] for y in cp)}
        e['same_tol_error_ratio_comp_over_vig']={str(x['rtol']):next(y['E'] for y in cp if y['rtol']==x['rtol'])/x['E'] for x in bp if any(y['rtol']==x['rtol'] for y in cp)}
        me.append(e)
out['matched_error_recomputed']=me
out['rtols_per_problem']={p:sorted({r['rtol'] for r in rows if r['problem']==p}) for p in out['rust_problems']}
json.dump(out,open('/tmp/claude-1000/dsg-audit/bench_recompute.json','w'),indent=1,allow_nan=False)
for k in ('matched_status_counts','matched_basis_counts','matched_with_time_ratio','corpus_problem_n','grid_points','rust_arms_present','free_stepping_vigilode_rows','julia_rows','julia_rows_with_error','julia_problems','rust_problems','build_target_features','build_git_dirty_context','rtols_per_problem'): print(k,out[k])
for c in clip: print(c)
for c in cmp: print(c)
for c in checks: print(json.dumps(c))
for m in me: print(json.dumps(m))
