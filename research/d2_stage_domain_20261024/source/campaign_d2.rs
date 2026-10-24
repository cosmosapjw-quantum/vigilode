#[path="domain.rs"]mod domain;
#[path="integer_gate.rs"]mod integer_gate;
#[path="admission.rs"]mod admission;
#[path="trajectory.rs"]mod trajectory;
#[path="stage_safe.rs"]mod stage_safe;
use admission::*;use stage_safe::*;use integer_gate::Decision;
use std::fs::File;use std::io::{Write,BufWriter};
fn bits(w:&mut impl Write,a:&[f64]){for x in a{write!(w," {:016x}",x.to_bits()).unwrap();}}
fn main(){
 environment().unwrap();
 let mut stages=BufWriter::new(File::create("evidence/D2_STAGES.txt").unwrap());
 let mut trials=BufWriter::new(File::create("evidence/D2_TRIALS.txt").unwrap());
 let mut outputs=BufWriter::new(File::create("evidence/D2_OUTPUTS.txt").unwrap());
 let mut runs=BufWriter::new(File::create("evidence/D2_RUNS.csv").unwrap());
 writeln!(runs,"run,alpha,tol,arm,status,attempts,accepted,rejected,endpoint_domain_rejections,stage_aborts,rhs,solves,checked,bad_rhs,completed_step_parities").unwrap();
 let mut run=0;
 for alpha in[0.,4.]{for tol in[3e-6,3e-9]{for arm in 0..3{
  let mut s=Session::new_nonnegative(0.,0.6,trajectory::initial(),0.04,20000).unwrap();
  let m=Metric{atol:[tol*1e-3;33],rtol:[tol;33]};
  let(mut aborted,mut total_rhs,mut total_solves,mut total_checked,mut bad_rhs,mut parities)=(0,0,0,0,0,0);
  let mut status="COMPLETE";
  'landing:for target in[0.075,0.15,0.3,0.6]{
   while s.snapshot().t<target{
    let old=s.snapshot();
    if arm==2{let max_rate=(0..30).map(|j|trajectory::rate(j)+2.*alpha*old.y[j]).fold(0f64,f64::max);s.cap_next_h(1.8/max_rate).unwrap();}
    let l=match s.begin_to(target){Ok(l)=>l,Err(_)=>{status="BEGIN_REFUSAL";break 'landing}};
    let mut bad=0;
    let result=attempt(&old.y,l.h,alpha,arm!=0,&mut|_,y,f|{
     let inside=domain::check(y,&lift(y).unwrap()).is_ok();
     if !inside{bad+=1;if arm!=0{return Err(CallbackFault::Fatal)}}
     for j in 0..30{f[j]=-trajectory::rate(j)*y[j]-alpha*y[j]*y[j];}Ok(())
    },&mut|stage,y,inside,called|{
     write!(stages,"{run} {} {stage} {} {}",l.id,u8::from(inside),u8::from(called)).unwrap();
     bits(&mut stages,y);bits(&mut stages,&lift(y).unwrap());writeln!(stages).unwrap();
    });
    bad_rhs+=bad;
    let mut cand=[0.;30];let mut error=[0.;30];let mut slo=[0.;33];let mut shi=[0.;33];
    let (tag,work)=match result{
     Ok(r)=>{
      // Differential check on actual new inputs; no separate old campaign replay.
      let(a,e)=trajectory::reduced_step(&old.y,l.h,alpha);
      assert_eq!(a.map(f64::to_bits),r.candidate.map(f64::to_bits));assert_eq!(e.map(f64::to_bits),r.error.map(f64::to_bits));parities+=1;
      cand=r.candidate;error=r.error;
      let out=s.finish(l,cand,error,&m).unwrap();slo=out.scales.lo;shi=out.scales.hi;
      (match out.decision{Decision::Accept=>"A",Decision::Reject=>"R",Decision::Indeterminate=>"I"},r.work)
     },
     Err(f)=>{
      let recover=matches!(f.kind,FailureKind::StageDomain|FailureKind::Callback(CallbackFault::Recoverable));
      s.abort_stage(l,recover).unwrap();aborted+=1;
      (if recover{"S"}else{"F"},f.work)
     }
    };
    total_rhs+=work.rhs;total_solves+=work.solves;total_checked+=work.checked;
    let after=s.snapshot();if tag!="A"{assert_eq!(after,old);}else{domain::check(&after.y,&lift(&after.y).unwrap()).unwrap();}
    write!(trials,"{run} {} {tag} {} {} {} {:016x} {:016x} {:016x}",l.id,work.rhs,work.solves,work.checked,old.t.to_bits(),l.h.to_bits(),after.t.to_bits()).unwrap();
    for v in[&old.y[..],&after.y[..],&cand[..],&error[..],&slo[..],&shi[..]]{bits(&mut trials,v);}writeln!(trials).unwrap();
    if tag=="I"||tag=="F"{status="TRIAL_HOLD";break 'landing}
   }
   write!(outputs,"{run} {:016x}",s.snapshot().t.to_bits()).unwrap();bits(&mut outputs,&lift(&s.snapshot().y).unwrap());writeln!(outputs).unwrap();
  }
  if arm!=0{assert_eq!(bad_rhs,0);}
  writeln!(runs,"{run},{alpha},{tol:.17e},{arm},{status},{},{},{},{},{aborted},{total_rhs},{total_solves},{total_checked},{bad_rhs},{parities}",s.attempts(),s.accepted(),s.rejected(),s.domain_rejected()).unwrap();
  println!("run={run} alpha={alpha} tol={tol} arm={arm} status={status} attempts={} stage_abort={aborted} rhs={total_rhs} bad_rhs={bad_rhs}",s.attempts());run+=1;
 }}}
 println!("FINISHED_D2_CONFIGURATIONS={run}");
}
