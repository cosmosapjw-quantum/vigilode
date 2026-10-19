//! Single-process native comparison. Model callbacks are shared across solvers.
//! No wall-clock or accuracy promotion is implied by a successful process.
use std::{env, io::{self, Write}, sync::Arc, time::Instant};
use diffsol::{DenseMatrix as _, FaerLU, FaerMat, MatrixCommon, OdeBuilder, OdeSolverMethod};
use rodas5p_core::DenseMatrix;
use rodas5p_integrators::{integrate_rodas5p_fast_observed, AdaptiveStepConfig, OdeProblem, OutputSchedule};
use serde_json::{json, Value};

#[derive(Clone)]
struct Case { id: i32, name: &'static str, n: usize, tf: f64, scale: f64 }
fn cases() -> Vec<Case> {
    vec![
        Case{id:1,name:"robertson",n:3,tf:40.0,scale:1e-4},
        Case{id:2,name:"hires",n:8,tf:321.8122,scale:1e-4},
        Case{id:3,name:"van-der-pol-mu1000",n:2,tf:2000.0,scale:1.0},
        Case{id:4,name:"brusselator-1d-50",n:100,tf:10.0,scale:1.0},
        Case{id:5,name:"brusselator-1d-200",n:400,tf:10.0,scale:1.0},
        Case{id:6,name:"pivot-linear",n:2,tf:1.0,scale:1e-4},
    ]
}
fn initial(c: &Case) -> Vec<f64> {
    match c.id {
        1 => vec![1.0,0.0,0.0],
        2 => vec![1.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0057],
        3 => vec![2.0,0.0],
        4|5 => (0..c.n/2).flat_map(|i| {let x=(i+1) as f64/(c.n/2+1) as f64; [1.0+(2.0*std::f64::consts::PI*x).sin(),3.0]}).collect(),
        6 => vec![1.0,0.0],
        _ => unreachable!(),
    }
}
fn rhs(id:i32,n:usize,y:impl Fn(usize)->f64,mut put:impl FnMut(usize,f64)) {
    match id {
        1 => {let a=y(0);let b=y(1);let c=y(2);put(0,-0.04*a+1e4*b*c);put(1,0.04*a-1e4*b*c-3e7*b*b);put(2,3e7*b*b);}
        2 => {
            put(0,-1.71*y(0)+0.43*y(1)+8.32*y(2)+0.0007);
            put(1,1.71*y(0)-8.75*y(1));put(2,-10.03*y(2)+0.43*y(3)+0.035*y(4));
            put(3,8.32*y(1)+1.71*y(2)-1.12*y(3));put(4,-1.745*y(4)+0.43*y(5)+0.43*y(6));
            put(5,-280.0*y(5)*y(7)+0.69*y(3)+1.71*y(4)-0.43*y(5)+0.69*y(6));
            put(6,280.0*y(5)*y(7)-1.81*y(6));put(7,-280.0*y(5)*y(7)+1.81*y(6));
        }
        3 => {put(0,y(1));put(1,1000.0*(1.0-y(0)*y(0))*y(1)-y(0));}
        4|5 => {let m=n/2;let d=((m+1) as f64).powi(2)/50.0;
            for k in 0..m {let a=2*k;let b=a+1;let u=y(a);let v=y(b);
                let (ul,vl)=if k==0 {(1.0,3.0)} else {(y(a-2),y(b-2))};
                let (ur,vr)=if k+1==m {(1.0,3.0)} else {(y(a+2),y(b+2))};
                put(a,1.0+u*u*v-4.0*u+d*(ul-2.0*u+ur));put(b,3.0*u-u*u*v+d*(vl-2.0*v+vr));
            }
        }
        6 => {put(0,-y(0));put(1,1000.0*y(0)-1000.0*y(1));}
        _ => unreachable!(),
    }
}
fn jac_entries(id:i32,n:usize,y:impl Fn(usize)->f64,mut emit:impl FnMut(usize,usize,f64)) {
    match id {
        1 => {emit(0,0,-0.04);emit(0,1,1e4*y(2));emit(0,2,1e4*y(1));emit(1,0,0.04);emit(1,1,-1e4*y(2)-6e7*y(1));emit(1,2,-1e4*y(1));emit(2,1,6e7*y(1));}
        2 => {
            for (i,j,z) in [(0,0,-1.71),(0,1,0.43),(0,2,8.32),(1,0,1.71),(1,1,-8.75),(2,2,-10.03),(2,3,0.43),(2,4,0.035),(3,1,8.32),(3,2,1.71),(3,3,-1.12),(4,4,-1.745),(4,5,0.43),(4,6,0.43),(5,3,0.69),(5,4,1.71),(5,6,0.69),(6,6,-1.81),(7,6,1.81)] {emit(i,j,z);}
            emit(5,5,-280.0*y(7)-0.43);emit(5,7,-280.0*y(5));emit(6,5,280.0*y(7));emit(6,7,280.0*y(5));emit(7,5,-280.0*y(7));emit(7,7,-280.0*y(5));
        }
        3 => {emit(0,1,1.0);emit(1,0,-2000.0*y(0)*y(1)-1.0);emit(1,1,1000.0*(1.0-y(0)*y(0)));}
        4|5 => {let m=n/2;let d=((m+1) as f64).powi(2)/50.0;
            for k in 0..m {let a=2*k;let b=a+1;let u=y(a);let v=y(b);
                emit(a,a,2.0*u*v-4.0-2.0*d);emit(a,b,u*u);emit(b,a,3.0-2.0*u*v);emit(b,b,-u*u-2.0*d);
                if k>0 {emit(a,a-2,d);emit(b,b-2,d);} if k+1<m {emit(a,a+2,d);emit(b,b+2,d);}
            }
        }
        6 => {emit(0,0,-1.0);emit(1,0,1000.0);emit(1,1,-1000.0);}
        _ => unreachable!(),
    }
}

// The C ABI is reached only through our checked Fortran bridge. Its pointers
// refer to non-overlapping buffers of the declared lengths; no callbacks escape.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strict_rhs(id:i32,n:i32,_t:f64,y:*const f64,out:*mut f64) {
    let y=unsafe{std::slice::from_raw_parts(y,n as usize)};
    let out=unsafe{std::slice::from_raw_parts_mut(out,n as usize)};
    rhs(id,n as usize,|i|y[i],|i,z|out[i]=z);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strict_jac(id:i32,n:i32,ml:i32,mu:i32,ld:i32,_t:f64,y:*const f64,pd:*mut f64) {
    let n=n as usize;let ld=ld as usize;
    let y=unsafe{std::slice::from_raw_parts(y,n)};
    // DVODE passes a pointer shifted by ml rows into the factor workspace.
    // The final column exposes only its logical band rows, not all ld rows.
    let band_rows=if ml<0 {ld}else{(ml+mu+1) as usize};
    let extent=ld*(n-1)+band_rows;
    let pd=unsafe{std::slice::from_raw_parts_mut(pd,extent)};
    for col in 0..n {pd[col*ld..col*ld+band_rows].fill(0.0);}
    jac_entries(id,n,|i|y[i],|i,j,z| {
        if ml<0 {pd[i+ld*j]=z;} else {
            let k=mu+i as i32-j as i32;
            if k>=0 && k<ml+mu+1 {pd[k as usize+ld*j]=z;}
        }
    });
}
unsafe extern "C" {
    fn strict_vode(id:i32,n:i32,nt:i32,tf:f64,y0:*const f64,rtol:f64,atol:f64,h0:f64,times:*const f64,band:i32,landing:i32,values:*mut f64,reached:*mut f64,completed:*mut i32,code:*mut i32,stats:*mut i32);
}

struct Outcome { times:Vec<f64>, states:Vec<Vec<f64>>, counters:Value, status:String, error:Option<String> }
fn failed(s:String)->Outcome {Outcome{times:vec![],states:vec![],counters:Value::Null,status:"solver_failure".into(),error:Some(s)}}
fn native(c:&Case,times:&[f64],rtol:f64,h:f64,audit:bool)->Outcome {
    let (id,n)=(c.id,c.n);
    let problem=OdeProblem::new(c.name,n,
        Arc::new(move |_t,y,out|{rhs(id,n,|i|y[i],|i,z|out[i]=z);Ok(())}),None,
        Some(Arc::new(move |_t,y|{let mut j=DenseMatrix::zeros(n,n);jac_entries(id,n,|i|y[i],|i,k,z|j[(i,k)]=z);Ok(j)})),None,None,true,None,None)
        .map(|p|p.with_jacobian_into(Arc::new(move |_t,y,j|{jac_entries(id,n,|i|y[i],|i,k,z|j[(i,k)]=z);Ok(())})));
    let p=match problem {Ok(p)=>p,Err(e)=>return failed(e.to_string())};
    let cfg=AdaptiveStepConfig{rtol,atol:rtol*c.scale,initial_step:h,min_step:1e-14,max_step:c.tf,max_attempts:1_000_000,..AdaptiveStepConfig::default()};
    let schedule=match OutputSchedule::new(times.to_vec()){Ok(s)=>s,Err(e)=>return failed(e.to_string())};
    match integrate_rodas5p_fast_observed(&p,(0.0,c.tf),&initial(c),&cfg,&schedule) {
        Err(e)=>failed(e.to_string()),
        Ok(r)=> {let complete=r.observed.success && r.observed.t==times && r.observed.y.len()==times.len();
            let counters=if audit {json!({"native":r.observed.counters,"attempts":r.attempts,"accepted":r.accepted_steps,"rejected":r.rejected_steps})} else {Value::Null};
            Outcome{times:r.observed.t,states:r.observed.y,counters,status:if complete {"ok"}else{"solver_failure"}.into(),error:if complete {None}else{Some("incomplete observed trajectory".into())}}
        }
    }
}
fn diff(c:&Case,times:&[f64],rtol:f64,h:f64,audit:bool)->Outcome {
    let (id,n)=(c.id,c.n);let y0=initial(c);
    let p=OdeBuilder::<FaerMat<f64>>::new().rtol(rtol).atol(vec![rtol*c.scale;n]).h0(h)
        .rhs_implicit(
            move |x,_p,_t,out| {rhs(id,n,|i|x[i],|i,z|out[i]=z);},
            move |x,_p,_t,v,out| {for i in 0..n {out[i]=0.0;}jac_entries(id,n,|i|x[i],|i,j,z|out[i]+=z*v[j]);})
        .init(move |_p,_t,y|{for i in 0..n {y[i]=y0[i];}},n).build();
    let p=match p {Ok(p)=>p,Err(e)=>return failed(e.to_string())};
    let mut solver=match p.bdf::<FaerLU<f64>>() {Ok(s)=>s,Err(e)=>return failed(e.to_string())};
    let solved=solver.solve_dense(times);
    let counters=if audit {serde_json::to_value(solver.get_statistics()).unwrap_or(Value::Null)}else{Value::Null};
    match solved {
        Err(e)=>{let mut r=failed(e.to_string());r.counters=counters;r},
        Ok((m,why))=> {
            let complete=m.ncols()==times.len() && m.nrows()==n;
            let states=(0..m.ncols()).map(|k|(0..n).map(|i|m.get_index(i,k)).collect()).collect();
            Outcome{times:times.iter().take(m.ncols()).copied().collect(),states,counters,status:if complete {"ok"}else{"solver_failure"}.into(),error:if complete {None}else{Some(format!("incomplete output {why:?}"))}}
        }
    }
}
fn vode(c:&Case,times:&[f64],rtol:f64,h:f64,audit:bool,landing:bool)->Outcome {
    let mut values=vec![f64::NAN;c.n*times.len()];let mut reached=0.0;let mut completed=0;let mut code=0;let mut stats=[0i32;8];
    let band=if c.id==4 || c.id==5 {2}else{-1};
    unsafe {strict_vode(c.id,c.n as i32,times.len() as i32,c.tf,initial(c).as_ptr(),rtol,rtol*c.scale,h,times.as_ptr(),band,landing as i32,values.as_mut_ptr(),&mut reached,&mut completed,&mut code,stats.as_mut_ptr());}
    let got=(completed.max(0) as usize).min(times.len());
    let complete=code==2 && got==times.len() && reached==c.tf;
    Outcome{times:times[..got].to_vec(),states:values[..got*c.n].chunks(c.n).map(|a|a.to_vec()).collect(),
        counters:if audit{json!({"nst":stats[0],"nfe":stats[1],"nje":stats[2],"nlu":stats[3],"nni":stats[4],"ncfn":stats[5],"netf":stats[6],"order":stats[7],"istate":code,"last_returned_time":reached,"bandwidth":band})}else{Value::Null},
        status:if complete {"ok"}else{"solver_failure"}.into(),error:if complete{None}else{Some(format!("DVODE ISTATE={code}, reached={reached}, outputs={got}"))}}
}
fn run(c:&Case,arm:&str,times:&[f64],rtol:f64,h:f64,audit:bool)->Outcome {
    let mut r=match arm {"vigilode-fast"=>native(c,times,rtol,h,audit),"diffsol-bdf"=>diff(c,times,rtol,h,audit),"netlib-dvode"=>vode(c,times,rtol,h,audit,false),"netlib-dvode-land"=>vode(c,times,rtol,h,audit,true),_=>failed("unknown arm".into())};
    if !r.states.iter().flatten().all(|x|x.is_finite()) {r.status="solver_failure".into();r.error=Some("nonfinite states".into());}
    r
}
fn output_grid(c:&Case,g:&str)->Result<Vec<f64>,String>{match g {"endpoint"=>Ok(vec![0.0,c.tf]),"uniform101"=>Ok((0..101).map(|i|c.tf*i as f64/100.0).collect()),_=>Err("unknown grid".into())}}
fn bits(r:&Outcome)->Vec<u64>{r.states.iter().flatten().map(|x|x.to_bits()).collect()}
fn listing()->Value {
    let cs:Vec<Value>=cases().iter().map(|c|{
        let y0=initial(c);let probe:Vec<f64>=y0.iter().enumerate().map(|(i,y)|y*(1.0+0.1*((i+1) as f64).sin())+0.01*((i+1) as f64).cos()).collect();
        let samples:Vec<Value>=[y0.clone(),probe].iter().map(|y|{let mut f=vec![0.0;c.n];rhs(c.id,c.n,|i|y[i],|i,z|f[i]=z);let mut j=vec![vec![0.0;c.n];c.n];jac_entries(c.id,c.n,|i|y[i],|i,k,z|j[i][k]=z);json!({"y":y,"f":f,"jac":j})}).collect();
        json!({"id":c.id,"name":c.name,"dimension":c.n,"tf":c.tf,"scale":c.scale,"y0":y0,"samples":samples})
    }).collect();
    json!({"schema":"strict-solver-models-v1","source_commit":env!("STRICT_SOURCE_COMMIT"),"cases":cs,"arms":["vigilode-fast","diffsol-bdf","netlib-dvode"]})
}
fn main()->Result<(),Box<dyn std::error::Error>> {
    let a:Vec<String>=env::args().skip(1).collect();
    if a.first().map(String::as_str)==Some("models") {println!("{}",listing());return Ok(());}
    if a.len()!=8 {return Err("usage: strict-compare {solve|bench} CASE ARM RTOL GRID H0_SCALE WARMUPS REPS".into());}
    let c=cases().into_iter().find(|c|c.name==a[1]).ok_or("unknown case")?;
    let arm=&a[2];if !["vigilode-fast","diffsol-bdf","netlib-dvode","netlib-dvode-land"].contains(&arm.as_str()){return Err("unknown arm".into());}
    let rtol:f64=a[3].parse()?;let scale:f64=a[5].parse()?;let warm:usize=a[6].parse()?;let reps:usize=a[7].parse()?;
    if !(rtol.is_finite()&&rtol>0.0&&rtol<1.0&&[0.5,1.0,2.0].contains(&scale)&&reps>0&&reps<=100&&warm<=10){return Err("invalid configuration".into());}
    let audit=match a[0].as_str(){"solve"=>true,"bench"=>false,_=>return Err("invalid mode".into())};
    let times=output_grid(&c,&a[4])?;let h=1e-6*scale;
    for _ in 0..warm {let r=run(&c,arm,&times,rtol,h,false);if r.status!="ok" {return Err(format!("warmup failed: {:?}",r.error).into());}}
    let mut ns=vec![];let mut first:Option<Outcome>=None;let mut deterministic=true;let mut statuses=vec![];
    for _ in 0..reps {let start=Instant::now();let r=run(&c,arm,&times,rtol,h,audit);let dt=start.elapsed().as_nanos();ns.push(dt);statuses.push(r.status.clone());
        if let Some(old)=&first {deterministic &= bits(old)==bits(&r) && old.status==r.status && old.times==r.times;}else{first=Some(r);}}
    let r=first.unwrap();
    let row=json!({"schema":"strict-native-row-v1","source_commit":env!("STRICT_SOURCE_COMMIT"),"case":c.name,"arm":arm,"dimension":c.n,"mode":a[0],"rtol":rtol,"atol":rtol*c.scale,"scale":c.scale,"grid":a[4],"h0_scale":scale,"h0":h,"times":r.times,"states":r.states,"status":r.status,"error":r.error,"counters":r.counters,"elapsed_ns":ns,"repetition_statuses":statuses,"warmups":warm,"repetitions":reps,"deterministic":deterministic,"timing_authority":"diagnostic_shared_runner","timing_includes_counter_serialization":audit,"output_policy":if arm=="vigilode-fast"||arm.ends_with("-land"){"land_on_requested_times"}else{"solver_dense_interpolation"}});
    let mut out=io::BufWriter::new(io::stdout().lock());serde_json::to_writer(&mut out,&row)?;writeln!(out)?;Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
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
    fn analytic_entries_match_centered_differences() {
        for c in cases(){let y=initial(&c);let mut j=vec![vec![0.0;c.n];c.n];jac_entries(c.id,c.n,|i|y[i],|i,k,z|j[i][k]=z);
            for k in 0..c.n.min(10){let step=1e-5*(1.0+y[k].abs());let mut yp=y.clone();let mut ym=y.clone();yp[k]+=step;ym[k]-=step;let mut fp=vec![0.0;c.n];let mut fm=fp.clone();rhs(c.id,c.n,|i|yp[i],|i,z|fp[i]=z);rhs(c.id,c.n,|i|ym[i],|i,z|fm[i]=z);
                for i in 0..c.n {let d=(fp[i]-fm[i])/(2.0*step);assert!((d-j[i][k]).abs()<1e-4*(1.0+d.abs()+j[i][k].abs()),"{} {i} {k}",c.name);}}}
    }
    #[test]
    fn three_native_solvers_match_exact_pivot_fixture(){let c=cases().pop().unwrap();let t=output_grid(&c,"uniform101").unwrap();for arm in ["vigilode-fast","diffsol-bdf","netlib-dvode","netlib-dvode-land"]{let r=run(&c,arm,&t,1e-9,1e-6,true);assert_eq!(r.status,"ok","{arm} {:?}",r.error);for (tt,y) in t.iter().zip(&r.states){let refy=[(-tt).exp(),1000.0/999.0*((-tt).exp()-(-1000.0*tt).exp())];for i in 0..2 {assert!((y[i]-refy[i]).abs()<1e-6,"{arm} t={tt}");}}}}
}
