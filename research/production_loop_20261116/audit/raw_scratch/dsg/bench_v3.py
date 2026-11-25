import json, collections
B='/home/cosmosapjw/vigilode/bench/out/'
d=json.load(open(B+'adversarial_spatial_battery_v3_results.json'))
c=collections.Counter((m['problem'],m['status'],m.get('basis')) for m in d['matched'])
per={}
for (p,s,b),n in sorted(c.items()): per.setdefault(p,{})[f"{s}|{b}"]=n
reasons=collections.Counter(m.get('reason') for m in d['matched'] if m['status']!='MATCHED')
# time ratios for matched dense: comparator vs vigilode baseline
tr=collections.defaultdict(list)
for m in d['matched']:
    if m['status']=='MATCHED' and m.get('time_ratio') is not None: tr[(m['problem'],m['baseline'],m['arm'])].append(m['time_ratio'])
def med(v): v=sorted(v); n=len(v); return v[n//2] if n%2 else 0.5*(v[n//2-1]+v[n//2])
summ=[{'problem':p,'baseline':b,'arm':a,'n_levels':len(v),'median_time_ratio':med(v)} for (p,b,a),v in sorted(tr.items())]
print(json.dumps(d['matched'][[i for i,m in enumerate(d['matched']) if m['status']=='MATCHED'][0]])[:700])
# vigilode step counts per problem (dense free-stepping) to show how small these runs are
steps=[]
for r in d['rows']:
    if r['arm'] in ('vigilode-banded-dense','vigilode-dense','vigilode-banded'):
        steps.append({'problem':r['problem'],'arm':r['arm'],'rtol':r['rtol'],'accepted':r['work']['accepted_steps'],'wall_ms':r['wall_ms_median'],'clipped':r['native'].get('output_clipped_steps'),'err':r['error_common']})
out={'per_problem_matched':per,'non_matched_reasons':dict(reasons),'matched_time_ratio_summary':summ,'vigilode_steps':steps,
     'problem_n':{p['id']:p['n'] for p in d['corpus']['problems']}}
json.dump(out,open('/tmp/claude-1000/dsg-audit/bench_v3.json','w'),indent=1)
print(json.dumps(per,indent=0)); print(reasons); 
for s in summ[:40]: print(s)
for s in steps:
    if s['arm']=='vigilode-banded-dense': print(s)
