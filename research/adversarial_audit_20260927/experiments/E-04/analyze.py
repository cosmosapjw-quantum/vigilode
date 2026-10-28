import json, glob, math, hashlib, os, subprocess, re
import matplotlib; matplotlib.use("Agg"); import matplotlib.pyplot as plt
RUN="/home/cosmosapjw/.local/state/vigilode/adversarial-audit/20260927T121000Z"; os.chdir(RUN+"/exp/E-04")
def sha(p): return hashlib.sha256(open(p,'rb').read()).hexdigest()
files=sorted(f for f in glob.glob("p*.jsonl") if not f.startswith("smoke"))
rows=[]; meta={}
for f in files:
    for l in open(f):
        d=json.loads(l)
        if d.get("meta"): meta[d["problem"]]=d["note"]; continue
        d["_file"]=f; rows.append(d)
def key(d): return (d["problem"], d["arm"])
arms={}
for d in rows: arms.setdefault(key(d),[]).append(d)
for v in arms.values(): v.sort(key=lambda d:d["k"])
def slopes(v,field):
    out=[None]
    for a,b in zip(v,v[1:]):
        try: out.append(math.log2(a[field]/b[field]))
        except (ZeroDivisionError,ValueError): out.append(float("nan"))
    return out
lines=[]; table=[]
def P(s): lines.append(s); print(s)
P("| problem | arm | k | h | err_max | slope | err_end | slope | rej? | fails | rhs | jvp | lin_it | eta_mean | eta_max | clamp@0.5 | status |")
P("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
for (prob,arm),v in sorted(arms.items()):
    sm=slopes(v,"err_max_rel_inf"); se=slopes(v,"err_end_rel_inf")
    for d,a,b in zip(v,sm,se):
        fmt=lambda x: "" if x is None else f"{x:.2f}"
        em=d["eta_mean"]; ex=d["eta_max"]; cl=d["eta_clamped_at_max_fraction"]
        P(f"| {prob} | {arm} | {d['k']} | {d['h']:.2e} | {d['err_max_rel_inf']:.3e} | {fmt(a)} | {d['err_end_rel_inf']:.3e} | {fmt(b)} | {d['would_reject_steps']}/{d['steps']} | {d['linear_solve_failures']} | {d['rhs_calls']} | {d['jvp_calls']} | {d['linear_iterations']} | {'' if em is None else f'{em:.1e}'} | {'' if ex is None else f'{ex:.1e}'} | {'' if cl is None else f'{cl:.2f}'} | {d['status']} {d['message'][:50]} |")
        table.append({"problem":prob,"arm":arm,"k":d["k"],"h":d["h"],"err_max":d["err_max_rel_inf"],"slope_max":a,"err_end":d["err_end_rel_inf"],"slope_end":b,"would_reject":d["would_reject_steps"],"steps":d["steps"],"linear_solve_failures":d["linear_solve_failures"],"rhs_calls":d["rhs_calls"],"jvp_calls":d["jvp_calls"],"linear_iterations":d["linear_iterations"],"eta_mean":em,"eta_max":ex,"eta_min":d["eta_min"],"eta_clamped_frac":cl,"tau_mean":d["tau_mean"],"status":d["status"],"message":d["message"],"adaptive_accepted":d["adaptive_accepted"],"adaptive_rejected":d["adaptive_rejected"],"adaptive_h_min":d["adaptive_h_min"],"adaptive_h_max":d["adaptive_h_max"],"roundoff_floor_estimate":d["roundoff_floor_estimate"],"wall_s":d["wall_s"]})
# verdict statistics per arm: consecutive halvings with slope>=4.5 from the start, floor, crossover vs direct
verd={}
for (prob,arm),v in sorted(arms.items()):
    if arm.startswith("adaptive"): continue
    direct=arms.get((prob,"direct"),[])
    dmap={d["k"]:d for d in direct}
    for field in ("err_max_rel_inf","err_end_rel_inf"):
        s=slopes(v,field); n=0
        for x in s[1:]:
            if x is not None and x>=4.5: n+=1
            else: break
        n_any=sum(1 for x in s[1:] if x is not None and x>=4.5)
        okv=[d for d in v if d["status"]=="ok"] or v
        floor=min(d[field] for d in okv)
        kfloor=[d["k"] for d in okv if d[field]==floor][0]
        # crossover: first k where arm error > 3x direct error (direct above its own floor)
        cross=None
        for d in v:
            dd=dmap.get(d["k"])
            if dd and d[field]>3*dd[field] and dd[field]>5e-15:
                cross=d["k"]; break
        verd[f"{prob}|{arm}|{field}"]={"consecutive_halvings_slope_ge_4.5_from_start":n,"halvings_slope_ge_4.5_any":n_any,"n_halvings":len(v)-1,"floor":floor,"k_at_floor":kfloor,"first_k_with_error_gt_3x_direct":cross,"slopes":[None if x is None else round(x,2) for x in s]}
# plots
for prob in sorted({p for p,_ in arms}):
    fig,axs=plt.subplots(1,2,figsize=(13,5.2))
    for ax,field,title in zip(axs,("err_max_rel_inf","err_end_rel_inf"),("max_k ||y_k-y*(t_k)||_inf/||y*||_inf","endpoint t=1")):
        for (p,arm),v in sorted(arms.items()):
            if p!=prob or arm.startswith("adaptive"): continue
            hs=[d["h"] for d in v if d["status"]=="ok"]; es=[d[field] for d in v if d["status"]=="ok"]  # failed rows (error 0) omitted from the plot; see table
            style="k-o" if arm=="direct" else ("--s" if arm.startswith("gmres") else ("-.^" if arm.startswith("forcing") else ":d"))
            ax.loglog(hs,es,style,label=arm,ms=4)
        for (p,arm),v in sorted(arms.items()):
            if p!=prob or not arm.startswith("adaptive"): continue
            for d in v:
                if d["status"]=="ok": ax.loglog([d["h"]],[d[field]],"r*",ms=9); ax.annotate(arm.split(":")[1],(d["h"],d[field]),fontsize=7,color="r")
        h=[d["h"] for d in arms[(prob,"direct")]]; c=arms[(prob,"direct")][0][field]/h[0]**5
        ax.loglog(h,[c*x**5 for x in h],"k:",lw=0.8,label="h^5 ref")
        ax.set_xlabel("h"); ax.set_ylabel("relative inf-norm error"); ax.set_title(f"{prob}: {title}"); ax.grid(True,which="both",alpha=0.3)
    axs[1].legend(fontsize=7,loc="lower right"); fig.suptitle(f"E-04 {prob} — {meta.get(prob,'')}",fontsize=9); fig.tight_layout(); fig.savefig(f"e04_{prob}.png",dpi=130); plt.close(fig)
json.dump({"table":table,"verdict_stats":verd,"meta":meta},open("analysis.json","w"),indent=1)
open("table.md","w").write("\n".join(lines)+"\n")
print(json.dumps(verd,indent=0)[:6000])
