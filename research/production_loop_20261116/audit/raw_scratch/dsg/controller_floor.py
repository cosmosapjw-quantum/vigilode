#!/usr/bin/env python3
"""Arithmetic of the PPSW1 growth rule (ppsw1.rs:841-956) applied to the logged h0 of each U11 run.
Not a solver run: N_min is the number of accepted steps the controller needs to reach t_end from
t0 + 2 h0 if every step is accepted with zero error (growth always proposed). Inputs come from the raw log."""
import json, math
H="/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116/research/production_unification_20261115/production_holdout/logs/holdout_runs.jsonl"
T={"hires":321.8122,"oregonator":360.0,"brusselator_mol32":10.0}
def nmin(h0,T,cap,hist=6,hysteresis=True):
    t=2*h0; h=h0; ratios=[]; n=0; g=0; hprev=h0
    while t<T*(1-1e-15):
        hh=min(h,T-t); sigma=hh/hprev; t+=hh; hprev=hh; n+=1
        ratios.append(sigma); ratios=ratios[-hist:]
        if sigma>1.0: g+=1
        if hysteresis and any(r>1.0 for r in ratios): h=hh
        else: h=hh*cap
        if n>10**7: break
    return n,g
runs=[json.loads(l) for l in open(H)]
R={(r["problem"],r["tol"],r["arm"]):r for r in runs}
out=[]
for (p,tol,arm),r in sorted(R.items()):
    if arm!="ppsw1_t1": continue
    h0=r["result"]["h0"]; acc=r["result"]["accepted"]; rej=r["result"]["rejected"]
    b=R[(p,tol,"rodas5p")]["result"]; batt=b["accepted"]+b["rejected"]
    n110,g110=nmin(h0,T[p],1.10); n120,_=nmin(h0,T[p],1.20); n150,_=nmin(h0,T[p],1.50); n150f,_=nmin(h0,T[p],1.50,hysteresis=False)
    out.append({"problem":p,"tol":tol,"h0":h0,"t_end_over_h0":T[p]/h0,"ppsw1_accepted":acc,"ppsw1_rejected":rej,"rodas5p_attempts":batt,
                "n_min_cap1.10_hyst6":n110,"growth_steps_in_n_min":g110,"n_min_over_accepted":n110/acc,"n_min_over_rodas5p_attempts":n110/batt,
                "n_min_cap1.20_hyst6":n120,"n_min_cap1.50_hyst6":n150,"n_min_cap1.50_no_hyst":n150f,
                "max_fraction_of_accepted_steps_at_cap":math.ceil(acc/7)/acc})
summary={"mean_growth_per_step_cap1.10_hyst6":1.10**(1/7),"mean_growth_per_step_cap1.50_hyst6":1.5**(1/7),
         "structural_max_fraction_of_steps_with_sigma_gt_1":1/7,
         "growth_err_threshold":(0.85/1.05)**5,
         "rows":out}
# Amdahl bound for a 6-way stage-parallel PPSW1 against RODAS5P given a step ratio rho:
# speed <= 1/(rho*(f + (1-f)/8)), f = (J+LU) share of a RODAS5P step, chain share (1-f) reduced 8x (6 parallel stages replace 8 sequential)
m=json.load(open("/tmp/claude-1000/dsg-audit/u11_recompute.json"))
rhos=sorted(q["steps_ratio"] for e in m["matched"] if e["arm"]=="ppsw1_t1" for q in e["points"])
def fmax(rho,target=1.15):
    x=1/(target*rho)   # need f + (1-f)/8 <= x
    return (x-1/8)/(1-1/8)
summary["amdahl"]={"model":"speed <= 1/(rho*(f+(1-f)/8)); f = share of Jacobian+LU in one RODAS5P step; ideal 6-way stage parallelism, zero overhead",
    "measured_matched_error_step_ratios":rhos,
    "max_speed_at_f0":[8/r for r in rhos],
    "max_f_for_speed_1.15":[fmax(r) for r in rhos],
    "rho_needed_for_1.15_at_f0":8/1.15,"rho_needed_for_1.15_at_f0.5":1/(1.15*(0.5+0.5/8))}
json.dump(summary,open("/tmp/claude-1000/dsg-audit/controller_floor.json","w"),indent=1,allow_nan=False)
for o in out: print({k:(round(v,4) if isinstance(v,float) else v) for k,v in o.items()})
print(json.dumps({k:v for k,v in summary.items() if k!="rows"},indent=1))
