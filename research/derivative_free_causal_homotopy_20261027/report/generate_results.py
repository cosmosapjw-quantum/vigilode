#!/usr/bin/env python3
import json,math,re,statistics,pathlib
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
HERE=pathlib.Path(__file__).resolve().parent
ROOT=HERE.parent
def read(p):return json.loads((ROOT/p).read_text())
struct=read("evidence/STRUCTURAL_SUMMARY.json")
hom=read("homotopy/SUMMARY.json")
causal=read("causal/WORK_RESULTS.json")
plt.rcParams.update({"font.family":"DejaVu Sans","font.size":10,"axes.spines.top":False,"axes.spines.right":False,"pdf.fonttype":42,"ps.fonttype":42})
fig,axs=plt.subplots(1,2,figsize=(10.8,4.4),layout="constrained")
ax=axs[0]
labels=[];ys=[];lo=[];hi=[]
for d in struct:
 if d["candidate"]!="quotient-fresh":continue
 labels.append(f'n={d["n"]}, workers={d["threads"]}')
 ys.append(d["median_ratio"]);lo.append(d["median_ratio"]-d["range"][0]);hi.append(d["range"][1]-d["median_ratio"])
order=sorted(range(len(labels)),key=lambda j:(int(labels[j].split(",")[0].split("=")[1]),labels[j]))
labels=[labels[j] for j in order];ys=[ys[j]for j in order];lo=[lo[j]for j in order];hi=[hi[j]for j in order]
ax.errorbar(ys,np.arange(len(labels)),xerr=[lo,hi],fmt="o",color="#126782",capsize=5,lw=2)
ax.axvline(1,color="#a0a0a0",ls="--",lw=1)
ax.set_yticks(np.arange(len(labels)),labels);ax.invert_yaxis()
ax.set_ylim(len(labels)-.6,-.5)
ax.set_xlabel("Sequential / quotient trajectory time")
ax.set_title("(a) Full trajectory, fresh plan each step",loc="left",fontweight="bold")
ax.set_xlim(0.7,2.5);ax.grid(axis="x",alpha=.2)
for i,y in enumerate(ys):ax.text(y,i-.16,f"{y:.3f}",ha="center",fontsize=9)
ax=axs[1]
names=["Causal cache\nRiccati trajectory","RHS homotopy\nmidpoint trajectory","Exact quotient\nmean-field trajectory"]
base=[1868,424,1024];new=[1702,296,576]
x=np.arange(3)
ax.bar(x-.17,[1]*3,.34,label="Baseline",color="#b8c4cc")
ax.bar(x+.17,np.array(new)/base,.34,label="New route",color="#126782")
for i,(b,n)in enumerate(zip(base,new)):
 ax.text(i+.17,n/b+.04,f"{b}→{n}\n−{100*(1-n/b):.2f}%",ha="center",va="bottom",fontsize=9)
ax.set_ylim(0,1.38);ax.set_xticks(x,names);ax.set_ylabel("RHS calls / own baseline");ax.legend(loc="upper right",frameon=False)
ax.set_title("(b) Qualified trajectories, all work charged",loc="left",fontweight="bold")
fig.savefig(HERE/"performance.pdf");fig.savefig(HERE/"performance.png",dpi=180);plt.close(fig)
fig,axs=plt.subplots(1,2,figsize=(10.8,4.4),layout="constrained")
errors=[1.8289908948476707e-8,3.9703010323854926e-10,7.258443845969964e-12,1.1898815266420115e-13]
steps=np.array([4,8,16,32]);slopes=[math.log2(a/b)for a,b in zip(errors,errors[1:])]
ax=axs[0];ax.loglog(steps,errors,"o-",color="#126782",lw=2,label="Exact-quotient RODAS target")
ax.loglog(steps,errors[0]*(steps/steps[0])**-5,"--",color="#aaa",label="Order 5 guide")
ax.set_xlabel("Steps on [0,1]");ax.set_ylabel("Maximum endpoint component error")
ax.set_title("(a) Affine specialization",loc="left",fontweight="bold")
ax.legend(frameon=False,fontsize=9);ax.grid(which="both",alpha=.2);ax.text(.03,.06,"Observed slopes: "+", ".join(f"{s:.2f}"for s in slopes),transform=ax.transAxes,fontsize=9)
ax=axs[1]
for mode,warm,label,marker,color in [("Picard",False,"Picard / zero seed","o","#b66b35"),("Screened",True,"Screened / warm seed","x","#126782")]:
 vals=[next(a for a in r["arms"] if a["mode"]==mode and a["warm_previous_stage"]==warm) for r in hom["trajectory_pairs"]]
 n=[r["steps"] for r in hom["trajectory_pairs"]];err=[v["max_grid_error"]for v in vals]
 ax.loglog(n,err,marker+"-",color=color,label=label,lw=1.5,markersize=7)
