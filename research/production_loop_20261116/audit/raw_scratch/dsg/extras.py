import json, math
m=json.load(open('u11_recompute.json'))
H="/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116/research/production_unification_20261115/production_holdout/logs/holdout_runs.jsonl"
runs=[json.loads(l) for l in open(H)]
fits=[]
for p in ("hires","oregonator","brusselator_mol32"):
    for arm in ("rodas5p","ppsw1_t1","ppsw1_t6"):
        pts=[(r["result"]["accepted"]+r["result"]["rejected"], r["wall_median_s"]) for r in runs if r["problem"]==p and r["arm"]==arm]
        n=len(pts); sx=sum(x for x,_ in pts); sy=sum(y for _,y in pts); sxx=sum(x*x for x,_ in pts); sxy=sum(x*y for x,y in pts)
        b=(n*sxy-sx*sy)/(n*sxx-sx*sx); a=(sy-b*sx)/n
        fits.append({"problem":p,"arm":arm,"slope_us_per_attempt":1e6*b,"intercept_ms":1e3*a,"intercept_share_of_loosest_tol_wall":a/min(y for _,y in pts)})
F={(f["problem"],f["arm"]):f for f in fits}
marg=[{"problem":p,"marginal_per_attempt_ratio_t1_over_rodas5p":F[(p,"ppsw1_t1")]["slope_us_per_attempt"]/F[(p,"rodas5p")]["slope_us_per_attempt"],
       "t6_minus_t1_slope_us_per_attempt":F[(p,"ppsw1_t6")]["slope_us_per_attempt"]-F[(p,"ppsw1_t1")]["slope_us_per_attempt"],
       "t6_minus_t1_intercept_ms":F[(p,"ppsw1_t6")]["intercept_ms"]-F[(p,"ppsw1_t1")]["intercept_ms"]} for p in ("hires","oregonator","brusselator_mol32")]
ss=json.load(open('step_stats.json'))
for s in ss:
    s["max_possible_growth_steps"]=math.ceil(s["ratios_classified"]/7); s["limiter_saturation"]=s["at_cap_1.10"]/s["max_possible_growth_steps"]
json.dump(ss,open('step_stats.json','w'),indent=1)
json.dump({"wall_linear_fits":fits,"marginal":marg},open('extras.json','w'),indent=1)
for f in fits: print({k:(round(v,4) if isinstance(v,float) else v) for k,v in f.items()})
for x in marg: print({k:(round(v,4) if isinstance(v,float) else v) for k,v in x.items()})
for s in ss: print(s["problem"],s["tol"],"at_cap",s["at_cap_1.10"],"/",s["max_possible_growth_steps"],"sat=%.3f"%s["limiter_saturation"],"frac_cap=%.4f"%s["fraction_at_cap"],"cap_or_hold=%.3f"%s["fraction_cap_or_post_growth_hold"])
