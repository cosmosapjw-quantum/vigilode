#!/usr/bin/env python3
"""Summarize actual causal_budget_pilot JSON without rewriting raw evidence."""
import argparse
import json
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument("raw", type=Path)
p.add_argument("output", type=Path)
a = p.parse_args()
r = json.loads(a.raw.read_text())
s, b = r["selected"], r["blanket"]
for x in (s, b):
    assert x["fallback_steps"] == 0
    assert x["steps"] == len(x["steps_detail"])
    assert x["maximum_output_error_upper"] <= x["inner_output_budget"]
    assert x["maximum_embedded_plus_bound"] <= x["embedded_plus_inner_limit"]
out = {
    "schema": "vigilode.causal-budget-summary.v1",
    "raw": str(a.raw),
    "selected": {k: v for k, v in s.items() if k != "steps_detail"},
    "blanket": {k: v for k, v in b.items() if k != "steps_detail"},
    "selected_over_blanket_total_evaluations": s["combined_rhs_and_verifier_evaluations"] / b["combined_rhs_and_verifier_evaluations"],
    "selected_over_native_sequential_evaluations": s["combined_rhs_and_verifier_evaluations"] / s["sequential_native_oracle_rhs"],
    "classification": "specialized_stage_budget_helper; scalar pilot selects direct sequential RK by counted evaluations",
    "wall_speedup_claim": False,
    "global_ode_error_certificate": False,
}
a.output.parent.mkdir(parents=True, exist_ok=True)
with a.output.open("x") as f:
    json.dump(out, f, indent=2)
    f.write("\n")
print(json.dumps({k: out[k] for k in ("selected_over_blanket_total_evaluations", "selected_over_native_sequential_evaluations", "classification")}))
