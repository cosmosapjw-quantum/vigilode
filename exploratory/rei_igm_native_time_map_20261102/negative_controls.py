#!/usr/bin/env python3
"""Read-only negative source authority, clock and binary-state tamper tests."""
from __future__ import annotations
import argparse,hashlib,json,subprocess,tempfile
from pathlib import Path
def negative(binfile,parent,oracle,photon,clock,raw,receipt,independent):
    cases={}
    with tempfile.TemporaryDirectory(prefix="rei-f05-native-tamper-") as td:
        td=Path(td)
        badclock=td/"clock.json"
        x=json.loads(clock.read_text())
        x["maps"][0]["tg_s"]+=3.0
        badclock.write_text(json.dumps(x,sort_keys=True))
        proc=subprocess.run([str(binfile),str(parent),str(oracle),str(photon),
                   str(badclock),str(td/"reject-state.raw")],capture_output=True,text=True,timeout=30)
        cases["clock_tampered_rejected"]=(proc.returncode!=0
                 and "EXACT_ORIGINAL_CLOCK_JSON_DRIFT" in proc.stderr)
        badparent=td/"parent.raw"
        src=bytearray(parent.read_bytes());src[32]^=1
        badparent.write_bytes(src)
        proc=subprocess.run([str(binfile),str(badparent),str(oracle),str(photon),
                   str(clock),str(td/"reject-parent.raw")],capture_output=True,text=True,timeout=30)
        cases["parent_sha_tampered_rejected"]=(proc.returncode!=0
                 and "ACCEPTED_REI_F05_PARENT_RAW_SHA_MISMATCH" in proc.stderr)
        badraw=td/"rust_state.raw"
        src=bytearray(raw.read_bytes());src[128]^=1
        badraw.write_bytes(src)
        proc=subprocess.run(["python3",str(independent),"--raw",str(badraw),"--parent",str(parent),
           "--clock",str(clock),"--receipt",str(receipt),"--out",str(td/"reject-audit.json")],
            capture_output=True,text=True,timeout=30)
        cases["rust_state_sha_tampered_rejected"]=(proc.returncode!=0
                  and "RAW_SHA_REPORT_MISMATCH" in proc.stderr)
    return cases
if __name__=="__main__":
    p=argparse.ArgumentParser()
    for x in ("binary","parent","oracle","photon","clock","raw","receipt","independent","out"):
        p.add_argument("--"+x,type=Path,required=True)
    a=p.parse_args()
    cases=negative(a.binary,a.parent,a.oracle,a.photon,a.clock,a.raw,a.receipt,a.independent)
    a.out.write_text(json.dumps(dict(schema="REI_F05_NATIVE_RUST_TAMPER_NEGATIVE_CONTROLS_V1",
        cases=cases,all_expected_failures_rejected=all(cases.values()),
        canonical_delta=0,official_tick161="NOT_ACCEPTED",production="HOLD"),indent=2,sort_keys=True)+"\n")
    print(json.dumps(cases,sort_keys=True))
    if not all(cases.values()):raise SystemExit("NEGATIVE_TAMPER_CONTROL_NOT_REJECTED")
