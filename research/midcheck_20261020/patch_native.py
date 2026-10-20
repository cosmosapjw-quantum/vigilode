"""Extend pinned native 3-way comparator with the current coupled-step arms.

All five legacy benchmark problems and their code stay unchanged. New case 7
has identical RHS for all solvers. This patch is an executable source
transformation, not numerical evidence until the compiled binary is checked.
"""
from pathlib import Path
import hashlib, json, sys

node=Path(sys.argv[1])
p=node/"src/main.rs"
s=p.read_text()
before=hashlib.sha256(s.encode()).hexdigest()

def rep(old,new):
    global s
    n=s.count(old)
    if n!=1: raise RuntimeError(f"anchor count {n}: {old[:110]}")
    s=s.replace(old,new)

rep("use serde_json::{json, Value};", """use serde_json::{json, Value};
use vigilode_coupled::{Config as CoupledConfig,coupled_step,coupled_step_residual_first};
use rodas5p_integrators::ParallelExecution;""")
rep('Case{id:6,name:"pivot-linear",n:2,tf:1.0,scale:1e-4},',
    'Case{id:6,name:"pivot-linear",n:2,tf:1.0,scale:1e-4},\n        Case{id:7,name:"few-mode-96",n:96,tf:0.001,scale:1e-4},')
rep('6 => vec![1.0,0.0],','6 => vec![1.0,0.0],\n        7 => vec![0.1;96],')
# id=7 inserted at unique closure following id=6 in both RHS and Jacobian
rep('6 => {put(0,-y(0));put(1,1000.0*y(0)-1000.0*y(1));}',
    '6 => {put(0,-y(0));put(1,1000.0*y(0)-1000.0*y(1));}\n        7 => {for i in 0..n {let a=[-1.0,-20.0,-100.0][i%3];put(i,a*y(i)+y(i)*y(i));}}')
rep('6 => {emit(0,0,-1.0);emit(1,0,1000.0);emit(1,1,-1000.0);}',
    '6 => {emit(0,0,-1.0);emit(1,0,1000.0);emit(1,1,-1000.0);}\n        7 => {for i in 0..n {emit(i,i,[-1.0,-20.0,-100.0][i%3]+2.0*y(i));}}')
# predecessor source identity is materialized in build source receipt, never
# substitute an external different-version binary.
marker='fn run(c:&Case,arm:&str,times:&[f64],rtol:f64,h:f64,audit:bool)->Outcome {'
coupled=r"""
fn coupled(c:&Case,times:&[f64],rtol:f64,lazy:bool,audit:bool)->Outcome {
    if c.id!=7 || times.len()!=2 {return failed("coupled arm only supports few-mode-96 endpoint".into());}
    let id=c.id;let n=c.n;let yy=initial(c);
    let rhs_cb=Arc::new(move |_t:f64,y:&[f64],out:&mut[f64]| {rhs(id,n,|i|y[i],|i,z|out[i]=z);Ok(())});
    let jvp_cb=Arc::new(move |_t:f64,y:&[f64],v:&[f64],out:&mut[f64]| {
        out.fill(0.0);
        jac_entries(id,n,|i|y[i],|i,j,z|out[i]+=z*v[j]);
        Ok(())
    });
    let p=match OdeProblem::new(c.name,n,rhs_cb,None,None,Some(jvp_cb),None,true,None,None) {
        Ok(p)=>p,Err(e)=>return failed(e.to_string())};
    let cfg=CoupledConfig{rtol,atol:rtol*c.scale,..CoupledConfig::default()};
    let exec=ParallelExecution::sequential();
    let attempt=if lazy {coupled_step_residual_first(&p,0.0,&yy,c.tf,&cfg,&exec)}
        else {coupled_step(&p,0.0,&yy,c.tf,&cfg,&exec)};
    let counters=if audit {serde_json::to_value(&attempt.work).unwrap_or(Value::Null)} else {Value::Null};
    match attempt.candidate {
        Some(candidate)=>{
            let valid=candidate.embedded_test_passes && candidate.y_new.iter().all(|x|x.is_finite());
            Outcome{times:times.to_vec(),states:vec![yy,candidate.y_new],counters,
                status:if valid {"ok"} else {"embedded_rejected"}.into(),
                error:if valid {None} else {Some(format!("embedded WRMS={}, root WRMS={}",candidate.embedded_wrms,candidate.full_residual_wrms))}}
        }
        None=>{let mut r=failed(format!("coupled declined: {:?}",attempt.failure));r.counters=counters;r}
    }
}
"""
rep(marker,coupled+"\n"+marker)
rep('"vigilode-fast"=>native(c,times,rtol,h,audit),',
    '"vigilode-fast"=>native(c,times,rtol,h,audit),"coupled-eager"=>coupled(c,times,rtol,false,audit),"coupled-lazy"=>coupled(c,times,rtol,true,audit),')
rep('"netlib-dvode","vigilode-banded"]',
    '"netlib-dvode","vigilode-banded","coupled-eager","coupled-lazy"]')
rep('"netlib-dvode-land","vigilode-banded"].contains',
    '"netlib-dvode-land","vigilode-banded","coupled-eager","coupled-lazy"].contains')
# Supply each solver the same initial step h=tf for the new short interval.
rep('let h=1e-6*scale;','let h=if c.id==7 {c.tf*scale} else {1e-6*scale};')
p.write_text(s)
cargo=node/"Cargo.toml"
txt=cargo.read_text()
if txt.count('[dependencies]')!=1:raise RuntimeError("missing Cargo deps")
txt=txt.replace('[dependencies]','[dependencies]\nvigilode-coupled = { path = "../../crates/vigilode-coupled" }')
cargo.write_text(txt)
(node/"PATCH_RECEIPT.json").write_text(json.dumps({"prior_sha256":before,"new_sha256":hashlib.sha256(s.encode()).hexdigest(),"new_case":"few-mode-96","source_equations":"a*y+y^2 (a=-1,-20,-100 repeated)","new_arms":["coupled-eager","coupled-lazy"],"legacy_five_rhs_unchanged":True},indent=2)+"\n")
print("PATCHED",before,hashlib.sha256(s.encode()).hexdigest())