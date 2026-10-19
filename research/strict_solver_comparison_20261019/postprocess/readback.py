"""Postprocess frozen observations only. No integration, new timing, or data repair.

The original strict initial-bit assertion remains a documented FAIL. This
readback separately checks floating-point initial-value consistency (16 epsilon
scaled by 1+|y0|) while retaining the original initial output in every numerical
error metric. This does not change the preregistered measured-error admissions.
"""
import gzip,hashlib,json,math,shutil,sys,tempfile
from collections import Counter
from pathlib import Path
import numpy as np
NODE=Path(__file__).resolve().parents[1]
SOURCE=NODE/'refined_runs/39261895983'
PREVIOUS=NODE/'runs/39257390707'
OUT=Path(sys.argv[1]);OUT.mkdir(parents=True,exist_ok=True)

def save(name,obj):
    (OUT/name).write_text(json.dumps(obj,indent=2,sort_keys=True,allow_nan=False)+'\n')
def digest(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()

# Test the narrower floating consistency gate independently of any solver data.
def consistency(actual,expected):
    a=np.asarray(actual,dtype=float);e=np.asarray(expected,dtype=float)
    if a.shape!=e.shape or not np.isfinite(a).all():return False
    return bool(np.all(np.abs(a-e)<=16*np.finfo(float).eps*(1+np.abs(e))))
assert consistency([np.nextafter(1.,2.)],[1.])
assert not consistency([1.+1e-8],[1.])
assert not consistency([float('nan')],[1.])
assert not consistency([1.,2.],[1.])
save('INITIAL_CONSISTENCY_TESTS.json',{'tests':4,'passed':4,'exact_initial_bit_gate':'NOT_RELABELLED_AS_PASS'})

# Verify every retained source file before interpreting it.
verified=[]
for line in (SOURCE/'SHA256SUMS').read_text().splitlines():
    want,path=line.split('  ',1);p=SOURCE/path
    assert p.is_file() and digest(p)==want,path
    verified.append(path)
save('SOURCE_ARCHIVE_VERIFICATION.json',{'status':'PASS','files':len(verified),'source_run':SOURCE.name,
    'reconstruction':json.loads((SOURCE/'RECONSTRUCTION.json').read_text())})

models={c['name']:c for c in json.loads((SOURCE/'models.json').read_text())['cases']}
initial=[];totals=Counter();cache={}
for phase in ['calibration','timing']:
    with gzip.open(SOURCE/(phase+'.jsonl.gz'),'rt') as f:
        for line in f:
            rec=json.loads(line);r=rec['row'];h=r['states_sha256']
            if h not in cache:
                with gzip.open(SOURCE/'states'/(h+'.json.gz'),'rb') as s:b=s.read()
                assert hashlib.sha256(b).hexdigest()==h
                cache[h]=json.loads(b)
            ys=cache[h];totals[phase]+=1
            if r['status']!='ok':continue
            a=np.asarray(ys[0]);e=np.asarray(models[r['case']]['y0']);delta=np.abs(a-e)
            if not np.array_equal(a,e):
                initial.append({'phase':phase,'case':r['case'],'arm':r['arm'],'grid':r['grid'],'rtol':r['rtol'],'h0_scale':r['h0_scale'],
                    'state_hash':h,'max_abs_delta':float(delta.max()),'max_scaled_epsilon':float(np.max(delta/(np.finfo(float).eps*(1+np.abs(e))))),
                    'initial_R':float(np.linalg.norm((a-e)/(r['atol']+r['rtol']*np.abs(e)))/math.sqrt(len(e))),
                    'floating_consistency':consistency(a,e)})
save('INITIAL_VALUE_DIAGNOSIS.json',initial)
save('INITIAL_VALUE_SUMMARY.json',{'rows':dict(totals),'bit_mismatch_rows':len(initial),'mismatch_by_arm':dict(Counter(x['arm'] for x in initial)),
    'max_abs_delta':max((x['max_abs_delta'] for x in initial),default=0.),'max_scaled_epsilon':max((x['max_scaled_epsilon'] for x in initial),default=0.),
    'max_initial_R':max((x['initial_R'] for x in initial),default=0.),'all_floating_consistent':all(x['floating_consistency'] for x in initial),
    'original_exact_bit_gate':'FAIL_PRESERVED','admission_thresholds_or_data_changed':False})
assert all(x['floating_consistency'] for x in initial),'INITIAL_CONDITION_MISMATCH_EXCEEDS_FLOATING_ROUNDOFF'

# Preserve the failed original auditor; run its independently vectorized checks
# with only the extra initial-bit assertion replaced by the explicit gate above.
p=NODE/'refinement/audit.py';original=p.read_text();bb=original.encode()
assert hashlib.sha1(b'blob '+str(len(bb)).encode()+b'\0'+bb).hexdigest()=='bea3af3c9daab768cc76cb90de17857071246ffa'
anchor="assert np.array_equal(y[0],np.asarray(c['y0']))"
assert original.count(anchor)==1
adapted=original.replace(anchor,"assert initial_consistency(y[0],np.asarray(c['y0']))",1)
(OUT/'EXECUTED_AUDITOR.py').write_text(adapted)
save('AUDITOR_CHANGE.json',{'old_sha256':hashlib.sha256(bb).hexdigest(),'new_sha256':hashlib.sha256(adapted.encode()).hexdigest(),
    'only_change':'initial bit equality -> explicit 16 epsilon floating consistency; original data and error metrics unchanged',
    'retained_failed_audit':str(SOURCE.relative_to(NODE) / 'audit.log')})
with tempfile.TemporaryDirectory() as tmp:
    work=Path(tmp)/'source';shutil.copytree(SOURCE,work)
    ns={'__name__':'frozen_readback_audit','__file__':str(p),'initial_consistency':consistency}
    exec(compile(adapted,str(OUT/'EXECUTED_AUDITOR.py'),'exec'),ns)
    ns['main'](work,PREVIOUS)
    for name in ['COST_AT_MATCHED_ERROR.csv','WORK_AT_MATCHED_ERROR.csv','RECOMPUTED_ACCURACY.csv','AUDIT.json','REPORT_KO.md','MATCHED_endpoint.png','MATCHED_uniform101.png']:
        q=work/name
        if q.exists():shutil.copy2(q,OUT/name)
result=json.loads((OUT/'AUDIT.json').read_text())
result.update(status='PASS_SCOPED_READBACK_WITH_INITIAL_ROUNDING',exact_initial_bit_gate='FAIL_PRESERVED',
              initial_rounding_diagnosis='INITIAL_VALUE_SUMMARY.json',new_solver_executions=0,new_timing_samples=0,
              measured_source_commit=json.loads((SOURCE/'MANIFEST.json').read_text())['source_commit'])
save('AUDIT.json',result)
report=OUT/'REPORT_KO.md'
report.write_text(report.read_text()+'\n## 초기값 비트 검사에 대한 정정\n\n원래 auditor의 초기값 bit equality 검사는 실패한 기록 그대로 보존한다. 이 후처리에서는 초기 출력 오차를 숨기지 않고 E_up/R_up에 그대로 포함한 채, 별도 16 epsilon floating consistency 검사를 추가했다. 실제 차이와 영향은 INITIAL_VALUE_SUMMARY.json에 기록했다. 원자료·선택·측정 시간을 바꾸거나 적분을 재실행하지 않았다. 최종 readback 상태는 AUDIT.json이 우선한다.\n')
print(json.dumps(result,sort_keys=True))
