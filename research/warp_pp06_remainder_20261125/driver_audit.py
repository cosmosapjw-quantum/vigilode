"""Reuse the archived PP06 independent reference check, NOT its historical G3 gate."""
import importlib.util,json,functools
from pathlib import Path
root=Path(__file__).resolve().parent
repo=root.parent.parent
s=importlib.util.spec_from_file_location("pp06",repo/"tools/pp06_fft_check.py")
pp06=importlib.util.module_from_spec(s)
s.loader.exec_module(pp06)
# Avoid recomputing a reference when both candidate representations have the same times.
def cached(fn):
    wrapped=functools.lru_cache(maxsize=16)(fn)
    return lambda omega,sigma,times:wrapped(omega,sigma,tuple(times))
pp06.pp05.reference_mp=cached(pp06.pp05.reference_mp)
pp06.pp05.reference_scipy=cached(pp06.pp05.reference_scipy)
rows=[]
for line in (root/"regression.log").read_text().splitlines():
    if "DRIVER_CASE " not in line:continue
    c=json.JSONDecoder().raw_decode(line.split("DRIVER_CASE ",1)[1])[0]
    r,ok=pp06.check_run(c)
    r.update({"mode":c["mode"],"omega":pp06.pp05.unhex(c["omega"]),"reference_check":bool(ok)})
    rows.append(r)
    print("DRIVER_AUDIT",json.dumps(r,sort_keys=True),flush=True)
assert len(rows)==4
result={"schema":"warp.pp06.driver.v1","rows":rows,"all_reference_checks_pass":all(r["reference_check"] for r in rows)}
(root/"DRIVER_AUDIT.json").write_text(json.dumps(result,indent=2,sort_keys=True)+"\n")
assert result["all_reference_checks_pass"]
