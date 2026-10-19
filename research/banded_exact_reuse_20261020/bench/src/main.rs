use std::{sync::Arc,time::Instant};
use rodas5p_integrators::{OdeProblem,AdaptiveStepConfig,OutputSchedule,BandedJacobian,BandedKernel,integrate_rodas5p_fast_banded_observed_with_kernel};
use serde_json::json;

fn build(name:&str)->Result<(OdeProblem,BandedJacobian,Vec<f64>,f64,usize,bool),String>{
    let (kind,num)=name.rsplit_once('-').ok_or("case must end with cell count")?;
    let cells:usize=num.parse().map_err(|_|"invalid cells")?;
    if ![50,127,200].contains(&cells){return Err("unregistered cell count".into());}
    let linear=match kind{"brusselator"=>false,"linear-diffusion"=>true,_=>return Err("unknown model".into())};
    let n=2*cells;let d=((cells+1) as f64).powi(2)/50.;
    let rhs=Arc::new(move|_t:f64,y:&[f64],f:&mut[f64]|{
        for i in 0..cells{let a=2*i;let b=a+1;let u=y[a];let v=y[b];
            let edge=if linear{(0.,0.)}else{(1.,3.)};
            let (ul,vl)=if i==0{edge}else{(y[a-2],y[b-2])};
            let (ur,vr)=if i+1==cells{edge}else{(y[a+2],y[b+2])};
            if linear{f[a]=d*(ul-2.*u+ur);f[b]=d*(vl-2.*v+vr);}
            else{f[a]=1.+u*u*v-4.*u+d*(ul-2.*u+ur);f[b]=3.*u-u*u*v+d*(vl-2.*v+vr);}
        }Ok(())
    });
    let band=BandedJacobian{lower:2,upper:2,fill:Arc::new(move|_t,y,band|{
        for i in 0..cells{let a=2*i;let b=a+1;let u=y[a];let v=y[b];
            if linear{band[a*5+2]=-2.*d;band[b*5+2]=-2.*d;}
            else{band[a*5+2]=2.*u*v-4.-2.*d;band[a*5+3]=u*u;band[b*5+1]=3.-2.*u*v;band[b*5+2]=-u*u-2.*d;}
            if i>0{band[a*5]=d;band[b*5]=d;}if i+1<cells{band[a*5+4]=d;band[b*5+4]=d;}
        }Ok(())
    })};
    let p=OdeProblem::new(name,n,rhs,None,None,None,None,true,None,None).map_err(|e|e.to_string())?;
    let y0=(0..cells).flat_map(|i|{let x=(i+1) as f64/(cells+1) as f64;
        if linear{[(std::f64::consts::PI*x).sin(),0.5*(2.*std::f64::consts::PI*x).sin()]}
        else{[1.+(2.*std::f64::consts::PI*x).sin(),3.]}}).collect();
    Ok((p,band,y0,if linear{1.}else{10.},cells,linear))
}
fn main()->Result<(),Box<dyn std::error::Error>>{
    let a:Vec<String>=std::env::args().skip(1).collect();
    if a.len()!=7{return Err("usage: band-study CASE KERNEL RTOL GRID H0_SCALE REPS WARMUPS".into());}
    let (p,band,y0,tf,cells,linear)=build(&a[0])?;
    let kernel=match a[1].as_str(){"slices"=>BandedKernel::Slices,
        #[cfg(feature="improvement")] "fixed2"=>BandedKernel::FixedWidth2,
        #[cfg(feature="improvement")] "exact-cache"=>BandedKernel::FixedWidth2ExactCache,
        _=>return Err("unknown kernel".into())};
    let rt:f64=a[2].parse()?;let hs:f64=a[4].parse()?;let reps:usize=a[5].parse()?;let warm:usize=a[6].parse()?;
    if !rt.is_finite()||rt<=0.||rt>=1.||![0.5,1.,2.].contains(&hs)||reps==0||reps>100||warm>10{return Err("invalid configuration".into());}
    let ts:Vec<f64>=match a[3].as_str(){"endpoint"=>vec![0.,tf],"uniform101"=>(0..101).map(|i|tf*i as f64/100.).collect(),"dyadic129"=>(0..129).map(|i|tf*i as f64/128.).collect(),_=>return Err("unknown grid".into())};
    let schedule=OutputSchedule::new(ts.clone())?;
    let cfg=AdaptiveStepConfig{rtol:rt,atol:rt,initial_step:1e-6*hs,min_step:1e-14,max_step:tf,max_attempts:1_000_000,..AdaptiveStepConfig::default()};
    let run=||integrate_rodas5p_fast_banded_observed_with_kernel(&p,&band,(0.,tf),&y0,&cfg,&schedule,kernel);
    for _ in 0..warm{let r=run()?;if !r.fast.observed.success{return Err("warmup solver failure".into());}}
    let mut elapsed=Vec::with_capacity(reps);let mut results=Vec::with_capacity(reps);
    for _ in 0..reps{let t=Instant::now();let r=run();let dt=t.elapsed().as_nanos();elapsed.push(dt);results.push(r);}
    let first=results[0].as_ref().map_err(|e|e.to_string())?;
    let initial_bits:Vec<u64>=first.fast.observed.y.iter().flatten().map(|x|x.to_bits()).collect();
    let deterministic=results.iter().all(|r|r.as_ref().is_ok_and(|r|r.fast.observed.t==first.fast.observed.t&&r.fast.observed.y.iter().flatten().map(|x|x.to_bits()).eq(initial_bits.iter().copied())&&r.fast.observed.success==first.fast.observed.success));
    let complete=results.iter().all(|r|r.as_ref().is_ok_and(|r|r.fast.observed.success&&r.fast.observed.t==ts&&r.fast.observed.y.len()==ts.len()&&r.fast.observed.y.iter().all(|row|row.len()==2*cells&&row.iter().all(|x|x.is_finite()))));
    let counters=&first.fast.observed.counters;
    let obj=json!({"schema":"banded-improvement-v1","case":a[0],"kernel":a[1],"cells":cells,"dimension":2*cells,"linear":linear,"rtol":rt,"atol":rt,"grid":a[3],"h0_scale":hs,
      "status":if complete{"ok"}else{"solver_failure"},"deterministic":deterministic,"times":first.fast.observed.t,"states":first.fast.observed.y,"elapsed_ns":elapsed,
      "warmups":warm,"repetitions":reps,"attempts":first.fast.attempts,"accepted":first.fast.accepted_steps,"rejected":first.fast.rejected_steps,"jacobian_reuses":first.fast.jacobian_reuses,
      "driver":first.fast.driver,"counters":counters,"banded_work":first.work,"exact_lu_reuses_in_successful_run":first.fast.attempts as u64-counters.direct_factorizations,
      "timing_scope":"integration workspace construction through result return; no model setup/serialization/drops/parity checks", "timing_authority":"diagnostic_shared_runner"});
    println!("{}",obj);Ok(())
}
