from pathlib import Path
import json, math
import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

HERE=Path(__file__).resolve().parent
rows=json.loads((HERE/"PLOT_DATA.json").read_text())
native=[r for r in rows if r["origin"]=="NativeAcceptedRodas5pFast"]
t=np.array([r["time"] for r in native])
err=np.array([float(r["analytic_error"]) for r in native])
bound=np.array([r["error_upper"] for r in native])
rounding=np.array([r["rounding_upper"] for r in native])
center=np.array([r["center"] for r in native])
threshold=15/16
plt.rcParams.update({"font.size":10,"axes.spines.top":False,"axes.spines.right":False})
fig,ax=plt.subplots(1,2,figsize=(11,4.8),layout="constrained")
for value,label,color,marker in [(bound,"Certified query error upper","#2563eb","o"),(err,"Analytic solution discrepancy (Decimal90)","#dc2626","x"),(rounding,"Pointwise evaluation rounding upper","#64748b","+")]:
    keep=value>0
    ax[0].scatter(t[keep],value[keep],label=label,color=color,marker=marker,s=27)
ax[0].set(yscale="log",xlabel="Physical time",ylabel="Absolute state error",title="A  Native affine error enclosure")
ax[0].legend(fontsize=8,loc="upper center",bbox_to_anchor=(0.5,-0.22))
ax[0].grid(alpha=.2,which="both")
fine=np.linspace(0,.125,201)
ax[1].plot(fine,np.exp(-fine)-threshold,color="#dc2626",lw=1.5,label="Analytic event value (diagnostic)")
ax[1].errorbar(t,center-threshold,yerr=bound,fmt="o",ms=3.5,capsize=2,color="#2563eb",label="Native center ± certified query error")
ax[1].axhline(0,color="#111827",lw=.8)
cross=-math.log(threshold)
ax[1].axvline(cross,color="#64748b",ls=":",lw=1,label="Analytic crossing (diagnostic)")
ax[1].set(xlabel="Physical time",ylabel="State minus 15/16",title="B  Threshold crossing on two accepted slabs")
ax[1].legend(fontsize=8,loc="upper right")
ax[1].grid(alpha=.2)
fig.suptitle("VigilODE SYM02 · source-bound scalar affine native fixture",fontsize=13)
fig.savefig(HERE/"VALIDATION.png",dpi=180)
fig.savefig(HERE/"VALIDATION.pdf")
report={"scope":"Two native accepted scalar-affine segments of y'=-y; 14 query records","native_records":len(native),"analytic_curve":"diagnostic; continuous proof is Bernstein/Dini in THEORY.md","zero_values_omitted_only_on_log_axis":int(np.sum(bound==0)+np.sum(err==0)+np.sum(rounding==0)),"all_raw_records_retained":len(rows),"artifact":["VALIDATION.png","VALIDATION.pdf"]}
(HERE/"PLOT_INFO.json").write_text(json.dumps(report,indent=2)+"\n")
print(json.dumps(report))
