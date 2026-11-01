#!/usr/bin/env python3
"""VigilODE standalone mission/DAG reconciliation, validation and deterministic overlay.

No solver, original 23-node DAG, ledger, source, tolerance or CI gate changes.
Command: python3 tools/validate_vigilode_core_realign.py --write / --check
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
D=ROOT/"docs/vigilode_original_mission_realign_20261101"
SOURCE=ROOT/"research/polyalgorithm_alternatives_20261028/LOCAL_DEVELOPMENT_PLAN.json"
OVERLAY=D/"REFOCUSED_DAG.json"
CATALOG=D/"INTEGRATOR_INVENTORY.json"
SOURCE_SHA256="8a05bdf42d999f724662d64b5fc9a36bc5bb0caa3a9f5022b7333dcc914bc3f1"
BASE_COMPLETED={"S-PRS","R-STRUCT","R-HOMO","R-ADAPT","I-PROD","V-CLOUD","P-PUSH"}
RULES={
 "L-HARDWARE":{"start":["P-PUSH"],"priority":"P0","role":"scoped_matched_error_research"},
 "PR-N01":{"start":["R-STRUCT","R-HOMO"],"priority":"P1","role":"operator_cost_and_fused_phi"},
 "PR-N02":{"start":["R-HOMO"],"priority":"P2","role":"finite_step_nonlin_bounds"},
 "PR-N03":{"start":["R-STRUCT"],"priority":"P2","role":"physical_error_structural_adaptive"},
 "PR-N04":{"start":["R-HOMO","R-ADAPT"],"priority":"P0","role":"owned_original_stage_session"},
 "PR-N05":{"start":["R-STRUCT","R-ADAPT"],"priority":"P0","role":"causal_depth_and_parallel_span"},
 "PR-N06":{"start":["R-ADAPT"],"priority":"P1","role":"reentrant_regime_switch_cost"},
 "Q-GIVENS":{"start":["P-PUSH"],"priority":"P1","role":"portable_true_residual_repair"},
 "Q-FAIR-AB":{"start":["P-PUSH"],"priority":"P1","role":"complete_attempted_work"},
 "Q-NEGATIVE":{"start":["P-PUSH"],"priority":"P1","role":"negative_control_unchanged"},
 "Q-LINT":{"start":["P-PUSH"],"priority":"P2","role":"lint_before_promotion"},
 "Q-API":{"start":["P-PUSH"],"priority":"P2","role":"public_api_compatibility"},
 "Q-BENCH":{"start":["P-PUSH"],"priority":"P2","role":"external_comparator_qualification"},
 "Q-LEDGER":{"start":["P-PUSH"],"priority":"P2","role":"global_historical_authority_debt"},
 "L-RELEASE":{"start":["P-PUSH","Q-GIVENS","Q-FAIR-AB","Q-LINT","Q-NEGATIVE"],"priority":"P3","role":"strict_release_qualification"},
 "R-QUALIFY":{"start":["L-RELEASE","L-HARDWARE","Q-LEDGER","Q-BENCH","Q-API"],"priority":"P3","role":"final_scoped_production_decision"},
}
NEXT={
 "L-HARDWARE":"One executable same-machine matched-achieved-global-error runner: protected R-JF, fused pexprb54s4 and RHS-only DOPRI5 on 4 frozen generic IVPs; report complete callbacks/work/time and regimes.",
 "PR-N04":"Use existing native owned sessions; implement/test exact original U-stage clock/epoch/retained controller transfer and bounded failure on full generic IVP.",
 "PR-N05":"Measure genuine 8-stage serial critical path vs q1/q2 and causal prefix on same original stage target, then full IVP at original tolerances; certify residual and charged failures.",
 "PR-N01":"Reuse fused phi-Chebyshev/Laguerre/Leja/Krylov code, select one source-independent operator-space cost comparison and build actual same-IVP work-precision table.",
 "PR-N02":"Extend finite-step cubic proof to stated class, test original physical cap with exact arithmetic; no solver speed claim without actual native run.",
 "PR-N03":"Reconnect full-state residual source ownership to adaptive result, retain negative nonlinear source and all rejected attempts.",
 "PR-N06":"Use real changed-regime and reentrant native IVPs to test residence policy against RHS-only and guarded opt-in, including actual rejected work.",
 "Q-LEDGER":"Recover exact original historical Git objects and preserve missing post-import CI receipts; global authority strict gate only for final release, not experimental math.",
 "Q-GIVENS":"Preserve failing historical true residual fixture and repair portable Givens acceptance independently.",
 "Q-FAIR-AB":"Prove and test total attempted + rejected + diagnostic work with original counters.",
 "Q-LINT":"Repair confirmed source-touched lints in isolated branch; never mass suppress.",
 "Q-BENCH":"Build version-locked external comparator in one verified local toolchain, no benchmark claims until source identities match.",
 "Q-API":"Exercise original public API compatibility against exact base before any ABI migration.",
 "Q-NEGATIVE":"Run original ignored tests and preserve 15/7 > 1.3 negative performance and source/epoch counterexamples.",
 "L-RELEASE":"Run original strict release/test/toolchain contracts after Q blockers; do not mark PASS based on scoped native compile.",
 "R-QUALIFY":"After full authority, fixed tolerance and matched-error receipt, evaluate scoped release; no automatic merge or default switch.",
}
GENERIC_CASES=["stiff_nonlinear_robertson","oscillatory_prothero_robinson","nonnormal_linear_or_nonlinear","changing_stiffness_reentrant"]
MILESTONES=[
 {"id":"M1","title":"Same-runner achieved-error work precision","status":"READY_FOR_SCOPED_EXECUTION","linked_nodes":["L-HARDWARE","PR-N01"],"artifact":"same source/IC/BC/output grid, independent high-accuracy reference, confidence intervals and full JVP/RHS/alloc/failed costs","stop":"No error-certified, source-matched common-runner measurement: no speedup claim"},
 {"id":"M2","title":"Original eight-stage causal depth demonstrator","status":"READY_FOR_SCOPED_EXECUTION","linked_nodes":["PR-N04","PR-N05"],"artifact":"full original U-stage residual, causal q1/q2/Pexprb prefix, exact attempted-work, stage critical depth and order>=4.8 where appropriate","stop":"Any failure in raw original target or endpoint error: reject candidate/fallback without retuning tolerances"},
 {"id":"M3","title":"Scientific polyalgorithm decision","status":"BLOCKED_ON_M1_M2","linked_nodes":["PR-N06","R-ADAPT"],"artifact":"frozen regime map, search overhead, adverse reentrant examples, honest transaction and epoch invalidation","stop":"No matched error or measured benefit: keep shadow and protected routing"},
 {"id":"M4","title":"Scoped production promotion","status":"BLOCKED_ON_EXISTING_Q_AND_RELEASE","linked_nodes":["L-RELEASE","R-QUALIFY"],"artifact":"strict code/CI/authority and original physical/accuracy/work budgets with distinct-author review","stop":"Any unsatisfied release/authority gate: HOLD"},
]


def sha(path:Path):
 return hashlib.sha256(path.read_bytes()).hexdigest()


def ensure_sorted_dag(nodes, depsfield):
 index={row["id"]:i for i,row in enumerate(nodes)}
 assert len(index)==len(nodes)
 for row in nodes:
  for dep in row[depsfield]:
   assert dep in index, (row["id"],dep)
   # Some historical plans place research-start deps after the node;
   # check acyclicity, not original topological file position.
 edges={r["id"]:set(r[depsfield]) for r in nodes}
 visiting,seen=set(),set()
 def walk(n):
  if n in visiting: raise ValueError("cycle: "+n)
  if n in seen:return
  visiting.add(n)
  for dep in edges[n]:walk(dep)
  visiting.remove(n);seen.add(n)
 for name in edges:walk(name)


def make():
 assert sha(SOURCE)==SOURCE_SHA256, "ORIGINAL_23_NODE_PLAN_CHANGED"
 p=json.loads(SOURCE.read_text())
 assert len(p["nodes"])==23 and {r["id"] for r in p["nodes"] if r.get("status")=="complete_scoped"}==BASE_COMPLETED
 assert len([r for r in p["nodes"] if r.get("status")=="pending_local"])==15
 assert len([r for r in p["nodes"] if r.get("status")=="registration_repaired_full_authority_pending"])==1
 entry=[]
 for row in p["nodes"]:
  id=row["id"];rule=RULES.get(id,{})
  entry.append({
   "id":id,"title":row["title"],"historical_status":row["status"],
   "historical_release_dependencies":row["dependencies"],
   "research_start_dependencies":rule.get("start",row["dependencies"]),
   "lane":rule.get("role","PRESERVE_CLOSED_SCOPED_NO_REPLAY"),
   "priority":rule.get("priority","CLOSED"),
   "next_executable_product":NEXT.get(id,"Reuse existing closed evidence without new research gate."),
   "research_start_entitlement":"scoped_only" if row["status"]!="complete_scoped" else "completed_historical",
   "may_promote_production":False,
  })
 ensure_sorted_dag(entry,"historical_release_dependencies")
 ensure_sorted_dag(entry,"research_start_dependencies")
 # Original release dependencies are intentionally unmodified; experimental
 # research edges are independently acyclic with their own typed permission.
 assert [e["id"] for e in entry]==[r["id"] for r in p["nodes"]]
 result={
  "schema":"vigilode.original_g0_science_first_reconciled_dag.v1",
  "type":"NONCANONICAL_EXECUTION_OVERLAY_WITH_LOCKED_HISTORICAL_NODES",
  "source_plan":"research/polyalgorithm_alternatives_20261028/LOCAL_DEVELOPMENT_PLAN.json",
  "source_plan_sha256":SOURCE_SHA256,
  "original_23_nodes_unchanged":True,
  "historical_progress":{"closed_scoped":7,"total":23,"pending_local":15,"authority_pending":1,"scope":"23 node LOCAL_DEVELOPMENT_PLAN; not all scientific achievement"},
  "official_progress_claim":"7/23_UNCHANGED",
  "historical_successor_claim":"0/12_UNCHANGED",
  "maturity_dimensions":["scientific_derivation","runnable_source","source_bound_ivp","matched_error_validation","independent_review","production_gate"],
  "permission_rule":"research_start_dependencies permit only isolated exploratory science, never production; historical_release_dependencies remain strictly necessary for production qualification",
  "source_scope":"VigilODE standalone generic stiff and oscillatory ODE numerical algorithm",
  "original_g0":{"protected_baseline":"RJF_SEQ","preferred_fast_lane":"causal parallel derivative/matrix/Newton-free; selective JVP/AD alternative","explicit_jacobian_builds_fast_lane":0,"newton_iterations_fast_lane":0,"critical_depth_max":8,"critical_depth_p95_target":6,"global_order_min":4.8,"same_error_median_speedup_min":1.15,"relative_accepted_steps_max":1.2,"fallback_fraction_max":0.10,"comparison":"full generic IVPs on same input/reference/grid/hardware including all refused work"},
  "no_default_change":True,"no_tolerance_relaxation":True,"no_auto_next_node":True,"no_merge":True,"production":"HOLD",
  "no_retroactive_gate_change":True,
  "generic_holdout_cases":GENERIC_CASES,
  "core_milestones":MILESTONES,
  "nodes":entry,
 }
 return result


def validate_catalog():
 inv=json.loads(CATALOG.read_text())
 assert len(inv["rows"])>=20
 ids=set()
 for row in inv["rows"]:
  assert row["id"] not in ids
  ids.add(row["id"])
  p=ROOT/row["source_path"]
  assert p.is_file(),("SOURCE_NOT_FOUND",row["id"],str(p))
  if row["symbol"]:
   assert row["symbol"] in p.read_text(),("SYMBOL_MISSING",row["id"],row["symbol"])
  assert row["science_admission"]=="not_promoted_by_this_audit"
  assert row["non_promotion_limit"]
 assert "RJF_SEQ" in ids and "PEXPRB54S4_FUSED" in ids and "HOMOTOPY_RHS_ONLY" in ids
 # BDF/Radau frozen scientific reference status cannot accidentally become
 # competitive production credentials in an inventory report.
 for id in ("BDF1_BDF2","RADAU_IIA1_IIA3"):
  row=next(r for r in inv["rows"] if r["id"]==id)
  assert row["classification"]=="reference-only"
 return len(ids)


def main():
 ap=argparse.ArgumentParser()
 g=ap.add_mutually_exclusive_group(required=True)
 g.add_argument("--write",action="store_true")
 g.add_argument("--check",action="store_true")
 args=ap.parse_args()
 result=make()
 n=validate_catalog()
 generated=json.dumps(result,indent=2,ensure_ascii=False,sort_keys=True)+"\n"
 if args.write: OVERLAY.write_text(generated)
 assert OVERLAY.read_text()==generated,"DAG_CONTENT_DRIFT_OR_MISSING"
 assert len(result["nodes"])==23
 assert result["historical_progress"]["closed_scoped"]==7
 assert result["production"]=="HOLD"
 assert result["no_tolerance_relaxation"] and result["no_merge"]
 print("VIGILODE_CORE_DAG_REALIGN_VALID",len(result["nodes"]),"official 7/23 unchanged; inventory",n,"entries; research/release acyclic; source SHA verified")


if __name__=="__main__":
 main()
