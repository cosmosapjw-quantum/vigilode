#!/usr/bin/env python3
"""Hostile failure controls for the original source thermal Rust path.

Fail-closed on source data, source owner official-history forgery and
derived root byte corruption. No original owner/source file is edited.
"""
from __future__ import annotations
import argparse,hashlib,json,shutil,subprocess,sys,tempfile
from pathlib import Path
def reject(cmd,needle,timeout=25):
    p=subprocess.run(cmd,stdout=subprocess.PIPE,stderr=subprocess.PIPE,
                     text=True,timeout=timeout)
    return p.returncode!=0 and needle in p.stderr
def main(a):
    passed={}
    with tempfile.TemporaryDirectory(prefix="f05_hostile_",dir=str(a.out.parent)) as d:
        q=Path(d)
        corrupt=q/"fake_owner.bin"
        shutil.copyfile(a.source,corrupt)
        with corrupt.open("r+b") as h:
            h.seek(400);old=h.read(1);h.seek(400)
            h.write(bytes([old[0]^0x01]))
        passed["source_bitflip_native_rejected"]=reject(
            [str(a.native_binary),str(corrupt),str(a.manifest),str(q/"fake.raw")],
            "ORIGINAL_THERMAL_SOURCE_AUTHORITY_DRIFT")
        m=json.loads(a.manifest.read_text())
        m["official_tick161"]="ACCEPTED"
        bad=q/"forged_official.json";bad.write_text(json.dumps(m))
        passed["forged_official_tick161_rejected"]=reject(
            [str(a.native_binary),str(a.source),str(bad),str(q/"fake2.raw")],
            "ORIGINAL_THERMAL_SOURCE_AUTHORITY_DRIFT")
        root=bytearray(a.native.read_bytes())
        root[100]^=1
        changed=q/"forged_native_roots.raw";changed.write_bytes(root)
        aud=[str(a.auditor),"--source",str(a.source),"--rust",str(changed),
             "--receipt",str(a.receipt),"--owner_manifest",str(a.manifest),
             "--out",str(q/"bad_audit.json")]
        passed["python_normal_tampered_native_rejected"]=reject(
            [sys.executable]+aud,"NATIVE_THERMAL_OUTPUT_SHA_MISMATCH")
        passed["python_optimized_tampered_native_rejected"]=reject(
            [sys.executable,"-O"]+aud,"NATIVE_THERMAL_OUTPUT_SHA_MISMATCH")
        passed["energy_ledger_tampered_native_rejected"]=reject(
            [str(a.energy_binary),str(a.source),str(changed)],
            "ORIGINAL_REI_THERMAL_LEDGER_IDENTITY_OR_SCHEMA")
    result={"schema":"REI_F05_THERMAL_NATIVE_HOSTILE_NEGATIVE_CONTROLS_V1",
        "results":passed,"all_rejected":all(passed.values()),
        "original_v1_tick161":"NOT_ACCEPTED",
        "production":"HOLD","canonical_delta":0}
    a.out.write_text(json.dumps(result,sort_keys=True,indent=2)+"\n")
    print(json.dumps(result,sort_keys=True))
    if not result["all_rejected"]:raise ArithmeticError("HOSTILE_TAMPER_NOT_REJECTED")
if __name__=="__main__":
    ap=argparse.ArgumentParser()
    for name in ("source","native","manifest","receipt","native_binary",
                 "energy_binary","auditor","out"):
        ap.add_argument("--"+name,type=Path,required=True)
    main(ap.parse_args())
