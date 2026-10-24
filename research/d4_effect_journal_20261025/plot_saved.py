#!/usr/bin/env python3
"""Plots of saved protocol evidence only. No solver or producer invocation."""
import os
os.environ.setdefault('MPLCONFIGDIR','/tmp/vigilode-d4-mpl')
import collections,hashlib,json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
root=Path(__file__).resolve().parent
maximum={k:0 for k in range(4)};finals=collections.Counter();sample=[]
with (root/'evidence/traces.jsonl').open() as f:
 for line in f:
  r=json.loads(line)
  maximum[r['cap']]=max(maximum[r['cap']],r['producer'])
  if r['step']==3:finals[r['cap']]+=1
  if r['cap']==1 and r['sequence']=='ALSA':sample.append(r)
assert len(sample)==4 and all(finals[k]==4096 for k in range(4))
sample.sort(key=lambda r:r['step'])
data={'max_producer_per_cap':maximum,'cases_per_cap':dict(finals),'lost_ack_then_success_without_ack':sample,'source_sha256':hashlib.sha256((root/'evidence/traces.jsonl').read_bytes()).hexdigest(),'scope':'Synthetic protocol fixtures; no ODE accuracy or speedup plot'}
(root/'figures').mkdir(exist_ok=True)
(root/'figures/PLOT_DATA.json').write_text(json.dumps(data,indent=2)+'\n')
plt.rcParams.update({'font.size':10,'axes.titlesize':11,'axes.labelsize':10,'legend.fontsize':9,'pdf.fonttype':42})
fig,ax=plt.subplots(1,2,figsize=(7.2,3.65),layout='constrained')
caps=list(range(4));ax[0].plot(caps,caps,'--',color='#8994a0',lw=2,label='Existing attempt cap')
ax[0].plot(caps,[maximum[x] for x in caps],'o',ms=7,color='#176b91',label='Observed maximum')
ax[0].set(xlabel='Attempt cap',ylabel='Producer invocations',xticks=caps,yticks=caps,title='(a) All 16,384 protocol cases',ylim=(-.2,3.45))
ax[0].legend(loc='upper left',frameon=False)
x=list(range(4))
for key,label,color,marker in [('producer','Producer calls','#176b91','o'),('deliverycalls','Sink calls','#9b4b13','s'),('published','Applied effects','#277b4a','^')]:
 ax[1].plot(x,[r[key] for r in sample],marker=marker,lw=1.8,ms=6,color=color,label=label)
ax[1].set(xticks=x,xticklabels=['Accept','Lost ACK','Retry OK','Advance'],yticks=[0,1,2],ylim=(-.15,2.65),ylabel='Cumulative count',title='(b) Same batch across delivery retry')
ax[1].text(1.5,2.4,'Pending until explicit acknowledgement',ha='center',va='center',fontsize=8.5)
ax[1].legend(loc='lower center',bbox_to_anchor=(.5,-.40),ncol=2,frameon=False)
for a in ax:a.grid(axis='y',alpha=.2);a.spines[['top','right']].set_visible(False)
fig.savefig(root/'figures/D4_PROTOCOL_EVIDENCE.png',dpi=220)
fig.savefig(root/'figures/D4_PROTOCOL_EVIDENCE.pdf')
plt.close(fig)
print(json.dumps({'plot':'figures/D4_PROTOCOL_EVIDENCE.png','max_producer':maximum,'sample':'cap1 ALSA','producer':[r['producer'] for r in sample],'sink_calls':[r['deliverycalls'] for r in sample],'effects':[r['published'] for r in sample],'pending_attempt':[r['pending_attempt'] for r in sample]}))
