//! Declared bandwidth-two implementation, using the SAME analytic entries.
use super::*;
use rodas5p_integrators::{BandedJacobian,BandedKernel,integrate_rodas5p_fast_banded_observed_with_kernel};

pub(super) fn solve(c:&Case,times:&[f64],rtol:f64,h:f64,audit:bool)->Outcome {
    if c.id!=4 && c.id!=5 {return failed("banded arm applies only to the declared Brusselator models".into());}
    let (id,n)=(c.id,c.n);
    let p=OdeProblem::new(c.name,n,
        Arc::new(move |_t,y,out|{rhs(id,n,|i|y[i],|i,z|out[i]=z);Ok(())}),None,
        Some(Arc::new(move |_t,y|{let mut j=DenseMatrix::zeros(n,n);jac_entries(id,n,|i|y[i],|i,k,z|j[(i,k)]=z);Ok(j)})),None,None,true,None,None);
    let p=match p {Ok(p)=>p,Err(e)=>return failed(e.to_string())};
    let band=BandedJacobian{lower:2,upper:2,fill:Arc::new(move |_t,y,b|{
        jac_entries(id,n,|i|y[i],|i,j,z|{let k=2+j as isize-i as isize;if (0..5).contains(&k){b[i*5+k as usize]=z;}else{assert_eq!(z,0.0,"out-of-band Jacobian entry");}});Ok(())})};
    let cfg=AdaptiveStepConfig{rtol,atol:rtol*c.scale,initial_step:h,min_step:1e-14,max_step:c.tf,max_attempts:1_000_000,..AdaptiveStepConfig::default()};
    let schedule=match OutputSchedule::new(times.to_vec()){Ok(s)=>s,Err(e)=>return failed(e.to_string())};
    match integrate_rodas5p_fast_banded_observed_with_kernel(&p,&band,(0.0,c.tf),&initial(c),&cfg,&schedule,BandedKernel::Slices) {
        Err(e)=>failed(e.to_string()),
        Ok(result)=>{let r=result.fast;let complete=r.observed.success && r.observed.t==times && r.observed.y.len()==times.len();
            let counters=if audit {json!({"native":r.observed.counters,"attempts":r.attempts,"accepted":r.accepted_steps,"rejected":r.rejected_steps,"banded_work":result.work})}else{Value::Null};
            Outcome{times:r.observed.t,states:r.observed.y,counters,status:if complete{"ok"}else{"solver_failure"}.into(),error:if complete{None}else{Some("incomplete native banded output".into())}}
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn banded_matches_dense_storage_on_both_grids_and_initial_steps(){
        let c=cases().into_iter().find(|c|c.id==4).unwrap();
        for g in ["endpoint","uniform101"]{let ts=output_grid(&c,g).unwrap();for h in [5e-7,1e-6,2e-6]{
            let a=native(&c,&ts,1e-6,h,true);let b=solve(&c,&ts,1e-6,h,true);
            assert_eq!(a.status,"ok");assert_eq!(b.status,"ok");assert_eq!(a.times,b.times);
            for (x,y) in a.states.iter().flatten().zip(b.states.iter().flatten()){assert!((x-y).abs()<1e-10*(1.0+x.abs()));}
        }}
    }
}
