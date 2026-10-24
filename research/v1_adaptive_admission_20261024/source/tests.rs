#[path="integer_gate.rs"]mod integer_gate;
#[path="admission.rs"]mod admission;
use admission::*;use integer_gate::Decision;
fn metric()->Metric{Metric{atol:[1.;33],rtol:[0.;33]}}
fn session(cap:u64)->Session{Session::new(0.,1.,[0.01;30],0.1,cap).unwrap()}
#[test]fn offset_absent_from_error(){let a=assess(&[0.;30],&[4.;33],&[9.;33],&metric()).unwrap();assert_eq!(a.decision,Decision::Accept);assert_eq!(a.proposal_norm,0.);}
#[test]fn correlation(){let mut e=[0.;30];e[24]=2.5;e[25]=2.5;assert_eq!(assess(&e,&[0.;33],&[0.;33],&metric()).unwrap().decision,Decision::Reject);e[25]=-2.5;assert_eq!(assess(&e,&[0.;33],&[0.;33],&metric()).unwrap().decision,Decision::Accept);}
#[test]fn denominator33(){let mut e=[0.;30];e[0]=5.5;assert_eq!(assess(&e,&[0.;33],&[0.;33],&metric()).unwrap().decision,Decision::Accept);}
#[test]fn scale_encloses_easy_value(){let m=Metric{atol:[0.25;33],rtol:[0.5;33]};let s=scales(&[2.;33],&[3.;33],&m).unwrap();assert!(s.lo[0]<=1.75&&s.hi[0]>=1.75);}
#[test]fn exact_absolute_scale(){let s=scales(&[1e308;33],&[1e308;33],&metric()).unwrap();assert_eq!(s.lo,[1.;33]);assert_eq!(s.hi,[1.;33]);}
#[test]fn overflowing_scale_refuses(){let m=Metric{atol:[1e308;33],rtol:[1.;33]};assert!(matches!(scales(&[1e308;33],&[1e308;33],&m),Err(Fault::ScaleRange)));}
#[test]fn invalid_tail_refuses(){let mut m=metric();m.atol[32]=0.;assert!(scales(&[0.;33],&[0.;33],&m).is_err());m=metric();m.rtol[32]=f64::NAN;assert!(scales(&[0.;33],&[0.;33],&m).is_err());}
#[test]fn nonfinite_state_refuses(){let mut y=[0.;33];y[32]=f64::INFINITY;assert!(scales(&y,&[0.;33],&metric()).is_err());}
#[test]fn fixed_lift_conservation(){let z=[0.01;30];let y=lift(&z).unwrap();assert!((y[23]+y[24]-1.).abs()<1e-15);assert!((y[25]+y[26]+y[27]-0.5).abs()<1e-15);assert!((y[28]+y[29]+y[30]+y[31]-0.2).abs()<1e-15);}
#[test]fn raw_lift_overflow_error_survives(){let mut e=[0.;30];e[24]=1e308;e[25]=1e308;let m=Metric{atol:[1e308;33],rtol:[0.;33]};assert_eq!(assess(&e,&[0.;33],&[0.;33],&m).unwrap().decision,Decision::Accept);}
#[test]fn accepted_commit_and_replay(){let mut s=session(4);let l=s.begin().unwrap();let a=s.finish(l,[0.02;30],[0.;30],&metric()).unwrap();assert_eq!(a.decision,Decision::Accept);assert_eq!(s.snapshot().t,0.1);assert_eq!(s.snapshot().epoch,1);let before=s.snapshot();assert!(matches!(s.finish(l,[0.03;30],[0.;30],&metric()),Err(Fault::Stale)));assert_eq!(s.snapshot(),before);}
#[test]fn rejected_preserves_accepted_history(){let mut s=session(3);let before=s.snapshot();let l=s.begin().unwrap();assert_eq!(s.finish(l,[0.02;30],[10.;30],&metric()).unwrap().decision,Decision::Reject);assert_eq!(s.snapshot(),before);assert_eq!(s.attempts(),1);assert_eq!(s.rejected(),1);assert!(s.begin().unwrap().h<l.h);}
#[test]fn failure_is_sticky_no_commit(){let mut s=session(3);let before=s.snapshot();let l=s.begin().unwrap();let mut m=metric();m.atol[32]=0.;assert!(s.finish(l,[0.02;30],[0.;30],&m).is_err());assert_eq!(s.snapshot(),before);assert!(s.begin().is_err());assert_eq!(s.attempts(),1);}
#[test]fn pending_attempt_cannot_be_replaced(){let mut s=session(4);let _=s.begin().unwrap();assert!(matches!(s.begin(),Err(Fault::Pending)));assert_eq!(s.attempts(),1);}
#[test]fn stale_lease_does_not_kill_live_one(){let mut s=session(4);let l=s.begin().unwrap();let mut bad=l;bad.epoch+=1;assert!(matches!(s.finish(bad,[0.02;30],[0.;30],&metric()),Err(Fault::Stale)));assert_eq!(s.finish(l,[0.02;30],[0.;30],&metric()).unwrap().decision,Decision::Accept);}
#[test]fn budget_persists_across_quota(){let mut s=session(2);for _ in 0..2{let l=s.begin().unwrap();s.finish(l,[0.02;30],[0.;30],&metric()).unwrap();}let before=s.snapshot();assert!(matches!(s.begin(),Err(Fault::Budget)));assert!(matches!(s.begin(),Err(Fault::Budget)));assert_eq!(s.snapshot(),before);assert_eq!(s.attempts(),2);}
#[test]fn indeterminate_holds_without_retry(){let mut e=[0.;30];e[..21].fill(1.);e[21]=f64::from_bits(1);e[23]=1.;e[24]=1.;e[26]=2.;let mut s=session(3);let before=s.snapshot();let l=s.begin().unwrap();assert_eq!(s.finish(l,[0.02;30],e,&metric()).unwrap().decision,Decision::Indeterminate);assert_eq!(s.snapshot(),before);assert!(matches!(s.begin(),Err(Fault::NeedsPrecision)));assert_eq!(s.attempts(),1);}
#[test]fn unrepresented_clock_does_not_charge_work(){let mut s=Session::new(1e20,1e20+1e6,[0.;30],1.,4).unwrap();assert!(matches!(s.begin(),Err(Fault::Clock)));assert_eq!(s.attempts(),0);}
#[test]fn final_arrival_wins_over_spent_budget(){let mut s=Session::new(0.,0.1,[0.01;30],0.1,1).unwrap();let l=s.begin().unwrap();s.finish(l,[0.02;30],[0.;30],&metric()).unwrap();assert!(matches!(s.begin(),Err(Fault::Finished)));}

