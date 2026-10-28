"""Diagnostic plots only. NO claim of rigorous floating plotting/rounding."""
import json, math
from pathlib import Path
from fractions import Fraction as F
from symbolic_oracle import example, eval_poly
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

c=example(); h=float(c.h); err=float(c.whole_slab_error_upper)
t=[h*k/512 for k in range(513)]
p=[float(eval_poly(c.power,F(k,512)*c.h)) for k in range(513)]
y=[math.exp(-s) for s in t]
threshold=float(c.threshold)
res=[s*s/2 for s in t]
fig,ax=plt.subplots(figsize=(8.0,4.8))
ax.plot(t,y,label="Analytic solution exp(-t)",lw=2)
ax.plot(t,p,label="Represented rational polynomial p(t)",ls="--")
ax.fill_between(t,[a-err for a in p],[a+err for a in p],alpha=.18,label="Whole-slab error envelope ±1/1088")
ax.axhline(threshold,ls=":",label="Event threshold 15/16")
for s,label in [(1/16,"POS"),(1/15,"NEG")]:
    ax.axvline(s,ls=":",alpha=.6)
    ax.text(s,threshold+.004,label,rotation=90,ha="right")
ax.set(xlabel="t (dimensionless toy clock)",ylabel="y and threshold",title="VigilODE-SYM-01: conservative sign/event example")
ax.legend(loc="lower left",fontsize=8)
fig.tight_layout();fig.savefig("event_certificate.png",dpi=180);plt.close(fig)
# Figure audit repair: defect [y/time] and state error [y] must not share an axis.
fig,ax=plt.subplots(figsize=(8,4.5))
ax.plot(t,res,label="Represented residual r(t)=t²/2")
ax.axhline(float(c.defect_upper),ls="--",label="Certified whole-slab defect δ=1/128")
ax.set(xlabel="t (toy clock units)",ylabel="r (state per time)",title="Original-equation defect bound")
ax.legend()
fig.tight_layout();fig.savefig("residual_only.png",dpi=180);plt.close(fig)
fig,ax=plt.subplots(figsize=(8,4.5))
ax.plot(t,[a-b for a,b in zip(p,y)],label="Diagnostic actual error p−exp(-t)")
ax.axhline(err,ls="--",label="Certified whole-slab error cap 1/1088")
ax.set(xlabel="t (toy clock units)",ylabel="state error (state units)",title="Certified trajectory error, distinct from defect")
ax.legend()
fig.tight_layout();fig.savefig("state_error_only.png",dpi=180);plt.close(fig)
manifest={"scope":"SCALAR_AFFINE_TOY_DIAGNOSTIC_PLOTS","event_bracket":["1/16","1/15"],"defect_upper":"1/128","whole_error_upper":"1/1088","float_plot_is_evidence_only":True,"files":["event_certificate.png","residual_only.png","state_error_only.png"],"rejected_plot":"diagnostic_mixed_units_REJECTED.png","plot_max_actual_error":max(abs(a-b) for a,b in zip(p,y)),"actual_endpoint_error":p[-1]-y[-1]}
Path("PLOT_DATA.json").write_text(json.dumps(manifest,indent=2)+"\n")
print(json.dumps(manifest,indent=2))
