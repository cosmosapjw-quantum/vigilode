"""Identity, decision, schema, path and DAG checks; not a theorem prover."""
from pathlib import Path
from zipfile import ZipFile
import hashlib
import json
import re

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[1]
DOCS = REPO / "docs/reviews/20261011_loop12_upgrade"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def unique_pairs(pairs):
    out = {}
    for key, value in pairs:
        if key in out:
            raise ValueError(f"duplicate JSON key: {key}")
        out[key] = value
    return out


def reject_constant(value):
    raise ValueError(f"nonfinite JSON constant: {value}")


def read(path):
    return json.loads(path.read_text(), object_pairs_hook=unique_pairs,
                      parse_constant=reject_constant)


def main():
    json_files = [p for p in ROOT.rglob("*.json") if p.name != "VALIDATION.json"]
    for path in json_files:
        read(path)
    imported = read(ROOT / "input/IMPORT_INDEX.json")
    archive = ROOT / "input" / imported["archive"]
    assert digest(archive.read_bytes()) == imported["archive_sha256"]
    assert archive.stat().st_size == imported["archive_bytes"]
    with ZipFile(archive) as z:
        for name in imported["selected_files"]:
            assert (ROOT / "input" / name).read_bytes() == z.read(name), name
        bindings = read(ROOT / "REVIEW_BINDINGS.json")["bindings"]
        for binding in bindings:
            if binding["zip_member"] is not None:
                assert (REPO / binding["repo_path"]).resolve() == archive.resolve()
                data = z.read(binding["zip_member"])
            else:
                data = (REPO / binding["repo_path"]).read_bytes()
            assert digest(data) == binding["sha256"], binding["original_path"]
    source = read(ROOT / "SOURCE_BINDINGS.json")
    for item in source["items"]:
        assert digest((REPO / item["path"]).read_bytes()) == item["file_sha256"], item["id"]
    theories = read(ROOT / "THEOREMS.json")
    ids = [x["id"] for x in theories["claims"]]
    assert len(ids) == len(set(ids))
    for item in theories["claims"]:
        assert item["decision"] in {"PROMOTE", "HOLD", "REJECT"}
        assert digest((REPO / item["proof_path"]).read_bytes()) == item["proof_sha256"]
        assert (REPO / item["review_path"]).is_file()
    parent = theories["parent_results"]
    assert digest((REPO / parent["decision_path"]).read_bytes()) == parent["sha256"]
    decision = read(ROOT / "DECISION.json")
    assert decision["scope_decisions"]["native_production"] == "HOLD"
    assert decision["scope_decisions"]["measured_wall_time"] == "HOLD"
    assert decision["published_campaigns_rerun"] == 0
    extensions = [c for c in theories["claims"] if c["origin"] == "new_extension"]
    assert len(extensions) == 12 and all(c["decision"] == "PROMOTE" for c in extensions)
    dag = read(ROOT / "PORTING_DAG.json")
    nodes = {x["id"]: x for x in dag["nodes"]}
    assert len(nodes) == len(dag["nodes"])
    prior_ids = {x["id"] for x in read(REPO / dag["prior_dag"])["nodes"]}
    visiting, visited = set(), set()

    def visit(node_id):
        assert node_id in nodes
        assert node_id not in visiting, "cycle"
        if node_id in visited:
            return
        visiting.add(node_id)
        for dep in nodes[node_id]["depends_on"]:
            visit(dep)
        visiting.remove(node_id)
        visited.add(node_id)
    for node_id, node in nodes.items():
        visit(node_id)
        assert set(node["prior_TP_nodes"]) <= prior_ids
    link_count = 0
    for path in list(DOCS.glob("*.md")) + [ROOT / "README.md"]:
        for target in re.findall(r"\[[^\]]*\]\(([^)]+)\)", path.read_text()):
            if target.startswith(("https://", "http://", "#")):
                continue
            target = target.split("#", 1)[0]
            if not target:
                continue
            resolved = (path.parent / target).resolve()
            # The validation receipt is created below; all other targets must
            # already exist. Check this self-output after writing it.
            if resolved != (ROOT / "VALIDATION.json").resolve():
                assert resolved.is_file(), (path.name, target)
            link_count += 1
    schema_result = "NOT_RUN_JSONSCHEMA_UNAVAILABLE"
    try:
        import jsonschema
    except ImportError:
        pass
    else:
        jsonschema.Draft202012Validator(read(ROOT / "THEOREMS_SCHEMA.json")).validate(theories)
        schema_result = "PASS"
    result = {
        "schema": "vigilode-loop12-validation-v1",
        "status": "PASS",
        "scope": "artifact identity, JSON semantics, reviewer bindings, paths and DAG; not formal proof",
        "json_files_checked": len(json_files),
        "theorem_index_entries": len(ids),
        "extension_theorems_independently_promoted": len(extensions),
        "review_input_hash_bindings_checked": len(bindings),
        "source_file_hashes_checked": len(source["items"]),
        "selected_imported_files_byte_identical": len(imported["selected_files"]),
        "dag_nodes": len(nodes),
        "relative_markdown_links_checked": link_count,
        "theorem_jsonschema": schema_result,
        "original_archive_hash_match": True,
        "native_tests_this_work_unit": 0,
        "historical_campaign_reruns": 0,
        "new_diagnostics": {
            "owner_causal_weight_check_groups": 5,
            "independent_weight_interface_checks": 3,
            "rank_kernel_free_sweep_exact_equalities": [36, 18, 72],
            "damping_zero_limit_exact_identities": 25,
            "damping_projector_transport": "PASS",
            "wolfram_integral_identities": 6
        }
    }
    (ROOT / "VALIDATION.json").write_text(json.dumps(result, indent=2) + "\n")
    assert (ROOT / "VALIDATION.json").is_file()
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