use std::alloc::{GlobalAlloc,Layout,System};use std::sync::atomic::{AtomicUsize,Ordering};
struct Count;static ALLOC:AtomicUsize=AtomicUsize::new(0);
unsafe impl GlobalAlloc for Count{unsafe fn alloc(&self,l:Layout)->*mut u8{ALLOC.fetch_add(1,Ordering::Relaxed);unsafe{System.alloc(l)}}unsafe fn dealloc(&self,p:*mut u8,l:Layout){unsafe{System.dealloc(p,l)}}unsafe fn realloc(&self,p:*mut u8,l:Layout,n:usize)->*mut u8{ALLOC.fetch_add(1,Ordering::Relaxed);unsafe{System.realloc(p,l,n)}}}
#[global_allocator]static ALLOCATOR:Count=Count;
#[test]fn admission_hot_path_does_not_allocate(){let e=[0.001;30];let y=[1.;33];let m=Metric{atol:[0.1;33],rtol:[0.01;33]};let before=ALLOC.load(Ordering::Relaxed);for _ in 0..128{std::hint::black_box(assess(std::hint::black_box(&e),std::hint::black_box(&y),std::hint::black_box(&y),std::hint::black_box(&m)).unwrap());}assert_eq!(ALLOC.load(Ordering::Relaxed)-before,0);}
#[test]fn forged_step_size_lease_refuses_without_commit(){let mut s=session(4);let l=s.begin().unwrap();let before=s.snapshot();let mut wrong=l;wrong.h*=2.;assert!(matches!(s.finish(wrong,[0.02;30],[0.;30],&metric()),Err(Fault::Stale)));assert_eq!(s.snapshot(),before);s.finish(l,[0.02;30],[0.;30],&metric()).unwrap();}
