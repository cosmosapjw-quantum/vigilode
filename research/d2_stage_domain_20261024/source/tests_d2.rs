#[path="domain.rs"]mod domain;
#[path="integer_gate.rs"]mod integer_gate;
#[path="admission.rs"]mod admission;
#[path="trajectory.rs"]mod trajectory;
#[path="stage_safe.rs"]mod stage_safe;
use admission::*;use stage_safe::*;
fn rhs(_:usize,y:&[f64;30],out:&mut[f64;30])->Result<(),CallbackFault>{for j in 0..30{out[j]=-trajectory::rate(j)*y[j];}Ok(())}
fn visit(_:usize,_:&[f64;30],_:bool,_:bool){}
fn session(cap:u64)->Session{Session::new_nonnegative(0.,0.5,trajectory::initial(),0.04,cap).unwrap()}
#[test]fn unsafe_stage_is_blocked_before_callback(){let mut z=[0.;30];z[2]=1e-12;let mut calls=0;let r=attempt(&z,4./300.,0.,true,&mut|i,y,o|{calls+=1;assert!(domain::check(y,&lift(y).unwrap()).is_ok());rhs(i,y,o)},&mut visit).unwrap_err();assert_eq!(r.kind,FailureKind::StageDomain);assert_eq!(r.stage,1);assert_eq!(r.work.rhs,1);assert_eq!(r.work.solves,1);assert_eq!(calls,1);}
#[test]fn endpoint_can_hide_negative_stage(){let mut z=[0.;30];z[2]=1e-12;let mut bad=0;let r=attempt(&z,4./300.,0.,false,&mut rhs,&mut|_,_,inside,called|{if called&&!inside{bad+=1}}).unwrap();assert!(r.candidate[2]>0.);assert!(bad>0);assert_eq!(r.work.rhs,8);}
#[test]fn successful_arithmetic_matches_pinned_step(){for alpha in[0.,4.]{for h in[1e-5,1e-3]{let z=trajectory::initial();let r=attempt(&z,h,alpha,true,&mut|_,y,o|{for j in 0..30{o[j]=-trajectory::rate(j)*y[j]-alpha*y[j]*y[j];}Ok(())},&mut visit).unwrap();let(a,e)=trajectory::reduced_step(&z,h,alpha);assert_eq!(a.map(f64::to_bits),r.candidate.map(f64::to_bits));assert_eq!(e.map(f64::to_bits),r.error.map(f64::to_bits));}}}
#[test]fn invalid_initial_never_calls_rhs(){let mut z=[0.;30];z[29]=f64::NAN;let mut calls=0;assert_eq!(attempt(&z,0.001,0.,true,&mut|_,_,_|{calls+=1;Ok(())},&mut visit).unwrap_err().kind,FailureKind::InvalidInput);assert_eq!(calls,0);}
#[test]fn initial_domain_is_checked(){let mut z=[0.;30];z[0]=-1.;let r=attempt(&z,0.001,0.,true,&mut rhs,&mut visit).unwrap_err();assert_eq!(r.kind,FailureKind::StageDomain);assert_eq!(r.work.rhs,0);}
#[test]fn incomplete_rhs_is_typed_failure(){let r=attempt(&[0.;30],0.001,0.,true,&mut|_,_,_|Ok(()),&mut visit).unwrap_err();assert_eq!(r.kind,FailureKind::NonFinite);assert_eq!(r.work.rhs,1);assert_eq!(r.work.solves,0);}
#[test]fn fatal_rhs_stops_immediately(){let r=attempt(&[0.;30],0.001,0.,true,&mut|_,_,_|Err(CallbackFault::Fatal),&mut visit).unwrap_err();assert_eq!(r.kind,FailureKind::Callback(CallbackFault::Fatal));assert_eq!(r.work.rhs,1);}
#[test]fn recoverable_rhs_keeps_its_kind(){let r=attempt(&[0.;30],0.001,0.,true,&mut|_,_,_|Err(CallbackFault::Recoverable),&mut visit).unwrap_err();assert_eq!(r.kind,FailureKind::Callback(CallbackFault::Recoverable));}
#[test]fn abort_preserves_accepted_and_charges_once(){let mut s=session(2);let old=s.snapshot();let l=s.begin().unwrap();s.abort_stage(l,true).unwrap();assert_eq!(s.snapshot(),old);assert_eq!(s.attempts(),1);assert_eq!(s.rejected(),1);let l2=s.begin().unwrap();assert_eq!(l2.h,0.02);s.abort_stage(l2,true).unwrap();assert_eq!(s.begin().unwrap_err(),Fault::Budget);assert_eq!(s.attempts(),2);}
#[test]fn fatal_abort_is_sticky(){let mut s=session(4);let old=s.snapshot();let l=s.begin().unwrap();s.abort_stage(l,false).unwrap();assert_eq!(s.snapshot(),old);assert_eq!(s.begin().unwrap_err(),Fault::CallbackFatal);}
#[test]fn stale_abort_does_not_cancel_current_lease(){let mut s=session(4);let l=s.begin().unwrap();let mut wrong=l;wrong.id+=1;assert_eq!(s.abort_stage(wrong,true),Err(Fault::Stale));s.abort_stage(l,true).unwrap();assert_eq!(s.abort_stage(l,true),Err(Fault::Stale));}
#[test]fn cap_is_only_a_proposal_not_reset(){let mut s=session(2);let old=s.snapshot();assert!(s.cap_next_h(f64::NAN).is_err());s.cap_next_h(0.003).unwrap();assert_eq!(s.snapshot(),old);assert_eq!(s.attempts(),0);let l=s.begin().unwrap();assert_eq!(l.h,0.003);assert_eq!(s.cap_next_h(0.001),Err(Fault::Pending));s.abort_stage(l,true).unwrap();s.cap_next_h(0.01).unwrap();assert!(s.begin().unwrap().h<0.003);}
#[test]fn partial_callback_failure_reports_spent_work(){let r=attempt(&trajectory::initial(),0.0001,0.,true,&mut|i,y,o|{if i==2{Err(CallbackFault::Fatal)}else{rhs(i,y,o)}},&mut visit).unwrap_err();assert_eq!(r.stage,2);assert_eq!(r.work.rhs,3);assert_eq!(r.work.solves,2);assert_eq!(r.kind,FailureKind::Callback(CallbackFault::Fatal));}
#[test]fn step_cap_does_not_override_norm_or_domain(){let mut s=session(4);s.cap_next_h(0.001).unwrap();let old=s.snapshot();let l=s.begin().unwrap();let m=Metric{atol:[1.;33],rtol:[0.;33]};assert_eq!(s.finish(l,old.y,[100.;30],&m).unwrap().decision,integer_gate::Decision::Reject);assert_eq!(s.snapshot(),old);}
use std::alloc::{GlobalAlloc,Layout,System};use std::sync::atomic::{AtomicUsize,Ordering};
struct Counter;static ALLOC:AtomicUsize=AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counter{unsafe fn alloc(&self,l:Layout)->*mut u8{ALLOC.fetch_add(1,Ordering::Relaxed);unsafe{System.alloc(l)}}unsafe fn dealloc(&self,p:*mut u8,l:Layout){unsafe{System.dealloc(p,l)}}unsafe fn realloc(&self,p:*mut u8,l:Layout,n:usize)->*mut u8{ALLOC.fetch_add(1,Ordering::Relaxed);unsafe{System.realloc(p,l,n)}}}
#[global_allocator]static COUNT:Counter=Counter;
#[test]fn guarded_attempt_without_io_has_zero_heap_requests(){let z=trajectory::initial();let before=ALLOC.load(Ordering::Relaxed);for _ in 0..128{std::hint::black_box(attempt(std::hint::black_box(&z),0.0001,0.,true,&mut rhs,&mut visit).unwrap());}assert_eq!(ALLOC.load(Ordering::Relaxed)-before,0);}
