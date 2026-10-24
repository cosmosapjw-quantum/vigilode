#[path="integer_gate.rs"] mod integer_gate;
#[path="domain.rs"] mod domain;
#[path="admission.rs"] mod admission;
#[path="trajectory.rs"] mod trajectory;
#[path="../intake/pr83/source/admission.rs"] mod baseline;
use admission::*;use trajectory::*;use integer_gate::Decision;
use std::io::{Write,BufWriter};use std::fs::File;
fn floats(w:&mut impl Write,x:&[f64]){for v in x{write!(w," {:016x}",v.to_bits()).unwrap();}}
fn main(){
 environment().unwrap();
 let mut traces=BufWriter::new(File::create("evidence/NEW_TRIALS.txt").unwrap());
 let mut outputs=BufWriter::new(File::create("evidence/NEW_STATES.txt").unwrap());
 let mut summary=BufWriter::new(File::create("evidence/NEW_RUNS.csv").unwrap());
 writeln!(summary,"model,tol,policy,accepted,rejected,domain_rejected,attempts,minimum_endpoint,max_output_error,legacy_parity_attempts").unwrap();
 let mut total=0;
 for model in 0..2 {for tol in [1e-5,1e-8] {for strict in [false,true] {
  let z0=initial();let m=Metric{atol:[tol*1e-3;33],rtol:[tol;33]};
  let bm=baseline::Metric{atol:m.atol,rtol:m.rtol};
  let mut old=baseline::Session::new(0.,0.8,z0,0.05,10000).unwrap();
  let mut s=if strict{Session::new_nonnegative(0.,0.8,z0,0.05,10000)}else{Session::new(0.,0.8,z0,0.05,10000)}.unwrap();
  let mut minimum=1.0f64;let mut error_max=0.0f64;let mut parity=0;
  for target in [0.1,0.2,0.4,0.8] {
   while s.snapshot().t<target {
    let before=s.snapshot();let l=s.begin_to(target).unwrap();
    let (candidate,e)=reduced_step(&before.y,l.h,4.*model as f64);
    let out=s.finish(l,candidate,e,&m).unwrap();
    assert_ne!(out.decision,Decision::Indeterminate);
    let after=s.snapshot();
    if !strict {
     let bl=old.begin_to(target).unwrap();
     assert_eq!(bl.h.to_bits(),l.h.to_bits());assert_eq!(bl.next.to_bits(),l.next.to_bits());
     let bo=old.finish(bl,candidate,e,&bm).unwrap();assert_eq!(bo.decision,out.decision);
     let os=old.snapshot();assert_eq!(os.t.to_bits(),after.t.to_bits());assert_eq!(os.y.map(f64::to_bits),after.y.map(f64::to_bits));
     assert_eq!(os.last_norm.map(f64::to_bits),after.last_norm.map(f64::to_bits));
     assert_eq!((old.accepted(),old.rejected(),old.attempts()),(s.accepted(),s.rejected(),s.attempts()));parity+=1;
    }
    if out.decision!=Decision::Accept {assert_eq!(before,after);}
    let p=lift(&candidate).unwrap();
    if out.decision==Decision::Accept {
     minimum=minimum.min(p.into_iter().fold(f64::INFINITY,f64::min));
     if strict {domain::check(&candidate,&p).unwrap();}
    }
    let tag=match out.decision{Decision::Accept=>"A",Decision::Reject=>"R",_=>"I"};
    let nt=match out.norm_decision{Decision::Accept=>"A",Decision::Reject=>"R",_=>"I"};
    write!(traces,"{model} {tol:.17e} {} {} {tag} {nt} {} {:016x} {:016x} {:016x}",u8::from(strict),l.id,u8::from(out.domain_violation.is_some()),before.t.to_bits(),l.h.to_bits(),after.t.to_bits()).unwrap();
    floats(&mut traces,&before.y);floats(&mut traces,&candidate);floats(&mut traces,&e);floats(&mut traces,&after.y);floats(&mut traces,&p);floats(&mut traces,&out.scales.lo);floats(&mut traces,&out.scales.hi);writeln!(traces).unwrap();
   }
   let snap=s.snapshot();let p=lift(&snap.y).unwrap();let exact=lift(&exact(&z0,snap.t,4.*model as f64)).unwrap();
   error_max=error_max.max(p.iter().zip(exact).map(|(a,b)|(a-b).abs()).fold(0f64,f64::max));
   write!(outputs,"{model} {tol:.17e} {} {:016x}",u8::from(strict),snap.t.to_bits()).unwrap();floats(&mut outputs,&p);writeln!(outputs).unwrap();
  }
  assert_eq!(s.snapshot().t,0.8);assert!(error_max<=20.*tol);
  if strict{assert!(minimum>=0.);}
  writeln!(summary,"{model},{tol:.17e},{},{},{},{},{},{minimum:.17e},{error_max:.17e},{parity}",u8::from(strict),s.accepted(),s.rejected(),s.domain_rejected(),s.attempts()).unwrap();
  total+=1;
 }}}
 // Direct compiled sign witness, not the old campaign.
 let mut y=[0.0;30];y[2]=1e-12;let (trial,e)=reduced_step(&y,8./rate(2),0.);
 let m=Metric{atol:[1e-8;33],rtol:[0.;33]};let a=assess(&e,&lift(&y).unwrap(),&lift(&trial).unwrap(),&m).unwrap();
 assert!(trial[2]<0.);assert_eq!(a.decision,Decision::Accept);assert!(domain::check(&trial,&lift(&trial).unwrap()).is_err());
 println!("PASS_NEW_TRAJECTORIES count={total} compiled_sign_witness={:.17e} embedded_proposal_norm={:.17e}",trial[2],a.proposal_norm);
}
