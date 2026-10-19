"""One-time, exact-source repair. Publish repaired source BEFORE compiling it.

No numerical solver library is changed. Initial source hashes are checked and
all before/after hashes are retained. A second invocation validates idempotence.
"""
import hashlib,json,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
OUT=Path(sys.argv[1])

def blob(s):
    b=s.encode();return hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest()
def change(path,expected,edits):
    p=ROOT/path;s=p.read_text();old=blob(s)
    if old!=expected:
        if all(new in s and oldtext not in s for oldtext,new in edits):return {'path':path,'status':'ALREADY_APPLIED','blob':old}
        raise RuntimeError('UNEXPECTED_SOURCE:'+path+':'+old)
    for oldtext,new in edits:
        if s.count(oldtext)!=1:raise RuntimeError('ANCHOR_COUNT:'+path)
        s=s.replace(oldtext,new,1)
    p.write_text(s)
    return {'path':path,'status':'APPLIED','before_blob':old,'after_blob':blob(s),'after_sha256':hashlib.sha256(s.encode()).hexdigest()}

helper='''def _json_numpy_scalar(value):
    if isinstance(value,(np.integer,np.floating,np.bool_)):
        return value.item()
    raise TypeError(f"Unsupported JSON type: {type(value).__name__}")

def dump(path, value):
    Path(path).write_text(json.dumps(value,indent=2,sort_keys=True,allow_nan=False,default=_json_numpy_scalar)+'\\n')'''
records=[change('scripts/campaign.py','467b2c1e51b35b60f856c8e9aba77d38fda2b1a0',[
    ("def dump(path, value):\n    Path(path).write_text(json.dumps(value,indent=2,sort_keys=True,allow_nan=False)+'\\n')",helper)])]
records.append(change('src/main.rs','fd804cb73450a6506b95a18040722f3d2e72c42e',[
    ('let pd=unsafe{std::slice::from_raw_parts_mut(pd,ld*n)};\n    pd.fill(0.0);',
     '''// DVODE passes a pointer shifted by ml rows into the factor workspace.
    // The final column exposes only its logical band rows, not all ld rows.
    let band_rows=if ml<0 {ld}else{(ml+mu+1) as usize};
    let extent=ld*(n-1)+band_rows;
    let pd=unsafe{std::slice::from_raw_parts_mut(pd,extent)};
    for col in 0..n {pd[col*ld..col*ld+band_rows].fill(0.0);}'''),
    ('let complete=r.observed.t==times && r.observed.y.len()==times.len();',
     'let complete=r.observed.success && r.observed.t==times && r.observed.y.len()==times.len();'),
    ('    #[test]\n    fn analytic_entries_match_centered_differences()',
     '''    #[test]
    fn banded_callback_preserves_trailing_guard() {
        let y=[1.0,3.0,1.0,3.0];let n=4usize;let ld=7usize;
        let sentinel=1234567.0;let mut storage=vec![sentinel;ld*n+4];
        unsafe {strict_jac(4,n as i32,2,2,ld as i32,0.0,y.as_ptr(),storage.as_mut_ptr());}
        let logical_end=ld*(n-1)+5;
        assert!(storage[logical_end..].iter().all(|x|*x==sentinel));
        let mut expected=vec![vec![0.0;n];n];jac_entries(4,n,|i|y[i],|i,j,z|expected[i][j]=z);
        for i in 0..n {for j in 0..n {let k=2+i as i32-j as i32;if (0..5).contains(&k){assert_eq!(storage[k as usize+ld*j],expected[i][j]);}}}
    }
    #[test]
    fn analytic_entries_match_centered_differences()''')]))
OUT.mkdir(parents=True,exist_ok=True)
(OUT/'SOURCE_REPAIR_RECEIPT.json').write_text(json.dumps(records,indent=2)+'\n')
print(json.dumps(records,indent=2))
