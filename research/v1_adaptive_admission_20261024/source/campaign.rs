#[path="integer_gate.rs"]mod integer_gate;
#[path="admission.rs"]mod admission;
#[path="trajectory.rs"]mod trajectory;
use admission::*;use trajectory::*;use integer_gate::Decision;
use std::io::{Write,BufWriter};use std::fs::File;
fn norm<const N:usize>(e:&[f64;N],old:&[f64;N],new:&[f64;N],tol:f64)->f64{(e.iter().zip(old).zip(new).map(|((&x,&a),&b)|(x/(tol*1e-3+tol*a.abs().max(b.abs()))).powi(2)).sum::<f64>()/N as f64).sqrt()}
fn write_state(out:&mut impl Write,model:usize,tol:f64,arm:usize,t:f64,y:&[f64;33]){write!(out,"{model},{tol:.17e},{arm},{t:.17e}").unwrap();for v in y{write!(out,",{v:.17e}").unwrap();}writeln!(out).unwrap();}
fn main(){
 environment().unwrap();let mut states=BufWriter::new(File::create("evidence/STATES.csv").unwrap());let mut trials=BufWriter::new(File::create("evidence/TRIALS_BITS.txt").unwrap());
 let mut summary=BufWriter::new(File::create("evidence/RUNS.csv").unwrap());writeln!(summary,"model,tol,arm,accepted,rejected,attempts,max_absolute_error,max_scaled_error,nanoseconds").unwrap();
 let z0=initial();
 for model in 0..2{let alpha=4.*model as f64;for tol in [1e-4,1e-7,1e-9]{for arm in 0..3{
  let m=Metric{atol:[tol*1e-3;33],rtol:[tol;33]};let start=std::time::Instant::now();
  let(mut accepted,mut rejected,mut attempts)=(0u64,0u64,0u64);let(mut max_abs,mut max_scaled)=(0f64,0f64);
  let mut session=Session::new(0.,1.,z0,0.05,10000).unwrap();let(mut t,mut h)=(0.,0.05f64);let mut z=z0;let mut full=lift(&z0).unwrap();
  write_state(&mut states,model,tol,arm,0.,&full);
  for landing in 1..=20{let target=landing as f64/20.;
   while t<target{
    if arm==0{
     let before=session.snapshot();let lease=session.begin_to(target).unwrap();let(candidate,error)=reduced_step(&before.y,lease.h,alpha);let old=lift(&before.y).unwrap();let new=lift(&candidate).unwrap();
     let a=session.finish(lease,candidate,error,&m).unwrap();assert_ne!(a.decision,Decision::Indeterminate);
     let tag=if a.decision==Decision::Accept{"A"}else{"R"};
     write!(trials,"{model} {tol:.17e} {} {:.17e} {:.17e} {tag}",lease.id,before.t,lease.h).unwrap();
     for x in error.iter().chain(old.iter()).chain(new.iter()).chain(a.scales.lo.iter()).chain(a.scales.hi.iter()){write!(trials," {:016x}",x.to_bits()).unwrap();}writeln!(trials).unwrap();
     let after=session.snapshot();if tag=="R"{assert_eq!(before,after);}t=after.t;full=lift(&after.y).unwrap();accepted=session.accepted();rejected=session.rejected();attempts=session.attempts();
    }else{
     attempts+=1;assert!(attempts<=10000);let next=(t+h.min(target-t)).min(target);let dt=next-t;assert!(dt>0.);
     let estimate=if arm==1{let(candidate,e)=reduced_step(&z,dt,alpha);let err=norm(&e,&z,&candidate,tol);if err<=1.{z=candidate;full=lift(&z).unwrap();}err}else{let(candidate,e)=full_step(&full,dt,alpha);let err=norm(&e,&full,&candidate,tol);if err<=1.{full=candidate;}err};
     assert!(estimate.is_finite());let take=estimate<=1.;h=propose(dt,estimate,take);if take{accepted+=1;t=next;}else{rejected+=1;}
    }
   }
   let reference=lift(&exact(&z0,t,alpha)).unwrap();for i in 0..33{let err=(full[i]-reference[i]).abs();max_abs=max_abs.max(err);max_scaled=max_scaled.max(err/(tol*1e-3+tol*reference[i].abs()));}
   write_state(&mut states,model,tol,arm,t,&full);
  }
  let elapsed=start.elapsed().as_nanos();assert!(max_abs<=20.*tol);assert_eq!(t,1.);
  writeln!(summary,"{model},{tol:.17e},{arm},{accepted},{rejected},{attempts},{max_abs:.17e},{max_scaled:.17e},{elapsed}").unwrap();
 }}}
 let mut parity=BufWriter::new(File::create("evidence/STAGE_PARITY.csv").unwrap());writeln!(parity,"model,h,state_diff,error_diff").unwrap();
 for model in 0..2{for h in [1e-5,1e-3,0.05,0.2]{let(a,e)=reduced_step(&z0,h,4.*model as f64);let(b,f)=full_step(&lift(&z0).unwrap(),h,4.*model as f64);let a=lift(&a).unwrap();let mut le=[0.;33];for j in 0..30{le[retained(j)]=e[j];}le[23]=-e[23];le[25]=-e[24]-e[25];le[28]=-e[26]-e[27]-e[28];let d=a.iter().zip(b).map(|(x,y)|(x-y).abs()).fold(0f64,f64::max);let ed=le.iter().zip(f).map(|(x,y)|(x-y).abs()).fold(0f64,f64::max);assert!(d<1e-12&&ed<1e-12);writeln!(parity,"{model},{h:.17e},{d:.17e},{ed:.17e}").unwrap();}}
 let mut order=BufWriter::new(File::create("evidence/ORDER.csv").unwrap());writeln!(order,"steps,error").unwrap();for n in [4,8,16,32,64]{let mut y=[0.;30];y[0]=0.3;for _ in 0..n{y=reduced_step(&y,1./n as f64,4.).0;}let z=0.3*(-1f64).exp()/(1.+1.2*(1.-(-1f64).exp()));writeln!(order,"{n},{:.17e}",(y[0]-z).abs()).unwrap();}
 println!("PASS_18_NEW_TRAJECTORIES_AND_8_STAGE_PARITIES");
}
