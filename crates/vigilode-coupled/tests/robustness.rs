use std::sync::{Arc,atomic::{AtomicU64,AtomicUsize,Ordering}};
use rodas5p_core::{CoreError,LinearMethod,LinearSolverConfig,PreconditionerKind};
use rodas5p_integrators::{OdeProblem,ParallelExecution};
use vigilode_coupled::{Config,FailureKind,coupled_step,coupled_or_sequential};
fn diagonal(n:usize)->OdeProblem{
 OdeProblem::new("diagonal",n,Arc::new(move|_,y,f|{for i in 0..n{f[i]=(-1.0-i as f64)*y[i];}Ok(())}),None,None,
 Some(Arc::new(move|_,_,v,f|{for i in 0..n{f[i]=(-1.0-i as f64)*v[i];}Ok(())})),None,true,None,None).unwrap()
}
#[test]fn budget_preflight_does_no_callbacks(){let p=diagonal(9);let c=Config{max_rhs:0,..Config::default()};let r=coupled_step(&p,0.0,&[1.0;9],0.01,&c,&ParallelExecution::sequential());assert!(r.candidate.is_none());assert_eq!(r.failure.unwrap().kind,FailureKind::Budget);assert_eq!(r.work.rhs_attempted,0);}
#[test]fn storage_plan_rejects_before_callbacks(){let c=Config{max_storage_bytes:1,..Config::default()};let r=coupled_step(&diagonal(9),0.0,&[1.0;9],0.01,&c,&ParallelExecution::sequential());assert!(r.candidate.is_none());assert_eq!(r.work.rhs_attempted,0);}
#[test]fn basis_leakage_can_expand_without_dense_jacobian(){let c=Config{initial_basis:1,max_basis:9,max_expansions:4,..Config::default()};let r=coupled_step(&diagonal(9),0.0,&[1.0;9],0.01,&c,&ParallelExecution::sequential());assert!(r.candidate.is_some(),"{:?}",r.failure);assert!(r.work.basis_expansions>0);assert!(r.candidate.unwrap().full_residual_wrms<=c.full_residual_wrms_limit);}
#[test]fn changing_client_epoch_invalidates_attempt(){
 let e=Arc::new(AtomicU64::new(0));let r=e.clone();let q=e.clone();let p=OdeProblem::new("epoch",1,Arc::new(move|_,y,f|{f[0]=-y[0];r.fetch_add(1,Ordering::SeqCst);Ok(())}),None,None,Some(Arc::new(|_,_,v,f|{f[0]=-v[0];Ok(())})),None,true,None,None).unwrap().with_model_epoch(Arc::new(move||q.load(Ordering::SeqCst)));
 let a=coupled_step(&p,0.0,&[1.0],0.01,&Config::default(),&ParallelExecution::sequential());assert_eq!(a.failure.unwrap().kind,FailureKind::ModelChanged);assert_eq!(a.work.rhs_attempted,1);
}
#[test]fn failed_parallel_batch_is_joined_and_counted(){
 let calls=Arc::new(AtomicUsize::new(0));let cc=calls.clone();let p=OdeProblem::new("failure",3,Arc::new(move|_,y,f|{let k=cc.fetch_add(1,Ordering::SeqCst);if k==2{return Err(CoreError::NonFinite("injected callback failure".into()));}for i in 0..3{f[i]=(-1.0-i as f64)*y[i];}Ok(())}),None,None,
 Some(Arc::new(|_,_,v,f|{for i in 0..3{f[i]=(-1.0-i as f64)*v[i];}Ok(())})),None,true,None,None).unwrap();
 let y=[1.0;3];let a=coupled_step(&p,0.0,&y,0.01,&Config::default(),&ParallelExecution::rayon(4).unwrap());
 assert!(a.candidate.is_none());assert_eq!(a.work.rhs_attempted,calls.load(Ordering::SeqCst)as u64);assert_eq!(a.work.rhs_attempted,9);
 assert_eq!(a.work.rhs_completed,8);assert_eq!(a.work.stage_tasks_scheduled,8);assert_eq!(a.work.stage_tasks_completed,8);assert_eq!(y,[1.0;3]);
}
#[test]fn partial_jvp_output_is_not_silently_zero_filled(){
 let p=OdeProblem::new("partial",2,Arc::new(|_,y,f|{f.copy_from_slice(y);Ok(())}),None,None,Some(Arc::new(|_,_,v,f|{f[0]=v[0];Ok(())})),None,true,None,None).unwrap();
 let a=coupled_step(&p,0.0,&[1.0;2],0.01,&Config::default(),&ParallelExecution::sequential());assert!(a.candidate.is_none());assert_eq!(a.work.jvp_attempted,1);assert_eq!(a.work.jvp_completed,0);
}
#[test]fn invalid_config_does_not_panic_or_run(){
 for c in [Config{max_basis:usize::MAX,..Config::default()},Config{theta:f64::NAN,..Config::default()},Config{rtol:-1.0,..Config::default()},Config{initial_basis:0,..Config::default()}]{let a=coupled_step(&diagonal(3),0.0,&[1.0;3],0.01,&c,&ParallelExecution::sequential());assert!(a.candidate.is_none());assert_eq!(a.work.rhs_attempted,0);}
}
#[test]fn nonautonomous_problem_is_explicitly_unsupported(){let mut p=diagonal(3);p.autonomous=false;let a=coupled_step(&p,0.0,&[1.0;3],0.01,&Config::default(),&ParallelExecution::sequential());assert_eq!(a.failure.unwrap().kind,FailureKind::Unsupported);}
#[test]fn lambda_one_preserves_target_for_two_theta_values(){let p=diagonal(9);let y=[1.0;9];let mut outputs=Vec::new();for theta in [0.0,0.4,1.0]{let c=Config{initial_basis:9,max_basis:9,theta,..Config::default()};let a=coupled_step(&p,0.0,&y,0.01,&c,&ParallelExecution::sequential());outputs.push(a.candidate.unwrap().y_new);}for i in 1..outputs.len(){for(a,b)in outputs[0].iter().zip(&outputs[i]){assert!((a-b).abs()<1e-11);}}}
#[test]fn root_budget_failure_is_not_an_accepted_state(){let c=Config{theta:0.0,max_newton:0,max_subdivisions:2,..Config::default()};let a=coupled_step(&diagonal(3),0.0,&[1.0;3],0.01,&c,&ParallelExecution::sequential());assert!(a.candidate.is_none());assert_eq!(a.work.lambda_rejections,2);assert_eq!(a.failure.unwrap().kind,FailureKind::RootIteration);}
#[test]fn protected_fallback_preserves_candidate_cost(){
 let p=diagonal(3);let c=Config{max_jvp:1,..Config::default()};let linear=LinearSolverConfig{method:LinearMethod::Gmres,preconditioner:PreconditionerKind::None,rtol:1e-12,atol:1e-14,maxiter:100,..LinearSolverConfig::default()};
 let s=coupled_or_sequential(&p,0.0,&[1.0;3],0.01,&c,&ParallelExecution::sequential(),&linear);assert_eq!(s.selection,"sequential-matrix-free-fallback");assert!(s.candidate_attempt.failure.is_some());assert_eq!(s.candidate_attempt.work.jvp_attempted,1);assert!(s.fallback.unwrap().counters.jvp_calls>0);
}
#[test]fn full_operator_invertible_does_not_make_projection_safe(){
 let hg=0.01*rodas5p_core::rodas5p_coefficients().unwrap().gamma;
 let p=OdeProblem::new("singular-projection",2,Arc::new(move|_,y,f|{f[0]=(y[0]-y[1])/hg+1.0;f[1]=(y[0]+y[1])/hg;Ok(())}),None,None,
 Some(Arc::new(move|_,_,v,f|{f[0]=(v[0]-v[1])/hg;f[1]=(v[0]+v[1])/hg;Ok(())})),None,true,None,None).unwrap();
 let c=Config{initial_basis:1,max_basis:1,..Config::default()};let r=coupled_step(&p,0.0,&[0.0;2],0.01,&c,&ParallelExecution::sequential());assert!(r.candidate.is_none());assert!(r.failure.is_some());
}
