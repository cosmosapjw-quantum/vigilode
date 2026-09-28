import json, hashlib, os, glob, subprocess
RUN="/home/cosmosapjw/.local/state/vigilode/adversarial-audit/20260927T121000Z"; TREE=RUN+"/tree"; os.chdir(RUN+"/exp/E-04")
def sha(p): return hashlib.sha256(open(p,'rb').read()).hexdigest()
an=json.load(open("analysis.json")); T=an["table"]; V=an["verdict_stats"]
def get(prob,arm): return [r for r in T if r["problem"]==prob and r["arm"]==arm]
def row(prob,arm,k):
    r=[x for x in get(prob,arm) if x["k"]==k]; return r[0] if r else None
def crossover(prob,arm,field="err_max",factor=3.0):
    d={r["k"]:r for r in get(prob,"direct")}
    for r in get(prob,arm):
        dd=d.get(r["k"])
        if dd and r["status"]=="ok" and r[field]>factor*dd[field] and dd[field]>5e-15: return {"k":r["k"],"h":r["h"],"arm_err":r[field],"direct_err":dd[field]}
    return None
def consec(prob,arm,field="err_max"):
    return V.get(f"{prob}|{arm}|{field}_rel_inf",{}).get("consecutive_halvings_slope_ge_4.5_from_start")
def slopes(prob,arm,field="err_max"): return V.get(f"{prob}|{arm}|{field}_rel_inf",{}).get("slopes")
def eta_growth(prob,arm):
    rs=[r for r in get(prob,arm) if r["status"]=="ok" and r["eta_mean"]]
    if len(rs)<2: return None
    import math
    a,b=rs[0],rs[-1]; return {"k_first":a["k"],"eta_first":a["eta_mean"],"k_last":b["k"],"eta_last":b["eta_mean"],"exponent_eta_vs_h":math.log(b["eta_mean"]/a["eta_mean"])/math.log(b["h"]/a["h"])}
def failed_ks(prob,arm): return [r["k"] for r in get(prob,arm) if r["status"]!="ok"]
probs=["p1pr","p1","p2"]; arms=["direct","gmres:1e-12","gmres:1e-9","gmres:1e-6","gmres:1e-3","forcing:1e-4","forcing:1e-6","forcing:1e-8","forcing:1e-10"]
summary={}
for p in probs:
    summary[p]={}
    for a in arms:
        if not get(p,a): continue
        summary[p][a]={"slopes_max":slopes(p,a),"slopes_end":slopes(p,a,"err_end"),"consecutive_halvings_ge4.5_max":consec(p,a),"consecutive_halvings_ge4.5_end":consec(p,a,"err_end"),
                       "floor_max":V[f"{p}|{a}|err_max_rel_inf"]["floor"],"k_at_floor":V[f"{p}|{a}|err_max_rel_inf"]["k_at_floor"],
                       "crossover_vs_direct_3x":crossover(p,a),"failed_k":failed_ks(p,a),"eta_growth":eta_growth(p,a),
                       "errors_max":[(r["k"],r["err_max"]) for r in get(p,a)],"errors_end":[(r["k"],r["err_end"]) for r in get(p,a)]}
    summary[p]["adaptive"]=[{"rtol":r["arm"].split(":")[1],"err_max":r["err_max"],"err_over_rtol":r["err_max"]/float(r["arm"].split(":")[1]),"accepted":r["adaptive_accepted"],"rejected":r["adaptive_rejected"],"h_min":r["adaptive_h_min"],"h_max":r["adaptive_h_max"],"linear_iterations":r["linear_iterations"],"linear_solve_failures":r["linear_solve_failures"],"status":r["status"]} for r in T if r["problem"]==p and r["arm"].startswith("adaptive")]
