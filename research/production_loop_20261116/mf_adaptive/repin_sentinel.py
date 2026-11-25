#!/usr/bin/env python3
"""Loop node V24: sets the pin of src/rodas5p_matrix_free_fast.rs in the
sentinel g3 (crates/rodas5p-integrators/tests/ppsw1_6b_contracts.rs) to the
hash of the file as it is, and writes SENTINEL_REPIN.json with the old and
the new pins. No hash is typed in: every value is computed here.

usage: python3 -I repin_sentinel.py [--base COMMIT]
"""
import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import time

sys.dont_write_bytecode = True
NODE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(NODE, "..", "..", ".."))
CRATE = os.path.join(ROOT, "crates", "rodas5p-integrators")
DRIVER = "src/rodas5p_matrix_free_fast.rs"
G3 = "tests/ppsw1_6b_contracts.rs"
H2 = "tests/ptsw2b_contracts.rs"
BLOCK_HEAD = "// Loop L1 node V24 (owner decision OD-L1-11): policy of the matrix-free stage solve."


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def read(rel):
    with open(os.path.join(CRATE, rel), "rb") as handle:
        return handle.read()


def at_base(base, rel):
    return subprocess.run(["git", "-C", ROOT, "show", f"{base}:crates/rodas5p-integrators/{rel}"],
                          capture_output=True, check=True).stdout


