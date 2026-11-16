#!/usr/bin/env python3
"""Explicit tamper rejection against the original Rust MPRK interval path."""
import argparse,json,subprocess,tempfile
from pathlib import Path
if __name__=="__main__":
    p=argparse.ArgumentParser()
    for a in ("binary","source","manifest","output","out"):p.add_argument("--"+a,type=Path,required=True)
    args=p.parse_args()
    outcomes={}
    with tempfile.TemporaryDirectory(prefix="rei-mprk-negative-") as td:
        t=Path(td)
        corrupt=bytearray(args.source.read_bytes());corrupt[256]^=1
        bad=t/"mutated.bin";bad.write_bytes(corrupt)
        rr=subprocess.run([str(args.binary),str(bad),str(args.manifest),str(t/"x.raw")],
                          capture_output=True,text=True,timeout=20)
        outcomes["source_binary_sha_rejected"]=rr.returncode!=0 and "AUTHORITY_DRIFT" in rr.stderr
        m=json.loads(args.manifest.read_text());m["official_tick161"]="ACCEPTED"
        (t/"forged.json").write_text(json.dumps(m))
        rr=subprocess.run([str(args.binary),str(args.source),str(t/"forged.json"),str(t/"x.raw")],
                          capture_output=True,text=True,timeout=20)
        outcomes["forged_tick161_admission_rejected"]=rr.returncode!=0 and "AUTHORITY_DRIFT" in rr.stderr
    rec=dict(schema="F05_ORIGINAL_MPRK_INTERVAL_NEGATIVE_CONTROL_V1",
       results=outcomes,all_rejected=all(outcomes.values()),
       official_tick161="NOT_ACCEPTED",production="HOLD",canonical_delta=0)
    args.out.write_text(json.dumps(rec,indent=2,sort_keys=True)+"\n")
    print(json.dumps(rec,sort_keys=True))
    if not rec["all_rejected"]:raise AssertionError("EXPECTED_TAMPER_REJECTION_MISSING")
