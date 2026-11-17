import json, math
H="/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116/research/production_unification_20261115/production_holdout/logs/holdout_runs.jsonl"
orig={(r["problem"],r["tol"],r["arm"]):r for r in map(json.loads,open(H))}
T={"hires":321.8122,"oregonator":360.0,"brusselator_mol32":10.0}
out=[]
for line in open("/tmp/claude-1000/dsg-audit/step_histories.jsonl"):
    r=json.loads(line); p,tol=r["problem"],r["tol"]; o=orig[(p,tol,"ppsw1_t1")]["result"]; ob=orig[(p,tol,"rodas5p")]["result"]
    t=r["t"]; h0=r["h0"]
    assert abs(t[0]-2*h0)<1e-12*max(1,t[0])
    hs=[b-a for a,b in zip(t,t[1:])]
    ratios=[hs[0]/h0]+[b/a for a,b in zip(hs,hs[1:])]
    # the final step is clipped to t_end: exclude it from ratio classes
    core=ratios[:-1]
    n=len(core)
    cap=sum(1 for s in core if s>=1.0999); grow=sum(1 for s in core if s>1.0005); hold=sum(1 for s in core if abs(s-1)<=5e-4); shrink=sum(1 for s in core if s<0.9995)
    growmid=grow-cap
    # steps while the growth-history rule is the binding constraint: a hold step whose 6-step window contains a growth
    blocked=0
    for i,s in enumerate(core):
        if abs(s-1)<=5e-4 and any(x>1.0005 for x in core[max(0,i-6):i]): blocked+=1
    out.append({"problem":p,"tol":tol,"accepted":len(hs),"accepted_matches_u11_log":len(hs)==o["accepted"],
                "final_state_bitwise_equal_u11_log":r["y_end"]==o["y_end"],"rodas5p_state_bitwise_equal_u11_log":r["rodas5p_y_end"]==ob["y_end"],
                "h0":h0,"h_min":min(hs),"h_max":max(hs),"h_max_over_h0":max(hs)/h0,"t_end_over_h_max":T[p]/max(hs),
                "ratios_classified":n,"at_cap_1.10":cap,"fraction_at_cap":cap/n,"growth_below_cap":growmid,"hold":hold,"fraction_hold":hold/n,
                "hold_within_6_steps_after_a_growth":blocked,"fraction_hold_after_growth":blocked/n,
                "cap_or_post_growth_hold":cap+blocked,"fraction_cap_or_post_growth_hold":(cap+blocked)/n,
                "shrink":shrink,"fraction_shrink":shrink/n,"max_ratio":max(core),"min_ratio":min(core)})
json.dump(out,open("/tmp/claude-1000/dsg-audit/step_stats.json","w"),indent=1,allow_nan=False)
for o in out: print({k:(float("%.4g"%v) if isinstance(v,float) else v) for k,v in o.items()})
