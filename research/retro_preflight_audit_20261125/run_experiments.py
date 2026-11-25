#!/usr/bin/env python3
"""Pinned MathHarness extension records for opt-in preparation fixtures only.
Default is plan; no reference adjudication or arbitrary command execution."""
import argparse, datetime, hashlib, json, os, pathlib, platform, subprocess, sys, time, zipfile
P=pathlib.Path
ROOT=P(__file__).resolve().parents[2]
ZIP_SHA="937ddfd13e860162e6bf454d0b3178c64d34a59de5c0d1df98dc513020f678ee"
CASES={
    "M1":("event_combinations",8,"existing crossing/terminal output; no true terminal transaction"),
    "M2":("dae_combinations",8,"consistent manufactured index-1 DAE; accepted nodes"),
    "M3":("rhs_combinations",288,"dyadic strict-dominance matrices; dense/sparse, 1/4 threads, 4 solve modes, 3 verifiers"),
    "M4":("dictionary_combinations",24,"scalar dissipative Green IVP; four tables, two step caps, three rates"),
    "M5":("aperiodic_combinations",32,"noncommuting discrete transitions and existing continuous nonautonomous solver"),
}
def hashfile(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def dump(p,obj): p.write_text(json.dumps(obj,indent=2,sort_keys=True,allow_nan=False)+"\n")
def utc(): return datetime.datetime.now(datetime.timezone.utc).isoformat()
def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--execute-smoke",action="store_true")
    ap.add_argument("--topic",choices=CASES,action="append")
    ap.add_argument("--harness-zip",type=P)
    ap.add_argument("--harness-root",type=P)
    ap.add_argument("--output",type=P)
    args=ap.parse_args()
    topics=args.topic or list(CASES)
    if len(topics)!=len(set(topics)): ap.error("duplicate topic")
    if not args.execute_smoke:
        print(json.dumps({"state":"PLANNED","cases":{k:CASES[k] for k in topics},"M0":"NOT_RUN","M6":"not full benchmark"},indent=2));return
    if not all([args.harness_zip,args.harness_root,args.output]): ap.error("execution needs pinned archive, extracted root and fresh output")
    assert hashfile(args.harness_zip)==ZIP_SHA,"archive identity"
    hroot=args.harness_root.resolve()
    with zipfile.ZipFile(args.harness_zip) as z:
        manifest=json.loads(z.read("mathharness-general-3.1.0/SHA256SUMS.json"))
        for name,digest in manifest["files"].items():
            path=(hroot/name).resolve()
            assert hroot in path.parents and hashfile(path)==digest,name
    sys.path.insert(0,str(hroot))
    from mhgext.validate import validate_run_manifest
    out=(ROOT/args.output).resolve()
    assert ROOT in out.parents
    out.mkdir(parents=True,exist_ok=False)
    tracked=subprocess.check_output(["git","ls-files","-z","--","crates","Cargo.toml","Cargo.lock","rust-toolchain.toml",".cargo"],cwd=ROOT).decode().split("\0")
    files=sorted(set([x for x in tracked if x and P(x).suffix in {".rs",".toml",".lock",".c",".h",".f",".json"}]+["crates/vigilode/tests/retro_preflight_combinations.rs","research/retro_preflight_audit_20261125/run_experiments.py"]))
    sources={f:hashfile(ROOT/f) for f in files if (ROOT/f).is_file()}
    source_file=out/"sources.json"
    dump(source_file,{"git_head":subprocess.check_output(["git","rev-parse","HEAD"],cwd=ROOT,text=True).strip(),"harness_sha256":ZIP_SHA,"files":sources})
    def unchanged():
        return all((ROOT/f).is_file() and hashfile(ROOT/f)==h for f,h in sources.items())
    env=dict(os.environ);env.update(RAYON_NUM_THREADS="4",OMP_NUM_THREADS="1",OPENBLAS_NUM_THREADS="1")
    argv=["cargo","test","--locked","--offline","-j6","--release","-p","vigilode","--features","retro-dictionary","--test","retro_preflight_combinations","--no-run","--message-format=json"]
    build=subprocess.run(argv,cwd=ROOT,env=env,capture_output=True,timeout=300)
    (out/"build.jsonl").write_bytes(build.stdout);(out/"build.stderr").write_bytes(build.stderr)
    assert build.returncode==0,build.stderr.decode(errors="replace")
    artifacts=[r["executable"] for l in build.stdout.splitlines() if (r:=json.loads(l)).get("reason")=="compiler-artifact" and r.get("target",{}).get("name")=="retro_preflight_combinations" and r.get("executable")]
    assert len(artifacts)==1 and unchanged()
    binary=P(artifacts[0]);binary_hash=hashfile(binary)
    src_hash=hashfile(source_file)
    summary={}
    for topic in topics:
        test,count,scope=CASES[topic]
        cmd=[str(binary),test,"--exact","--nocapture","--test-threads=1"]
        started=utc();t0=time.monotonic()
        try:
            run=subprocess.run(cmd,cwd=ROOT,env=env,capture_output=True,timeout=60)
            code=run.returncode;stdout=run.stdout;stderr=run.stderr;timeout=False
        except subprocess.TimeoutExpired as exc:
            code=None;stdout=exc.stdout or b"";stderr=exc.stderr or b"";timeout=True
        log=out/(topic+".log");err=out/(topic+".stderr")
        log.write_bytes(stdout);err.write_bytes(stderr)
        cells=[]
        for line in stdout.decode(errors="replace").splitlines():
            if "CELL " in line: cells.append(json.loads(line.split("CELL ",1)[1]))
        fields={"error","bound","accepted","constraint","outcome"}
        keys=[json.dumps({k:v for k,v in c.items() if k not in fields},sort_keys=True) for c in cells]
        pass_cells=len(cells)==count and len(set(keys))==count and all(c.get("topic")==topic for c in cells)
        same=unchanged() and hashfile(binary)==binary_hash
        passed=(code==0 and pass_cells and same and b"1 passed; 0 failed" in stdout)
        result=out/(topic+".json");dump(result,{"topic":topic,"expected":count,"observed":len(cells),"passed":passed,"cells":cells})
        status="completed" if passed else ("timed_out" if timeout else "failed")
        m={"schema":"mhg-run/1.0","run_id":"retro-preflight-"+topic,"project_id":"vigilode","objective":"prepare actual experiment combinations","assertion_tested":test,"domain":{"scope":scope,"expected_cells":count},"conventions":{"binary64":"Rust release; original solver contracts","results":"finite smoke fixtures; not a performance adjudication"},"status":status,"execution":{"argv":cmd,"cwd":".","exit_code":code,"started_at":started,"finished_at":utc(),"runtime_seconds":time.monotonic()-t0,"binary_sha256":binary_hash},"environment":{"os":platform.platform(),"architecture":platform.machine(),"software":{"rustc":subprocess.check_output(["rustc","--version"],text=True).strip(),"mathharness":"3.1.0","harness_archive_sha256":ZIP_SHA},"threads":{"RAYON_NUM_THREADS":"4","OMP_NUM_THREADS":"1","OPENBLAS_NUM_THREADS":"1"}},"inputs":[{"artifact_id":"source-lock","path":str(source_file.relative_to(ROOT)),"sha256_before":src_hash,"sha256_after":hashfile(source_file),"immutable":True}],"outputs":[{"artifact_id":p.name,"path":str(p.relative_to(ROOT)),"sha256":hashfile(p),"role":role} for p,role in [(log,"log"),(err,"log"),(result,"result")]],"checks":[{"check_id":"process-and-coverage","verdict":"PASS" if passed else "FAIL","observed":{"exit":code,"cells":len(cells),"unique":len(set(keys)),"source_unchanged":same},"expected":{"exit":0,"cells":count,"source_unchanged":True}}],"non_claims":["M0 not executed; no new reference or matched-error classification","No universal DAE/event/aperiodic solver qualification","Finite test cases only"],"failure_kind":None if passed else ("runtime_environment" if timeout else ("provenance" if not same else "implementation"))}
        dump(out/(topic+".manifest.json"),m)
        msgs=validate_run_manifest(m,ROOT,check_artifacts=True)
        (out/(topic+".validation.txt")).write_text("\n".join(msgs)+"\n")
        summary[topic]={"status":status,"cells":len(cells),"expected_refusals":sum(c.get("outcome")=="expected_refusal" for c in cells)}
        print(topic,summary[topic],flush=True)
    dump(out/"SUMMARY.json",summary)
    if not all(x["status"]=="completed" for x in summary.values()):sys.exit(1)
if __name__=="__main__":main()
