"""Figures and transaction-log audit from saved D3 evidence; no solver calls."""
from pathlib import Path
import json, struct, hashlib
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.lines import Line2D

R=Path(__file__).resolve().parent
summary=json.loads((R/'evidence/SUMMARY.json').read_text())
review=json.loads((R/'REVIEW.json').read_text())
assert review['status'].startswith('PASS_ALL_')
runs={str(r['run']):r for r in summary['runs']}
audits={r['run']:r for r in review['runs']}
failures=[json.loads(s) for s in (R/'evidence/failures.jsonl').read_text().splitlines() if s]
def bits(x): return struct.pack('>d',float(x))
counts={k:0 for k in runs}; phases={}
for f in failures:
 k=str(f['run']);counts[k]+=1;phases[f['phase']]=phases.get(f['phase'],0)+1
 assert all(f[x] is True for x in ['accepted_state_preserved','incoming_preserved','cursor_preserved','attempt_budget_increased'])
 assert all(bits(a)==bits(b) for a,b in zip(f['state_before'],f['state_after']))
 assert len(f['state_before'])==len(f['state_after'])==30
 assert all(bits(a)==bits(b) for a,b in zip(f['incoming_before'],f['incoming_after']))
 assert len(f['incoming_before'])==len(f['incoming_after'])==30
 assert f['cursor_before']==f['cursor_after'] and f['epoch_before']==f['epoch_after']
 assert f['attempts_after']==f['attempts_before']+1
 assert f['rejected_after']==f['rejected_before']+1
 assert f['external_effects_rolled_back'] is False
for k,r in runs.items():
 assert counts[k]==r['rejected']
 assert r['attempts']==r['accepted']+r['rejected']
 assert r['forbidden_rhs']==0
 assert r['stage_rhs']==8*r['attempts']
 assert r['curve_rhs_vectors']==r['curve_builds']*(2 if r['degree']==3 else 4)
 assert r['analytic_jf_vectors']==r['curve_builds']*(0 if r['degree']==3 else 2)
receipt={'status':'PASS_SAVED_REFUSAL_RECORDS','records':len(failures),'phases':phases,
 'per_run_records':counts,'bitwise_state_and_incoming_preservation':True,
 'cursor_epoch_preservation':True,'attempt_and_rejected_budget_increments':True,
 'external_side_effects_rollback_claim':False,'new_solver_runs':0,
 'source_sha256':hashlib.sha256((R/'evidence/failures.jsonl').read_bytes()).hexdigest()}
(R/'REFUSAL_REVIEW.json').write_text(json.dumps(receipt,indent=2)+'\n')

out=R/'figures';out.mkdir(exist_ok=True)
plt.rcParams.update({'font.size':10,'axes.spines.top':False,'axes.spines.right':False,
 'pdf.fonttype':42,'savefig.facecolor':'white'})
colors={3:'#1864ab',5:'#a61e4d'}
fig,axes=plt.subplots(2,2,figsize=(12,7.5),sharex=True,sharey=True,layout='constrained')
for r in runs.values():
 ax=axes[0 if r['alpha']==0 else 1,0 if r['curve_budget']==2e-6 else 1]
 rows=sorted((q for q in review['query_audit_rows'] if str(q['run'])==str(r['run'])),key=lambda q:float(q['t']))
 t=[float(q['t']) for q in rows]; budget=r['curve_budget']; c=colors[r['degree']]
 ax.plot(t,[float(q['maxradius'])/budget for q in rows],color=c,lw=1.5)
 ax.plot(t,[max(float(q['maxabs_error'])/budget,1e-13) for q in rows],color=c,ls='--',lw=1.1,alpha=.9)
for i,alpha in enumerate([0,4]):
 for j,budget in enumerate([2e-6,2e-8]):
  ax=axes[i,j];ax.set_yscale('log');ax.set_ylim(1e-7,2);ax.set_xlim(0,.25)
  ax.axhline(1,color='#495057',lw=1,ls=':');ax.grid(which='major',alpha=.18)
  ax.set_title(rf'$\alpha={alpha}$, full33 absolute budget ${budget:.0e}$')
  if i==1:ax.set_xlabel('Normalized time')
  if j==0:ax.set_ylabel('Full33 maximum / budget')
handles=[Line2D([0],[0],color=colors[3],lw=2,label='Cubic'),Line2D([0],[0],color=colors[5],lw=2,label='Quintic'),
 Line2D([0],[0],color='#343a40',lw=1.5,label='Returned certified radius'),
 Line2D([0],[0],color='#343a40',ls='--',lw=1.2,label='Actual returned-value error (Decimal80)')]
fig.legend(handles=handles,loc='outside lower center',ncol=4,frameon=False)
fig.suptitle('D3: saved query bounds and independently checked errors',fontsize=15)
for suffix in ['png','pdf']:fig.savefig(out/f'D3_CERTIFIED_OUTPUT.{suffix}',dpi=180)
plt.close(fig)

order=[0,2,1,3,4,6,5,7];ordered=[runs[str(k)] for k in order]
fig,ax=plt.subplots(figsize=(12,5),layout='constrained')
x=list(range(8));stage=[r['stage_rhs'] for r in ordered];extra=[r['curve_rhs_vectors'] for r in ordered]
ax.bar(x,stage,color='#2b8a3e',label='Stage RHS vector calls')
ax.bar(x,extra,bottom=stage,color='#94d82d',label='Curve RHS vector calls (including Jf inputs)')
for i,r in enumerate(ordered):
 ax.text(i,stage[i]+extra[i]+100,f"{stage[i]+extra[i]:,}",ha='center',fontsize=10)
labels=[f"{'Cubic' if r['degree']==3 else 'Quintic'}\nalpha={r['alpha']:.0f}\nbudget={r['curve_budget']:.0e}" for r in ordered]
ax.set_xticks(x,labels);ax.set_ylabel('RHS vector evaluations, including refused attempts')
ax.set_ylim(0,max(a+b for a,b in zip(stage,extra))*1.14);ax.grid(axis='y',alpha=.2);ax.set_axisbelow(True)
ax.set_title('D3 workload at each frozen accuracy budget',fontsize=15)
ax.legend(loc='upper left',frameon=False)
fig.text(.5,-.025,'Additional quintic analytic Jf vector evaluations: 594 total. Interval arithmetic and other work are not priced. No wall-time speedup claim.',ha='center',fontsize=9)
for suffix in ['png','pdf']:fig.savefig(out/f'D3_WORKLOAD.{suffix}',dpi=180,bbox_inches='tight')
plt.close(fig)
print(json.dumps({'refusal_review':receipt,'figures':[p.name for p in sorted(out.iterdir())]},indent=2))
