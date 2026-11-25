"""Offline exact integer table for R[x,y,z]/(x,y,z)^7.
Rows: exponent triple followed by ordered positive-degree convolution pairs.
Projection to fewer axes/lower order needs no recomputation of products.
"""
from pathlib import Path
import hashlib
root=Path(__file__).resolve().parent
e=[(i,j,k) for d in range(7) for i in range(d+1) for j in range(d-i+1) for k in [d-i-j]]
idx={a:i for i,a in enumerate(e)}
lines=[]
for a in e:
    pairs=[]
    for i,b in enumerate(e[1:],1):
        c=tuple(x-y for x,y in zip(a,b))
        if min(c)>=0 and sum(c)>0:
            j=idx[c]
            pairs.extend([i,j])
    lines.append(' '.join(map(str,[*a,*pairs])))
data=('\n'.join(lines)+'\n').encode('ascii')
p=root.parent.parent/'crates/rodas5p-integrators/src/retro_jet_table.txt'
if p.exists():
    assert p.read_bytes()==data,'refuse to replace a different table'
else:
    p.write_bytes(data)
print('channels',len(e),'pairs',sum((len(l.split())-3)//2 for l in lines),'bytes',len(data),'sha256',hashlib.sha256(data).hexdigest())
