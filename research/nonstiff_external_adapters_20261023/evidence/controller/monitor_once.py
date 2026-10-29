from pathlib import Path
import json,collections,time,os
r=Path(__file__).resolve().parent;d=json.loads((r/'external-comparison/RECEIPTS.json').read_text());a={'utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'receipts':len(d),'status':dict(collections.Counter(x['status'] for x in d)),'phase':dict(collections.Counter(x['phase'] for x in d)),'loadavg':os.getloadavg()};print(json.dumps(a))
with (r/'CAMPAIGN_MONITOR.jsonl').open('a') as f:f.write(json.dumps(a)+'\n')
