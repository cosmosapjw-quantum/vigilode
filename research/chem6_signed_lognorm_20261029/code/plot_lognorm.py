import json
from fractions import Fraction as F
import matplotlib.pyplot as plt
from chem6_exact import default_fixture, sharp_mu, entrywise_upper

e,w,rates=default_fixture()
rows=[]
for eta in [F(0),F(1,4),F(1,2),F(3,4),F(1)]:
    boxes=[]
    for lo,hi in rates:
        mid=(lo+hi)/2
        half=(hi-lo)/2
        boxes.append((mid-eta*half,mid+eta*half))
    sharp=sharp_mu(e,w,boxes).value
    naive=entrywise_upper(e,w,boxes)
    assert sharp<=naive
    rows.append({'eta':str(eta),'sharp':str(sharp),'entrywise':str(naive),
                 'sharp_float':float(sharp),'entrywise_float':float(naive),
                 'gain_fraction':float((naive-sharp)/naive) if naive else None})
with open('FIGURE_DATA.json','w') as f:
    json.dump({'schema':'vigilode.chem6.mu01.figure.v1',
               'scope':'synthetic rate boxes, exact Fraction data; no physical performance claim',
               'rows':rows},f,indent=2)
plt.figure(figsize=(8.5,5.1))
plt.plot([float(F(r['eta'])) for r in rows],[r['sharp_float'] for r in rows],marker='o',label='Sharp correlated lognorm')
plt.plot([float(F(r['eta'])) for r in rows],[r['entrywise_float'] for r in rows],marker='s',label='Independent-entry bound')
plt.xlabel('Fraction of declared rate-box half-width')
plt.ylabel('Weighted-infinity logarithmic norm upper')
plt.title('Frozen chemical6 source topology, synthetic rates')
plt.grid(alpha=0.3)
plt.legend()
plt.tight_layout()
plt.savefig('CHEM6_LOGNORM.png',dpi=180)
print('FIGURE_DATA_PASS',len(rows),'max gap',max(r['entrywise_float']-r['sharp_float'] for r in rows))
