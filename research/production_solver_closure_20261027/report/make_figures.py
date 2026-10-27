#!/usr/bin/env python3
"""Render measured block ranges from retained JSON; no synthetic samples."""
import json
from pathlib import Path
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

node = Path(__file__).resolve().parents[1]
read = lambda name: json.loads((node/name).read_text())
structural = read("structural/FINAL_BENCHMARK_SUMMARY.json")
homotopy = read("homotopy/ANALYSIS.json")
adapter = read("adapter/PERFORMANCE.json")
rows = []
for dimension,threads in ((1,1),(64,1),(2048,1),(2048,2)):
    row = next(r for r in structural["rows"] if (r["dimension"],r["threads"]) == (dimension,threads))
    result = next(r for r in row["comparisons"] if r["candidate"] == "enclosed")
    rows.append((f"Structural n={dimension}, workers={threads}",result["median_speedup"],*result["observed_range"],"#1f697c"))
for first,second in (("generic","scalar"),("generic","rational"),("scalar","rational")):
    row = next(r for r in homotopy["pass1_timing"] if (r["case"],r["a"],r["b"]) == ("nonlinear",first,second))
    rows.append((f"Cubic nonlinear: {first} / {second}",row["median_a_over_b"],row["minimum"],row["maximum"],"#4d7858"))
for row in adapter["cases"]:
    rows.append(("Adapter: "+row["case"],row["median_speedup"],*row["observed_speedup_range"],"#a04e38"))
plt.rcParams.update({"font.size":10,"axes.spines.top":False,"axes.spines.right":False})
fig,ax=plt.subplots(figsize=(10,6),layout="constrained")
for i,(label,median,low,high,color) in enumerate(rows):
    ax.errorbar(median,i,xerr=[[median-low],[high-median]],fmt="o",color=color,capsize=3,lw=1.6)
    ax.text(high+0.07,i,f"{median:.3f}",va="center",fontsize=9,color=color)
ax.set_yticks(range(len(rows)),[r[0] for r in rows]);ax.invert_yaxis()
ax.axvline(1,color="#555555",ls="--",lw=1);ax.set_xlim(0,4.1)
ax.set_xlabel("Baseline time / candidate time; median and observed five-block range")
ax.set_title("Complete qualified trajectories: improvements and adverse controls",loc="left",pad=14)
ax.grid(axis="x",alpha=.2)
fig.savefig(node/"report/performance.pdf");fig.savefig(node/"report/performance.png",dpi=170)
plt.close(fig)

profiles=[("Structural: enclosed / PR91",read("structural/instruction_profile/SUMMARY.json")["ratio_to_first_arm"]["enclosed"]),
          ("Cubic: scalar / generic",read("homotopy/instruction_profile_r1/SUMMARY.json")["ratio_to_first_arm"]["scalar"]),
          ("Cubic: rational / generic",read("homotopy/instruction_profile_r1/SUMMARY.json")["ratio_to_first_arm"]["rational"])]
for row in adapter["cases"]:
    slopes=row["instruction_slopes_per_trajectory"]
    profiles.append(("Adapter: "+row["case"],slopes[row["candidate"]]/slopes[row["baseline"]]))
fig,ax=plt.subplots(figsize=(9,3.6),layout="constrained")
values=[r[1] for r in profiles]
bars=ax.barh([r[0] for r in profiles],values,color=["#1f697c" if v<1 else "#a04e38" for v in values])
ax.bar_label(bars,labels=[f"{v:.4f}" for v in values],padding=4);ax.invert_yaxis()
ax.axvline(1,color="#555555",ls="--",lw=1);ax.set_xlim(0,2.85)
ax.set_xlabel("Guest instructions per trajectory / baseline; two-count startup subtraction")
ax.set_title("Instruction profiles are distinct from native wall time",loc="left",pad=14)
fig.savefig(node/"report/instructions.pdf");fig.savefig(node/"report/instructions.png",dpi=170)
plt.close(fig)