def pin_pattern(path):
    return re.compile(r'(\(\s*"' + re.escape(path) + r'",\s*")([0-9a-f]{64})(",\s*\))')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", default="892132e5d879b5d8c2740ef7a73946fd2b1d25f9")
    args = parser.parse_args()

    driver_new = sha256(read(DRIVER))
    driver_old = sha256(at_base(args.base, DRIVER))
    g3_text = read(G3).decode("utf-8")
    start = g3_text.index("fn g3_protected_sources_are_byte_identical()")
    end = g3_text.index("\n}\n", start)
    body = g3_text[start:end]
    match = pin_pattern(DRIVER).search(body)
    if match is None:
        sys.exit("the pin of the driver was not found in g3")
    pin_before_this_run = match.group(2)
    body = pin_pattern(DRIVER).sub(lambda m: m.group(1) + driver_new + m.group(3), body, count=1)
    g3_new_text = g3_text[:start] + body + g3_text[end:]
    if g3_new_text != g3_text:
        with open(os.path.join(CRATE, G3), "w", encoding="utf-8") as handle:
            handle.write(g3_new_text)

    base_g3 = at_base(args.base, G3).decode("utf-8")
    base_pin = pin_pattern(DRIVER).search(base_g3[base_g3.index("fn g3_protected_sources_are_byte_identical()"):]).group(2)
    lib_now = read("src/lib.rs").decode("utf-8")
    block_start = lib_now.index("\n" + BLOCK_HEAD)
    block_end = lib_now.index("};\n", block_start) + len("};\n")
    block = lib_now[block_start:block_end]

    # The second sentinel that pins the same file. It is outside the write
    # list of the node: nothing is written to it; the values the orchestrator
    # would need are computed in memory.
    h2_now = read(H2)
    h2_text = h2_now.decode("utf-8")
    h2_match = pin_pattern(DRIVER).search(h2_text)
    h2_proposed = pin_pattern(DRIVER).sub(lambda m: m.group(1) + driver_new + m.group(3), h2_text, count=1).encode("utf-8")
    g3_h2_pin = pin_pattern(H2).search(g3_new_text[start:]).group(2)

    record = {
        "schema": "vigilode.loop-l1.sentinel-repin.v1",
        "node": "V24_MF_ADAPTIVE_STAGE_SOLVE",
        "date_kst": time.strftime("%Y-%m-%d"),
        "authority": "owner decision OD-L1-11 (the protected matrix-free driver is edited; the sentinel is re-pinned for it and the old and new pins are recorded)",
        "sentinel": "crates/rodas5p-integrators/tests/ppsw1_6b_contracts.rs, test g3_protected_sources_are_byte_identical",
        "what_changed": "the pin of src/rodas5p_matrix_free_fast.rs, and one more allowed lib.rs block (the re-exports of the policy of the matrix-free stage solve). No other pin changed: with the four allowed blocks removed, src/lib.rs still hashes to the pin of 2026-11-16.",
        "pin_of_the_driver": {
            "file": "crates/rodas5p-integrators/" + DRIVER,
            "old": base_pin,
            "old_equals_the_file_at_the_base": base_pin == driver_old,
            "new": driver_new,
            "pin_found_in_g3_before_this_run_of_the_script": pin_before_this_run,
        },
        "pin_of_stripped_lib_rs_unchanged": pin_stripped(g3_new_text[start:]),
        "stripped_lib_rs_hash_now": stripped_hash(lib_now, block),
        "allowed_block_added": block.split("\n")[:-1],
        "files": {
            "crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs": {"sha256_before": driver_old, "sha256_after": driver_new},
            "crates/rodas5p-integrators/src/lib.rs": {"sha256_before": sha256(at_base(args.base, "src/lib.rs")), "sha256_after": sha256(read("src/lib.rs"))},
            "crates/rodas5p-integrators/tests/ppsw1_6b_contracts.rs": {"sha256_before": sha256(at_base(args.base, G3)), "sha256_after": sha256(g3_new_text.encode("utf-8"))},
        },
        "second_sentinel_not_repinned": {
            "sentinel": "crates/rodas5p-integrators/tests/ptsw2b_contracts.rs, test h2_protected_sources_are_byte_identical",
            "why": "it pins the sha256 of src/rodas5p_matrix_free_fast.rs as well; the file is protected and is not in the write list of node V24, and g3 pins the bytes of tests/ptsw2b_contracts.rs itself. h2 therefore fails on this branch until the orchestrator changes two lines.",
            "h2_pin_now": h2_match.group(2) if h2_match else None,
            "h2_pin_needed": driver_new,
            "sha256_of_ptsw2b_contracts_rs_now": sha256(h2_now),
            "sha256_of_ptsw2b_contracts_rs_after_that_one_line_change": sha256(h2_proposed),
            "g3_pin_of_tests_ptsw2b_contracts_rs_now": g3_h2_pin,
            "g3_pin_of_tests_ptsw2b_contracts_rs_needed": sha256(h2_proposed),
            "written_by_this_node": False,
        },
        "not_pinned_by_this_sentinel": "src/routing.rs and crates/rodas5p-krylov/src/gmres_staged.rs (pinned by the loop checkpoint only)",
    }
    with open(os.path.join(NODE, "SENTINEL_REPIN.json"), "w", encoding="utf-8") as handle:
        json.dump(record, handle, indent=2)
        handle.write("\n")
    print(json.dumps(record["pin_of_the_driver"], indent=1))
    print("stripped lib.rs:", record["stripped_lib_rs_hash_now"], "pin:", record["pin_of_stripped_lib_rs_unchanged"])


def pin_stripped(g3_body):
    match = re.search(r'sha256_hex\(stripped\.as_bytes\(\)\),\s*"([0-9a-f]{64})"', g3_body)
    return match.group(1) if match else None


def stripped_hash(lib, v24_block):
    allowed = [
        "pub mod ppsw1;\n",
        "\n// Loop L1 (2026-11-16) opt-in modules.\npub mod mol;\npub mod stage_clients;\n",
    ]
    v22_start = lib.index("\n// Loop L1 node V22 (owner decision OD-L1-9)")
    v22_end = lib.index("};\n", v22_start) + len("};\n")
    allowed.append(lib[v22_start:v22_end])
    allowed.append(v24_block)
    stripped = lib
    for block in allowed:
        stripped = stripped.replace(block, "")
    return sha256(stripped.encode("utf-8"))


if __name__ == "__main__":
    sys.exit(main())
