"""Five new contract probes, not PR79's original 15-test or IVP campaign."""
import ctypes, hashlib, json, math, platform, subprocess, tempfile
from fractions import Fraction
from pathlib import Path
from readiness import norm_squared

root=Path(__file__).resolve().parent
expected={'lifted_error_norm.c':'74d4bf9247900dded1bb017fb1d216d58086d42d',
          'lifted_error_norm.h':'0ee50c5fb43218759f019b1a8d7adbab8c0e80ab'}
for name, want in expected.items():
    b=(root/'evidence'/name).read_bytes()
    assert hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest()==want

cases=[]
def add(name, entries, scale=1.0):
    e=[0.0]*30
    for i,v in entries.items(): e[i]=v
    cases.append((name,e,[scale]*33))
add('correlated_same_sign',{24:2.5,25:2.5})
add('correlated_opposite_sign',{24:2.5,25:-2.5})
add('denominator_33_vs_30',{0:5.5})
add('lift_overflow_but_normalized_finite',{24:1e308,25:1e308},1e308)
add('three_component_cancellation',{26:1e308,27:-1e308,28:1.0},1e308)
results=[]
with tempfile.TemporaryDirectory(prefix='vigilode-v1-c99-') as tmp:
    lib=Path(tmp)/'libnorm.so'
    command=['cc','-std=c99','-O2','-fPIC','-shared','-Wall','-Wextra',str(root/'evidence/lifted_error_norm.c'),'-lm','-o',str(lib)]
    built=subprocess.run(command,capture_output=True,text=True,timeout=20)
    (root/'evidence/C_BUILD.log').write_text(built.stdout+built.stderr)
    if built.returncode: raise RuntimeError('C_BUILD_FAILED')
    fn=ctypes.CDLL(str(lib)).bassrei_full33_error_norm
    fn.argtypes=[ctypes.POINTER(ctypes.c_double)]*3;fn.restype=ctypes.c_int
    for name,e,s in cases:
        q=norm_squared(e,s); oracle=math.sqrt(float(q))
        out=ctypes.c_double(-17.0)
        status=fn((ctypes.c_double*30)(*e),(ctypes.c_double*33)(*s),ctypes.byref(out))
        if status==3:
            assert out.value==-17.0
            classification='UNSUPPORTED_LONG_DOUBLE_NO_NATIVE_INFERENCE'
        else:
            assert status==0,(name,status)
            assert math.isclose(out.value,oracle,rel_tol=4e-15,abs_tol=0.0),(name,out.value,oracle)
            classification='C99_REFERENCE_CONFIRMED_ONLY'
        results.append({'name':name,'error30':e,'scale33':s,'exact_squared':str(q),'exact_accepts':q<=1,
                        'C99_status':status,'C99_norm':out.value,'oracle_norm':oracle,'classification':classification})
report={'schema':'vigilode.v1-prep-c99-crosscheck.v1','status':'PASS_SCOPED' if all(r['C99_status']==0 for r in results) else 'UNSUPPORTED_C99_ENVIRONMENT',
        'source_commit':'69e83044aacfb9a8fd2c6c3fb1146911beeedd79','source_blobs':expected,
        'compiler':subprocess.check_output(['cc','--version'],text=True).splitlines()[0],
        'host':platform.platform(),'cases':results,'native_rodas_executions':0,'ivp_executions':0,
        'original_campaign_replays':0,'new_allocation_measurement':False}
(root/'CROSSCHECK.json').write_text(json.dumps(report,indent=2,allow_nan=False)+'\n')
print(json.dumps({k:v for k,v in report.items() if k!='cases'}))
for r in results: print(r['name'],r['C99_status'],r['oracle_norm'],r['exact_accepts'])
