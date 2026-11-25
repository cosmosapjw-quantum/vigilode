"""Check the actual Rust binary64 interval endpoints against independent high-precision functions."""
from pathlib import Path
import json,struct,mpmath as mp
mp.mp.dps=100
root=Path(__file__).resolve().parent
def f(h):
    return mp.mpf(struct.unpack('>d',bytes.fromhex(h))[0])
rows=[]
for line in (root/'regression.log').read_text().splitlines():
    if 'PHI_CASE ' not in line:continue
    a=json.JSONDecoder().raw_decode(line.split('PHI_CASE ',1)[1])[0]
    x,j,lo,hi=f(a['x']),a['j'],f(a['lo']),f(a['hi'])
    ref=mp.exp(-x) if j==0 else mp.hyp1f1(1,j+1,-x)/mp.factorial(j)
    assert lo<=ref<=hi,(x,j,lo,ref,hi)
    rows.append({'x':float(x),'j':j,'width':float(hi-lo),'relative_width':float((hi-lo)/ref) if ref>mp.mpf('1e-300') else None})
assert len(rows)==52,len(rows)
out={'cases':len(rows),'underbounds':0,'max_absolute_width':max(r['width'] for r in rows),'rows':rows,'mpmath':mp.__version__}
(root/'PHI_AUDIT.json').write_text(json.dumps(out,indent=2,sort_keys=True)+'\n')
print(json.dumps({k:v for k,v in out.items() if k!='rows'}))
