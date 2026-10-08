"""B6: h0-seed robustness of the frozen detector (h0 x {0.3, 0.5, 2, 3}) on the 18 corpus rows and the
robertson / hires / bruss50 benchmark rows.  EXPLORATORY."""
import os, sys, json, glob
import numpy as np
HERE = os.path.dirname(os.path.abspath(__file__))
SUB = sys.argv[1] if len(sys.argv) > 1 else "rows_seed"
rows = [json.load(open(f)) for f in glob.glob(os.path.join(HERE, SUB, "*.json"))]
out = []
for d in rows:
    tr = d["truth"]; de = d["detector"]
    exceed = d["grid"]["max_grid_case"] > 1.0 if d["kind"] == "corpus" else tr["G_max_nodes"] > 1.0
    out.append({"key": d["key"], "R_tot": tr["R_tot"], "A5": de["res5"]["A_max"], "AR": de["exact"]["A_max"],
                "A5mf4": de["res5_mf4"]["A_max"], "ARmf": de["exact_mf"]["A_max"], "A5mf": de["res5_mf"]["A_max"], "Rrun": tr["R_run_max"], "guard5": de["res5"]["guard_value"], "F5": de["res5"]["FLAG"], "FR": de["exact"]["FLAG"],
                "amp": tr["R_tot"] > 4, "exceeds": bool(exceed), "grid": d["grid"]["max_grid_case"],
                "rival_flag": d["rival"]["FLAG_exceed"], "att": d["counters"]["attempts"]})
out.sort(key=lambda r: r["key"])
for r in out:
    print(f"{r['key']:30s} att {r['att']:4d} grid {r['grid']:8.3g} R_tot {r['R_tot']:7.3g} A5 {r['A5']:7.3g} AR {r['AR']:7.3g} A5mf4 {r['A5mf4']:7.3g} "
          f"F5 {int(r['F5'])} FR {int(r['FR'])} amp {int(r['amp'])} exc {int(r['exceeds'])} rival {int(r['rival_flag'])}")
def cm(flag, truth):
    return [sum(1 for r in out if r[flag] and r[truth]), sum(1 for r in out if r[flag] and not r[truth]),
            sum(1 for r in out if not r[flag] and r[truth]), sum(1 for r in out if not r[flag] and not r[truth])]
res = {"n_rows": len(out), "confusion": {f"{f}|{t}": cm(f, t) for f in ("F5", "FR", "rival_flag") for t in ("amp", "exceeds")}}
neg = [r for r in out if not r["amp"]]; pos = [r for r in out if r["amp"]]
res["margins"] = {k: {"neg_max": max(r[k] for r in neg), "neg_argmax": max((r[k], r["key"]) for r in neg)[1],
                      "pos_min": min(r[k] for r in pos) if pos else None,
                      "pos_argmin": min((r[k], r["key"]) for r in pos)[1] if pos else None} for k in ("A5", "AR", "A5mf4")}
res["kills"] = {"K1": [r["key"] for r in out if r["R_tot"] > 5 and min(r["A5"], r["AR"]) < 3],
                "K2": [r["key"] for r in out if r["R_tot"] < 2 and max(r["A5"], r["AR"]) > 4],
                "K3": [r["key"] for r in out if r["F5"] != r["FR"]]}
res["rows"] = out
print(json.dumps({k: v for k, v in res.items() if k != "rows"}, indent=1))
json.dump(res, open(os.path.join(HERE, "summary_" + SUB + ".json"), "w"), indent=1)
