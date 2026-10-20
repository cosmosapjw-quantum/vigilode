use std::sync::Arc;
use rodas5p_core::{CoreError,LinearMethod,LinearSolverConfig,PreconditionerKind};
use rodas5p_integrators::{OdeProblem,ParallelExecution};
use vigilode_coupled::{Config,coupled_step,coupled_or_sequential};
#[test]
fn local_error_decreases_against_exact_riccati_solution(){
 for eps in [0.0,0.5]{
  let p=OdeProblem::new("riccati-convergence",6,Arc::new(move|_,y,f|{for i in 0..6{f[i]=(-1.0-(i%3)as f64)*y[i]+eps*y[i]*y[i];}Ok(())}),None,None,
   Some(Arc::new(move|_,y,v,f|{for i in 0..6{f[i]=(-1.0-(i%3)as f64+2.0*eps*y[i])*v[i];}Ok(())})),None,true,None,None).unwrap();
  let mut errors=Vec::new();
  for h in [0.2,0.1,0.05,0.025]{
   let r=coupled_step(&p,0.0,&[0.1;6],h,&Config::default(),&ParallelExecution::sequential());
   let y=r.candidate.expect("convergence probe must complete").y_new;
   let error=(0..6).map(|i|{let a=-1.0-(i%3)as f64;let e=(a*h).exp();let exact=0.1*e/(1.0+(eps*0.1/a)*(1.0-e));(y[i]-exact).abs()}).fold(0.0,f64::max);
   errors.push(error);
  }
  println!("RICCATI_CONVERGENCE eps={eps} errors={errors:?}");
  assert!(errors.windows(2).all(|e|e[1]<e[0]));
  assert!(errors[3]<errors[0]/1000.0,"local error refinement failed: {errors:?}");
 }
}
#[test]
fn failed_fallback_keeps_structured_callback_counts(){
 let p=OdeProblem::new("fallback-callback",2,Arc::new(|_,_,_|Err(CoreError::NonFinite("injected fallback failure".into()))),None,None,
  Some(Arc::new(|_,_,v,f|{f.copy_from_slice(v);Ok(())})),None,true,None,None).unwrap();
 let c=Config{max_rhs:0,..Config::default()};
 let l=LinearSolverConfig{method:LinearMethod::Gmres,preconditioner:PreconditionerKind::None,..LinearSolverConfig::default()};
 let r=coupled_or_sequential(&p,0.0,&[1.0;2],0.01,&c,&ParallelExecution::sequential(),&l);
 assert!(r.fallback_attempted);assert!(r.fallback.is_none());assert!(r.fallback_failure.is_some());
 assert_eq!(r.candidate_attempt.work.rhs_attempted,0);assert!(r.fallback_counters.rhs_calls>0);
}
