import csv,json,statistics as st
from pathlib import Path
p=Path(__file__).with_name('bench.csv')
rows=list(csv.DictReader(p.open()))
out={}
for n in (16,64,128):
 blocks=[]
 for block in range(7):
  x=[r for r in rows if int(r['n'])==n and int(r['block'])==block]
  med={arm:st.median(float(r['ns_per_solve']) for r in x if r['arm']==arm) for arm in ['identity','left','right']}
  counts={arm:sorted(set(int(r['columns']) for r in x if r['arm']==arm)) for arm in med}
  checks={arm:sorted(set(int(r['checks']) for r in x if r['arm']==arm)) for arm in med}
  errors={arm:max(float(r['max_achieved_error']) for r in x if r['arm']==arm) for arm in med}
  blocks.append({'block':block,'median_ns':med,'ratio_id_left':med['identity']/med['left'],'ratio_id_right':med['identity']/med['right'],'columns':counts,'checks':checks,'max_achieved_error':errors})
 out[str(n)]={'ratio_id_left_median':st.median(x['ratio_id_left'] for x in blocks),'ratio_id_right_median':st.median(x['ratio_id_right'] for x in blocks),'ratio_id_left_range':[min(x['ratio_id_left'] for x in blocks),max(x['ratio_id_left'] for x in blocks)],'ratio_id_right_range':[min(x['ratio_id_right'] for x in blocks),max(x['ratio_id_right'] for x in blocks)],'blocks':blocks}
assert len(rows)==126
result={'status':'PASS','source':'synthetic dense point matrix','blocks':7,'rows':len(rows),'repetitions_per_measurement':20,'results':out,'limitations':['not native RODAS full-IVP','different achieved errors','no matched-accuracy claim','same tolerance and stage budget only']}
Path(__file__).with_name('bench_summary.json').write_text(json.dumps(result,indent=2,sort_keys=True)+'\n')
print(json.dumps({n:{k:v for k,v in data.items() if k not in ('blocks',)} for n,data in out.items()},indent=2))
