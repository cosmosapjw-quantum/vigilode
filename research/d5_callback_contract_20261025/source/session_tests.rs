//! New D5 composition tests using the unchanged Session. No ODE accuracy claim.
#![forbid(unsafe_code)]
extern crate d3_fixture;
#[path="gateway.rs"] mod gateway;
use gateway::{BoxDomain,CallbackFault,Fault,Gate,RuntimeWitness};
use d3_fixture::admission::{Session,Metric,Fault as SessionFault};
use std::cell::Cell;

fn state()->[f64;30]{let mut z=[0.0;30];z[0]=0.25;z[1]=0.75;z}
fn gate()->Gate<2>{Gate::new(BoxDomain::new(0.0,1.0,[0.0;2],[1.0;2]).unwrap(),7)}
fn probe()->RuntimeWitness{RuntimeWitness{epoch:7,environment_ok:d3_fixture::admission::environment().is_ok()}}
fn rhs(_:f64,z:&[f64;2],out:&mut[f64;2])->Result<(),CallbackFault>{*out=[-z[0]+2.0*z[1],z[0]-2.0*z[1]];Ok(())}
fn jac(_:f64,_:&[f64;2],out:&mut[[f64;2];2])->Result<(),CallbackFault>{*out=[[-1.0,2.0],[1.0,-2.0]];Ok(())}

#[test]
fn recoverable_pair_failure_preserves_prefix_and_reuses_original_attempt_cap(){
 let mut s=Session::new_nonnegative(0.0,1.0,state(),0.125,3).unwrap();let before=s.snapshot();
 let mut g=gate();let effects=Cell::new(0_u64);
 for attempt in 1..=3{
  let lease=s.begin().unwrap();assert_eq!(lease.id,attempt);
  let result=g.prepare_pair(before.t,&[before.y[0],before.y[1]],&mut probe,
   &mut|t,z,out|{effects.set(effects.get()+1);rhs(t,z,out)},
   &mut|_,_,out|{effects.set(effects.get()+1);out[0][0]=91.0;Err(CallbackFault::Recoverable)});
  assert_eq!(result,Err(Fault::RecoverableCallback));s.abort_stage(lease,true).unwrap();
  assert_eq!(s.snapshot(),before);assert_eq!(s.attempts(),attempt);
 }
 assert!(matches!(s.begin(),Err(SessionFault::Budget)));
 assert_eq!((s.attempts(),s.accepted(),s.rejected()),(3,0,3));
 assert_eq!((g.counters().rhs,g.counters().jac,g.counters().pairs),(3,3,0));
 assert_eq!(effects.get(),6);assert_eq!(s.snapshot(),before);
}

#[test]
fn fatal_callback_halts_same_session_and_blocks_later_gate_dispatch(){
 let mut s=Session::new_nonnegative(0.0,1.0,state(),0.125,9).unwrap();let before=s.snapshot();
 let lease=s.begin().unwrap();let mut g=gate();let calls=Cell::new(0);
 assert_eq!(g.eval_rhs(0.0,&[0.25,0.75],&mut probe,&mut|_,_,out|{calls.set(calls.get()+1);out[0]=3.0;Err(CallbackFault::Fatal)}),Err(Fault::FatalCallback));
 s.abort_stage(lease,false).unwrap();assert_eq!(s.snapshot(),before);
 assert!(matches!(s.begin(),Err(SessionFault::CallbackFatal)));assert_eq!(s.attempts(),1);
 assert_eq!(g.eval_rhs(0.0,&[0.25,0.75],&mut probe,&mut rhs),Err(Fault::FatalCallback));
 assert_eq!(calls.get(),1);assert_eq!(g.counters().rhs,1);
}

#[test]
fn successful_pair_does_not_itself_advance_or_accept_a_solver_state(){
 let mut s=Session::new_nonnegative(0.0,1.0,state(),0.125,2).unwrap();let before=s.snapshot();
 let lease=s.begin().unwrap();let mut g=gate();
 let p=g.prepare_pair(0.0,&[0.25,0.75],&mut probe,&mut rhs,&mut jac).unwrap();
 assert_eq!(p.rhs,[1.25,-1.25]);assert_eq!(s.snapshot(),before);assert_eq!(s.accepted(),0);
 s.abort_stage(lease,true).unwrap();assert_eq!(s.snapshot(),before);assert_eq!(s.rejected(),1);
}

#[test]
fn existing_nonzero_accepted_prefix_survives_a_later_callback_refusal(){
 let mut s=Session::new_nonnegative(0.0,1.0,state(),0.125,3).unwrap();
 // Synthetic admission fixture only: unchanged candidate and exact zero defect.
 // It is not asserted to integrate the reversible-exchange RHS.
 let lease=s.begin().unwrap();s.finish(lease,state(),[0.0;30],&Metric{atol:[1.0;33],rtol:[0.0;33]}).unwrap();
 let before=s.snapshot();assert_eq!(before.epoch,1);assert_eq!(before.t,0.125);
 let lease=s.begin().unwrap();let mut g=gate();
 assert_eq!(g.prepare_pair(before.t,&[before.y[0],before.y[1]],&mut probe,&mut rhs,
  &mut|_,_,out|{out[0][0]=1.0;Ok(())}),Err(Fault::NonFiniteOutput));
 s.abort_stage(lease,false).unwrap();assert_eq!(s.snapshot(),before);assert_eq!(s.accepted(),1);
 assert_eq!(s.attempts(),2);assert!(matches!(s.begin(),Err(SessionFault::CallbackFatal)));
}

#[test]
fn repeated_domain_refusal_consumes_session_attempts_but_zero_user_callbacks(){
 let mut s=Session::new_nonnegative(0.0,1.0,state(),0.125,2).unwrap();let before=s.snapshot();let mut g=gate();
 for _ in 0..2{let lease=s.begin().unwrap();assert_eq!(g.prepare_pair(2.0,&[0.25,0.75],&mut probe,&mut rhs,&mut jac),Err(Fault::OutsideDomain));s.abort_stage(lease,true).unwrap();}
 assert!(matches!(s.begin(),Err(SessionFault::Budget)));assert_eq!(s.snapshot(),before);
 assert_eq!((s.attempts(),s.rejected(),g.counters().rhs,g.counters().jac),(2,2,0,0));
}