rustc=subprocess.run(["rustc","--version"],capture_output=True,text=True).stdout.strip()
res={"id":"E-04","purpose":"H2: observed order of RODAS5P under exact (LU) vs inexact (GMRES restart 32, no preconditioner, x0=previous stage) stage solves at fixed eta in {1e-12,1e-9,1e-6,1e-3} and under the production inner-forcing rule (sequential_matrix_free_step_with_inner_forcing, the step the adaptive integrator calls at integrate.rs:774) at fixed h=1/2^k, k=3..10, plus the free adaptive integrator at rtol 1e-4..1e-10. Problems: P1 (n=256 A=Q^T diag(lambda) Q, lambda log-spaced [-1e6,-1], forcing g_i=sin(t+phi_i), y0=g(0)+1: initial transient in all modes), P1PR (same, y0=g(0): pure Prothero-Robinson, added because P1's max-norm is dominated by the h-independent first-step transient term max_z|R(z)-e^z|), P1NS (lambda in [-10,-1], for E-07), P2 (semilinear_advection_diffusion_problem n=512, d=0.02, a=3, r=-1, nu=10, manufactured exact). Norm max_k ||y_k-y*(t_k)||_inf/||y*||_inf (and endpoint).",
 "commands":["build: "+RUN+"/bin/cargo-wrapped.sh build --profile measurement --bin e04_order_krylov  (log build_e04_v2.log; predecessor binary + p1pr variant added, pert amplitude parameter)"]+
   [f"RAYON_NUM_THREADS=1 {RUN}/cargo-target/measurement/e04_order_krylov {os.path.basename(f).split('_')[0]} 3 10 <arm>  > {f}  2> {f[:-6]}.stderr   # arm(s) per file name" for f in sorted(glob.glob("p*.jsonl")) if not f.startswith("smoke")]+
   ["python3 analyze.py > analyze.out   # table.md, analysis.json, e04_<problem>.png","python3 write_results.py"],
 "inputs":[{"path":p,"sha256":sha(p)} for p in [RUN+"/harness/src/bin/e04_order_krylov.rs",RUN+"/cargo-target/measurement/e04_order_krylov",TREE+"/crates/rodas5p-integrators/src/sequential.rs",TREE+"/crates/rodas5p-integrators/src/g4_s5b0_inner_tolerance.rs",TREE+"/crates/rodas5p-integrators/src/problems.rs",TREE+"/crates/rodas5p-integrators/src/integrate.rs"]],
 "outputs":[{"path":os.path.abspath(p),"sha256":sha(p)} for p in sorted(glob.glob("p*.jsonl"))+sorted(glob.glob("p*.stderr"))+["table.md","analysis.json","analyze.out","e04_p1.png","e04_p1pr.png","e04_p2.png","e04_p1ns.png","build_e04_v2.log"] if os.path.exists(p)],
 "env":{"RAYON_NUM_THREADS":"1","profile":"measurement","rustc":rustc,"gmres":"restart 32, maxiter 20000, atol=1e-2*eta, PreconditionerKind::None, InitialGuess::Previous","forcing_arm":"config rtol 1e-10 (overridden per stage by eta), atol=RODAS5P_INNER_FORCING_FLOOR, outer atol=1e-2*rtol, force_accept=true, rejections recorded as would_reject"},
 "summary_metrics":summary,
 "pass_fail":"EVIDENCE_ONLY (H2 SUPPORTED per PLAN rule: eta<=1e-9 and production-rule arms do not reach 4 halvings with slope>=4.5 on any problem; floor/slope collapse and crossover h* quantified)",
 "hypothesis_verdicts":[],"notes":[]}
json.dump(res,open("results.json","w"),indent=1); print(json.dumps({p:{a:(v.get("consecutive_halvings_ge4.5_max"),v.get("crossover_vs_direct_3x"),v.get("failed_k"),(v.get("eta_growth") or {}).get("exponent_eta_vs_h")) for a,v in summary[p].items() if a!="adaptive"} for p in probs},indent=0))
for p in probs: print(p,"adaptive",[(x["rtol"],f'{x["err_over_rtol"]:.2f}',x["accepted"],x["rejected"],x["linear_solve_failures"]) for x in summary[p]["adaptive"]])
