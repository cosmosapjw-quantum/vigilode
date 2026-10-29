import pandas as pd,numpy as np,json
from pathlib import Path
p=Path(__file__).resolve().parent
x=pd.read_csv(p/'bench.csv')
rows=[]
for (n,backend),grp in x.groupby(['n','backend']):
    per=[]
    for block,g in grp.groupby('block'):
        a=g[g['mode']=='always'];t=g[g['mode']=='trigger']
        m_a=a['elapsed_ns'].median();m_t=t['elapsed_ns'].median()
        per.append(m_a/m_t)
        assert a['columns'].nunique()==t['columns'].nunique()==1
        assert a['achieved_inf'].iloc[0]==t['achieved_inf'].iloc[0]
        assert a['stage_bound'].iloc[0]==t['stage_bound'].iloc[0]
        assert a['original_checks'].iloc[0]==t['original_checks'].iloc[0]
    z=grp.groupby('mode')[['columns','candidate_builds','original_checks','operator_apps','pc_apps','achieved_inf','stage_bound']].first()
    rows.append({'n':int(n),'backend':backend,'paired_blocks':len(per),'ratio_always_over_trigger_median':float(np.median(per)),
     'ratio_range':[float(min(per)),float(max(per))], 'all_blocks_faster':bool(all(v>1 for v in per)),
     'median_time_ns':{mode:float(grp[grp['mode']==mode]['elapsed_ns'].median()) for mode in ['always','trigger']},
     'work':{mode:{k:float(v) for k,v in z.loc[mode].to_dict().items()} for mode in ['always','trigger']},
     'ratios_by_block':list(map(float,per))})
inter=[]
for n in x.n.unique():
    block_ratios=[]
    for block in range(7):
        d=x[(x.n==n)&(x.backend=='dense')&(x['mode']=='trigger')&(x.block==block)]
        s=x[(x.n==n)&(x.backend=='sparse')&(x['mode']=='trigger')&(x.block==block)]
        block_ratios.append(d.elapsed_ns.median()/s.elapsed_ns.median())
        assert d.achieved_inf.iloc[0]==s.achieved_inf.iloc[0],(n,block,'achieved')
        assert d['columns'].iloc[0]==s['columns'].iloc[0],(n,block,'columns')
    inter.append({'n':int(n),'dense_over_sparse_trigger_median':float(np.median(block_ratios)),
      'range':list(map(float,[min(block_ratios),max(block_ratios)])),'ratios_by_block':list(map(float,block_ratios))})
out={'schema':'vigilode.sg01.benchmark.v2','scope':'standalone dense represented-J vs exact structural-zero sparse snapshot, linear stage only',
'paired_design':'7 ABBA/BAAB blocks per n/backend, 6 repetitions per arm per appearance, same physical-equation residual acceptance','timed_rows':len(x),'summary':rows,'sparse_vs_dense':inter,
'limits':['Full-IVP not run','Native VigilODE not modified','Same achieved error per fixture and candidate trigger, not a general production claim','Amortized sparse precomputation excluded from stage runtime; caller must justify reuse','All arms use identical acceptance trigger; always arm computes otherwise unnecessary candidate prefixes','Source/clock/physical33 not certified']}
(p/'bench_summary.json').write_text(json.dumps(out,indent=2,ensure_ascii=False)+'\n')
for r in rows:print(r['n'],r['backend'],round(r['ratio_always_over_trigger_median'],3),[round(z,3) for z in r['ratio_range']],r['all_blocks_faster'],r['work']['always']['candidate_builds'],r['work']['trigger']['candidate_builds'])
print('cross:',inter)
import matplotlib
matplotlib.use('Agg');import matplotlib.pyplot as plt
fig,ax=plt.subplots(figsize=(7.0,4))
for r in rows:
    shift=-0.7 if r['backend']=='dense' else 0.7
    ax.scatter([r['n']+shift]*7,r['ratios_by_block'],alpha=.30,label=None)
    ax.scatter(r['n']+shift,r['ratio_always_over_trigger_median'],marker='D',s=65,
        label=f"{r['backend']} n={r['n']}")
ax.axhline(1,linestyle='--',linewidth=1)
ax.set(xlabel='System dimension n',ylabel='Always build / triggered-only wall-time ratio',title='SG01 paired standalone linear-stage negative-cost control')
ax.grid(alpha=.2)
ax.legend(ncol=2,fontsize=7)
fig.tight_layout();fig.savefig(p/'bench_ratios.png',dpi=160)