ax.set_xlabel("Steps on [0,1]");ax.set_ylabel("Maximum error over every grid state")
ax.set_title("(b) Nonlinear implicit-midpoint consumer",loc="left",fontweight="bold")
ax.legend(frameon=False,fontsize=9);ax.grid(which="both",alpha=.2);ax.text(.03,.06,"Observed slopes: 2.000, 2.001, 2.000",transform=ax.transAxes,fontsize=9)
fig.savefig(HERE/"accuracy.pdf");fig.savefig(HERE/"accuracy.png",dpi=180);plt.close(fig)
summary={"baseline_commit":"68a28357d960c4c2743c299770f0a33ef63245b2","status":"pass_scoped",
 "structural":{"summary":struct,"order_errors":errors,"order_slopes":slopes},
 "causal":{"grid_cells":len(causal["grid"]),"qualified":sum(x["qualified"]for x in causal["grid"]),"trajectories":causal["trajectories"],"timing":read("causal/TIMING_SUMMARY.json")},
 "homotopy":hom,
 "adapter_initial":read("adapter/SUMMARY.json"),
 "native_tests_initial":33,"global_physical_completion":None,"closed_v6_v8_campaigns_reopened":False}
for key,path in [("adapter_seed","adapter/SEED_SUMMARY.json"),
                 ("adapter_exact_work","adapter/EXACT_WORK_SUMMARY.json"),
                 ("structural_exact_constants","evidence/EXACT_STRUCTURAL_CONSTANTS.json"),
                 ("homotopy_exact_constants","homotopy/EXACT_CONSTANTS_SUMMARY.json"),
                 ("validation","evidence/VALIDATION_SUMMARY.json")]:
 if (ROOT/path).exists():summary[key]=read(path)
(ROOT/"RESULTS.json").write_text(json.dumps(summary,indent=2)+"\n")
lines=[r"\begin{table*}[t]\centering",r"\caption{Qualified complete structural trajectories. Ratios compare the strong derivative-free sequential baseline with the quotient route using the same worker count and original-space screen. Brackets are the minimum/maximum of five paired block ratios, not confidence intervals.}",r"\begin{tabular}{rrll}\toprule",r"$n$ & Workers & Fresh-plan ratio [range] & Reused-plan ratio [range]\\\midrule"]
for n,t in [(64,1),(2048,1),(2048,2)]:
 a=next(x for x in struct if x["n"]==n and x["threads"]==t and x["candidate"]=="quotient-fresh")
 b=next(x for x in struct if x["n"]==n and x["threads"]==t and x["candidate"]=="quotient-reused")
 f=lambda x:f'{x["median_ratio"]:.3f} [{x["range"][0]:.3f}, {x["range"][1]:.3f}]'
 lines.append(f'{n}&{t}&{f(a)}&{f(b)}'+r"\\")
lines += [r"\bottomrule\end{tabular}\label{tab:structural-time}\end{table*}"]
(HERE/"results_table.tex").write_text("\n".join(lines)+"\n")
print(json.dumps({"structural_cells":len(struct),"figures":["performance.pdf","accuracy.pdf"],"source":"actual stored measurements; no new scientific execution"}))

# The exact-constant extension has a different, proof-derived grid. Preserve
# all four seed/acceleration arms so an acceleration claim cannot borrow a
# warm-start saving. Count outward polynomial checks as actual work.
if (ROOT/"homotopy/EXACT_CONSTANTS_RESULTS.json").exists():
 rows=read("homotopy/EXACT_CONSTANTS_RESULTS.json")["rows"]
 fig,axs=plt.subplots(1,2,figsize=(10.8,4.4),layout="constrained")
 eps=sorted({r["physical_tolerance"] for r in rows},reverse=True)
 groups=[next(r for r in rows if r["physical_tolerance"]==e and r["mode"]=="ResidualDominance" and r["warm_previous_stage"]) for e in eps]
 base=[next(r for r in rows if r["physical_tolerance"]==e and r["mode"]=="Picard" and not r["warm_previous_stage"]) for e in eps]
 ax=axs[0];x=np.arange(len(eps));ax.bar(x-.17,[r["total_polynomial_evaluations"] for r in base],.34,color="#b8c4cc",label="Picard / zero seed")
 ax.bar(x+.17,[r["total_polynomial_evaluations"] for r in groups],.34,color="#126782",label="Dominance / warm seed")
 ax.set_xticks(x,[f'$\\epsilon={e:g}$\n$N={r["plan"]["steps"]}$' for e,r in zip(eps,groups)])
 ax.set_ylabel("RHS + outward polynomial evaluations");ax.legend(frameon=False,fontsize=9)
 ax.set_title("(a) Derived grids, counted validation",loc="left",fontweight="bold")
 ax=axs[1]
 ax.loglog(eps,eps,"--",color="#777",label="Requested physical tolerance")
 ax.loglog(eps,[max(p["physical_error_bound"] for p in r["trajectory"]["points"]) for r in groups],"s-",color="#b66b35",label="Computed outward bound")
 ax.loglog(eps,[r["maximum_grid_error"] for r in groups],"o-",color="#126782",label="Observed exact-solution error")
 ax.set_xlabel("Requested global absolute tolerance");ax.set_ylabel("Maximum over the returned grid")
 ax.set_title("(b) Bound and independent reference",loc="left",fontweight="bold")
 ax.grid(which="both",alpha=.2);ax.legend(frameon=False,fontsize=9)
 fig.savefig(HERE/"exact_constants.pdf");fig.savefig(HERE/"exact_constants.png",dpi=180);plt.close(fig)
