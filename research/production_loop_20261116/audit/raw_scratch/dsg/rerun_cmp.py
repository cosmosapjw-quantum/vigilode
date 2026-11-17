import json
H="/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116/research/production_unification_20261115/production_holdout/logs/holdout_runs.jsonl"
a=[json.loads(l) for l in open(H)]; b=[json.loads(l) for l in open('/tmp/claude-1000/dsg-audit/rerun_holdout_runs.jsonl')]
same_state=sum(x['result']['y_end']==y['result']['y_end'] for x,y in zip(a,b))
def strip(r):
    r=dict(r['result']); return r
same_all=sum(strip(x)==strip(y) for x,y in zip(a,b))
key=lambda r:(r['problem'],r['tol'],r['arm'])
assert [key(x) for x in a]==[key(y) for y in b]
rows=[]
A={key(x):x for x in a}; B={key(x):x for x in b}
def per(R,p,t,arm):
    r=R[(p,t,arm)]; return r['wall_median_s']/(r['result']['accepted']+r['result']['rejected'])
ratios={'orig':{}, 'rerun_under_perf':{}}
for name,R in (('orig',A),('rerun_under_perf',B)):
    for p in ('hires','oregonator','brusselator_mol32'):
        v1=[per(R,p,t,'ppsw1_t1')/per(R,p,t,'rodas5p') for t in (1e-4,1e-5,1e-6,1e-7,1e-8)]
        v6=[R[(p,t,'ppsw1_t6')]['wall_median_s']/R[(p,t,'ppsw1_t1')]['wall_median_s'] for t in (1e-4,1e-5,1e-6,1e-7,1e-8)]
        us=[1e6*per(R,p,t,'ppsw1_t1') for t in (1e-4,1e-5,1e-6,1e-7,1e-8)]
        usr=[1e6*per(R,p,t,'rodas5p') for t in (1e-4,1e-5,1e-6,1e-7,1e-8)]
        ratios[name][p]={'t1_per_attempt_over_rodas5p':[min(v1),max(v1)],'t6_over_t1_wall':[min(v6),max(v6)],'ppsw1_t1_us_per_attempt':[min(us),max(us)],'rodas5p_us_per_attempt':[min(usr),max(usr)]}
out={'rows':len(a),'bitwise_identical_final_states':same_state,'identical_result_objects_incl_counters':same_all,'ratios':ratios}
json.dump(out,open('/tmp/claude-1000/dsg-audit/rerun_cmp.json','w'),indent=1); print(json.dumps(out,indent=1))
