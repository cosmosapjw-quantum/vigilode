import json, collections
B='/home/cosmosapjw/vigilode/bench/out/'
out={}
for f in ['rust_bench_results.json','adversarial_rust_results.json','adversarial_spatial_grid_rust_results.json','adversarial_spatial_grid_v2_results.json','adversarial_spatial_battery_v3_results.json']:
    d=json.load(open(B+f)); rows=d['rows']
    probs={p['id']:{'n':p['n'],'banded':p.get('banded'),'kind':p.get('kind'),'t_end':p.get('t_end')} for p in d['corpus']['problems']}
    arms=collections.Counter(r['arm'] for r in rows)
    st=collections.Counter((r['arm'].startswith('vigilode'), (r['status'] if isinstance(r['status'],str) else json.dumps(r['status'])[:40])) for r in rows)
    clipped=[(r['problem'],r['arm'],r['rtol'],r['native'].get('output_clipped_steps'),r['native'].get('counters',{}).get('accepted_steps')) for r in rows if r['arm'].startswith('vigilode') and isinstance(r.get('native'),dict)]
    ms=collections.Counter((m['status'],m.get('basis')) for m in d.get('matched',[]))
    out[f]={'corpus_id':d['corpus']['id'],'problems':probs,'max_n':max(p['n'] for p in probs.values()),'arms_with_rows':dict(arms),
            'matched_status_basis':{f"{a}|{b}":c for (a,b),c in ms.items()},'timing_authority':d.get('timing_authority'),
            'vigilode_dense_rows':sum(c for a,c in arms.items() if a.startswith('vigilode') and 'dense' in a),
            'vigilode_clipped_rows':sum(c for a,c in arms.items() if a in('vigilode','vigilode-banded')),
            'status_counts':{f"vig={a}|{b}":c for (a,b),c in st.items()},
            'dense_rows_with_clipped_steps':sum(1 for (p,a,t,c,acc) in clipped if 'dense' in a and c),
            'warmups':d['corpus'].get('warmups'),'repetitions':d['corpus'].get('repetitions')}
    print('##',f); print(json.dumps(out[f])[:2600])
json.dump(out,open('/tmp/claude-1000/dsg-audit/bench_scan.json','w'),indent=1)
