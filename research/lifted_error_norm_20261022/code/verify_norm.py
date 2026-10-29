#!/usr/bin/env python3
"""Compile C99 once and compare finite cases with an independent Decimal oracle."""
import argparse, ctypes, decimal, hashlib, json, math, os, platform, subprocess, tempfile, time
from pathlib import Path

p=argparse.ArgumentParser();p.add_argument('--output',type=Path,required=True);args=p.parse_args()
root=Path(__file__).resolve().parent;t0=time.monotonic();rows=[]
# Enough digits for exact input conversion/cancellation across binary64 extremes.
# The first 100-digit oracle rounded a 309-digit input before cancellation.
decimal.getcontext().prec=1200
D=decimal.Decimal
def oracle(e,sc):
    # Independent construction of the full33 tangent, not a state lift.
    f=[D.from_float(float(v)) for v in e[:23]]
    h=list(map(lambda x:D.from_float(float(x)),e[23:24]));he=list(map(lambda x:D.from_float(float(x)),e[24:26]));li=list(map(lambda x:D.from_float(float(x)),e[26:29]))
    for block in (h,he,li):f += [-sum(block,D(0))]+block
    f.append(D.from_float(float(e[29])))
    return float((sum((a/D.from_float(float(b)))**2 for a,b in zip(f,sc))/D(33)).sqrt())

with tempfile.TemporaryDirectory(prefix='lifted_norm_') as tmp:
    libpath=Path(tmp)/'liblifted_norm.so'
    cmd=['cc','-std=c99','-O2','-Wall','-Wextra','-Werror','-pedantic','-fPIC','-shared',str(root/'lifted_error_norm.c'),'-lm','-o',str(libpath)]
    build=subprocess.run(cmd,capture_output=True,text=True,check=False,timeout=10)
    if build.returncode:raise RuntimeError(build.stderr)
    lib=ctypes.CDLL(str(libpath));fn=lib.bassrei_full33_error_norm
    P=ctypes.POINTER(ctypes.c_double);fn.argtypes=[P,P,P];fn.restype=ctypes.c_int
    def call(e,s):
        out=ctypes.c_double(-123.)
        status=fn((ctypes.c_double*30)(*e),(ctypes.c_double*33)(*s),ctypes.byref(out))
        return status,out.value
    cases=[]
    cases.append(('zero',[0.]*30,[1.]*33))
    cases.append(('full33_denominator',[1.]*23+[0.]*7,[1.]*33))
    e=[0.]*30;e[26:29]=[1.,1.,1.];cases.append(('correlated_lithium_error',e,[1.]*33))
    e=[0.]*30;e[24:26]=[1e308,-1e308];s=[1.]*33;s[25]=1e-308;s[26:28]=[1e308,1e308];cases.append(('extreme_exact_cancellation',e,s))
    e=[1e300]*30;cases.append(('large_safe_norm',e,[1e300]*33))
    e=[1e-300]*30;cases.append(('tiny_safe_norm',e,[1e-300]*33))
    # State-dependent scales from reconstructed full33 old/new states.
    e=[(i-15)*1e-9 for i in range(30)];s=[1e-10+1e-6*max(abs(math.sin(i)),abs(math.cos(i))) for i in range(33)]
    cases.append(('dynamic_full33_scales',e,s))
    e=[0.]*30;e[26:29]=[1e308,-1e308,1e-300];s=[1.]*33;s[28]=1e-300;s[29:32]=[1e308]*3
    cases.append(('review_mixed_exponent_cancellation',e,s))
    e=[0.]*30;e[26:29]=[1e308,1e-300,-1e308]
    cases.append(('review_reordered_cancellation',e,s))
    e=[0.]*30;e[0]=1e308;s=[1.]*33;s[0]=.2
    cases.append(('ratio_overflows_but_rms_finite',e,s))
    for label,e,s in cases:
        status,value=call(e,s);expected=oracle(e,s)
        okay=status==0 and abs(value-expected)<=32*math.ulp(expected)
        rows.append(dict(case=label,status=status,value=value,decimal_oracle=expected,pass_case=okay))
    controls=[]
    for label,e,s,expected in [('nan',[math.nan]+[0.]*29,[1.]*33,1),
        ('zero_scale',[0.]*30,[0.]+[1.]*32,1),('negative_scale',[0.]*30,[-1.]+[1.]*32,1),
        ('norm_overflow',[1e308]*30,[1e-308]*33,2)]:
        status,value=call(e,s);controls.append(dict(case=label,status=status,output_preserved=value==-123.,pass_case=status==expected and value==-123.))
    nullout=ctypes.c_double(-123.);null_status=fn(None,None,ctypes.byref(nullout))
    controls.append(dict(case='null_pointers',status=null_status,pass_case=null_status==1 and nullout.value==-123.))
    corr=rows[2]['value'];naive=math.sqrt(3/30)
    semantic=dict(lifted_norm=corr,naive_diagonal30_norm=naive,ratio=corr/naive,
                  pass_case=corr>1.8*naive and abs(rows[1]['value']-math.sqrt(23/33))<1e-15)
    passed=all(r['pass_case'] for r in rows+controls) and semantic['pass_case']
    result=dict(status='PASS_SCOPED' if passed else 'FAIL',cases=rows,controls=controls,
                representation_counterexample=semantic,compile=dict(command=cmd[:-1]+['<temporary-library>'],exit_code=build.returncode,stderr=build.stderr),
                compiler=subprocess.check_output(['cc','--version'],text=True).splitlines()[0],
                python=platform.python_version(),elapsed_seconds=time.monotonic()-t0,
                source_sha256={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in root.iterdir() if p.is_file()},
                native_vigilode_solver_runs=0,ncp_jobs=0,core_or_default_changes=False)
args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(result,indent=2,allow_nan=False)+'\n')
print(json.dumps(result,indent=2));raise SystemExit(0 if passed else 1)
