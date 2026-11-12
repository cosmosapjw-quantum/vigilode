#!/usr/bin/env python3
"""Validate the VigilODE unified local-agent DAG and evidence contract."""
import json, sys
from pathlib import Path

HERE=Path(__file__).resolve().parent
DAG=HERE/"VIGILODE_UNIFIED_DAG_20261109.json"
MUT=HERE/"MUTATION_GATES_AND_EVIDENCE_CONTRACT.json"

def load(p):
    with p.open(encoding="utf-8") as f: return json.load(f)

def main():
    dag=load(DAG); mut=load(MUT)
    assert dag["schema_version"]=="vigilode.unified-development-dag.v1"
    assert dag["authority"]["g0_authority"]=={"nodes":23,"edges":34,"progress":"7/23","successor_admitted":"0/12","production":"HOLD","policy":"immutable; this DAG is an additive execution overlay only"}
    nodes=dag["nodes"]; ids=[n["id"] for n in nodes]
    assert len(ids)==len(set(ids)), "duplicate task id"
    idset=set(ids)
    for n in nodes:
        for field in ("id","title","status","evidence","depends_on","mutation","source_refs"):
            assert field in n, f"{n.get('id')} missing {field}"
        unknown=set(n["depends_on"])-idset
        assert not unknown, f"{n['id']} has unknown dependencies {sorted(unknown)}"
    edges={(e["from"],e["to"]) for e in dag["edges"]}
    expected={(p,n["id"]) for n in nodes for p in n["depends_on"]}
    assert edges==expected, "edge list and node dependencies differ"
    # Kahn topological check
    incoming={x:0 for x in ids}; outgoing={x:[] for x in ids}
    for a,b in edges: incoming[b]+=1; outgoing[a].append(b)
    ready=[x for x,d in incoming.items() if d==0]; seen=[]
    while ready:
        x=ready.pop(); seen.append(x)
        for y in outgoing[x]:
            incoming[y]-=1
            if incoming[y]==0: ready.append(y)
    assert len(seen)==len(ids), "cycle detected"
    required={"T02_CURRENT_GLOBAL_ERROR_AUTHORITY_REPLAY","PW01_PTSW2C_ORDER3_FIRST_PEER_IMPLEMENTATION","PW02_PTSW2B_LSTABLE_ORDER2_CONTROL","PW03_G0_ORDER5_PEER_W_TABLEAU_SOURCE_RECOVERY","PW04_PEER_HISTORY_STARTUP_ERROR_AND_SESSION","PW05_RHS_ONLY_IMPLICIT_PEER_HOMOTOPY","PW06_PEER_MATCHED_FULL_IVP_AND_G0_GATE","PW07_COMPOSITIONAL_ERROR_CERTIFICATE_AND_MOCK_EXPERIMENT","T09_MATCHED_FULL_IVP_BENCHMARK_AND_RELEASE_REVIEW"}
    assert required.issubset(idset), f"missing required nodes {required-idset}"
    assert dag["authority"]["g0_authority"]["production"]=="HOLD"
    assert dag["validation"]["G0_mutation"] is False
    assert dag["validation"]["code_source_mutation"] is False
    assert mut["default_mutation_mode"]=="DOCS_ONLY_ON_DEDICATED_BRANCH"
    assert "NOT_RUN" in mut["evidence_status"]
    print(f"PASS: {len(nodes)} tasks, {len(edges)} edges, acyclic DAG; G0 immutable; mutation/evidence contract valid")
    print("NOTE: this validates package structure only; it does not run Rust/Cargo or certify scientific claims.")

if __name__=="__main__":
    try: main()
    except Exception as exc:
        print(f"FAIL: {exc}",file=sys.stderr); sys.exit(1)
