"""Check that reports are linked to the original retained observations."""
import gzip,json,statistics,sys
from pathlib import Path
out=Path(sys.argv[1]);load=lambda p:json.loads((out/p).read_text())
cal={};tim={}
with gzip.open(out/'calibration.jsonl.gz','rt') as f:
    for line in f:
        a=json.loads(line);r=a['row'];key=(r['case'],r['grid'],r['arm'],r['rtol'],r['h0_scale'])
        assert key not in cal;cal[key]=a
with gzip.open(out/'timing.jsonl.gz','rt') as f:
    for line in f:
        a=json.loads(line);r=a['row'];key=(r['case'],r['grid'],r['arm'],a['block'])
        assert key not in tim;tim[key]=a
for e in load('FROZEN_MATCHED_SELECTION.json'):
    for arm,q in e['matched'].items():
        rs=[cal[(e['case'],e['grid'],arm,q['rtol'],hs)] for hs in [.5,1.,2.]]
        assert all(r['metrics']['admitted'] for r in rs)
        assert q['E_max']==max(r['metrics']['E_up'] for r in rs)
        assert q['E_nominal']==rs[1]['metrics']['E_up']
        assert q['states_sha256']['1.0']==rs[1]['row']['states_sha256']
        assert .5*e['anchor']<=q['E_nominal']<=q['E_max']<=e['anchor']
for u in load('timing_units.json'):
    a=tim[(u['case'],u['grid'],u['arm'],u['block'])];r=a['row']
    if u['valid']:
        assert u['median_ns']==statistics.median(r['elapsed_ns'])
        assert r['states_sha256']==u['states_sha256']
        assert a['metrics']['E_up']==u['E_up']
result={'status':'PASS_RAW_TO_METRIC_TO_SELECTION_TO_TIMING_LINKS','calibration_rows':len(cal),'timing_rows':len(tim)}
(out/'LINK_AUDIT.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
