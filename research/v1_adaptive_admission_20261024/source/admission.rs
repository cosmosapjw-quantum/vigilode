//! Research consumer only. Does not construct a production native RODAS hook.
//! Exact-scale target is relative to the supplied binary64 physical states.
use crate::integer_gate::{self,Certificate,Decision,THRESHOLD};
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Fault{Environment,InvalidTolerance,InvalidState,ScaleRange,Clock,Stale,Pending,NeedsPrecision,Budget,Finished}
#[derive(Clone,Copy,Debug)]pub struct Scales{pub lo:[f64;33],pub hi:[f64;33]}
#[derive(Clone,Copy,Debug)]pub struct Metric{pub atol:[f64;33],pub rtol:[f64;33]}
#[derive(Clone,Copy,Debug)]pub struct Assessment{pub decision:Decision,pub proposal_norm:f64,pub scales:Scales}
/// Read-only per-call guard. The only unsafe instruction reads MXCSR into a local.
/// It neither changes the environment nor proves compiler no-fast-math settings.
pub fn environment()->Result<(),Fault>{
 #[cfg(target_arch="x86_64")]{
  let mut csr=0u32;
  unsafe{std::arch::asm!("stmxcsr [{p}]",p=in(reg)&mut csr,options(nostack,preserves_flags));}
  if csr&0xe040!=0{return Err(Fault::Environment)} Ok(())
 }
 #[cfg(not(target_arch="x86_64"))]{Err(Fault::Environment)}
}
pub fn scales(old:&[f64;33],new:&[f64;33],m:&Metric)->Result<Scales,Fault>{
 environment()?;
 if old.iter().chain(new).any(|x|!x.is_finite()){return Err(Fault::InvalidState)}
 if m.atol.iter().any(|a|!a.is_finite()||*a<=0.)||m.rtol.iter().any(|r|!r.is_finite()||*r<0.){return Err(Fault::InvalidTolerance)}
 let mut out=Scales{lo:[0.;33],hi:[0.;33]};
 for i in 0..33{
  let a=m.atol[i];let r=m.rtol[i];let v=old[i].abs().max(new[i].abs());
  if r==0.||v==0.{out.lo[i]=a;out.hi[i]=a;continue}
  let p=r*v;
  let lo=(a+p.next_down().max(0.)).next_down().max(0.);
  let hi=(a+p.next_up()).next_up();
  if !(lo>0.&&lo.is_finite()&&hi.is_finite()){return Err(Fault::ScaleRange)}
  out.lo[i]=lo;out.hi[i]=hi;
 }
 Ok(out)
}
pub fn assess(error:&[f64;30],old:&[f64;33],new:&[f64;33],m:&Metric)->Result<Assessment,Fault>{
 let s=scales(old,new,m)?;
 let bits=error.map(f64::to_bits);
 let upper=integer_gate::evaluate(&bits,&s.lo.map(f64::to_bits)).map_err(|_|Fault::InvalidState)?;
 let decision=if upper.decision()==Decision::Accept{Decision::Accept}else{
  let lower=integer_gate::evaluate(&bits,&s.hi.map(f64::to_bits)).map_err(|_|Fault::InvalidState)?;
  if lower.decision()==Decision::Reject{Decision::Reject}else{Decision::Indeterminate}
 };
 // Heuristic proposal value only: conversion/sqrt cannot grant acceptance.
 let estimate=match upper{Certificate::LargeRow{..}=>8.,Certificate::Bound{hi,..}=>(hi as f64/THRESHOLD as f64).sqrt()};
 Ok(Assessment{decision,proposal_norm:estimate,scales:s})
}
/// Fixed research totals. This returned-state rounding is not enclosed by scales().
pub fn lift(z:&[f64;30])->Result<[f64;33],Fault>{
 environment()?;
 if z.iter().any(|v|!v.is_finite()){return Err(Fault::InvalidState)}
 let mut y=[0.;33];y[..23].copy_from_slice(&z[..23]);
 y[23]=1.-z[23];y[24]=z[23];y[25]=0.5-z[24]-z[25];y[26]=z[24];y[27]=z[25];
 y[28]=0.2-z[26]-z[27]-z[28];y[29]=z[26];y[30]=z[27];y[31]=z[28];y[32]=z[29];
 if y.iter().any(|v|!v.is_finite()){return Err(Fault::InvalidState)}Ok(y)
}
#[derive(Debug,Clone,Copy,PartialEq)]pub struct Accepted{pub t:f64,pub y:[f64;30],pub epoch:u64,pub last_norm:Option<f64>}
#[derive(Debug,Clone,Copy)]pub struct Lease{pub epoch:u64,pub id:u64,pub h:f64,pub next:f64}
pub struct Session{state:Accepted,tf:f64,h:f64,attempts:u64,accepted:u64,rejected:u64,cap:u64,pending:Option<Lease>,halt:Option<Fault>}
impl Session{
 pub fn new(t:f64,tf:f64,y:[f64;30],h:f64,cap:u64)->Result<Self,Fault>{
  environment()?;
  if !t.is_finite()||!tf.is_finite()||tf<t||!(tf-t).is_finite()||!h.is_finite()||h<=0.{return Err(Fault::Clock)}
  if y.iter().any(|v|!v.is_finite()){return Err(Fault::InvalidState)}
  Ok(Self{state:Accepted{t,y,epoch:0,last_norm:None},tf,h,attempts:0,accepted:0,rejected:0,cap,pending:None,halt:None})
 }
 pub fn snapshot(&self)->Accepted{self.state}
 pub fn attempts(&self)->u64{self.attempts}
 pub fn accepted(&self)->u64{self.accepted}
 pub fn rejected(&self)->u64{self.rejected}
 pub fn begin(&mut self)->Result<Lease,Fault>{self.begin_to(self.tf)}
 /// A yield boundary never creates a new session or resets the attempt budget.
 pub fn begin_to(&mut self,target:f64)->Result<Lease,Fault>{
  if self.state.t==self.tf{return Err(Fault::Finished)}
  if let Some(e)=self.halt{return Err(e)}
  if self.pending.is_some(){return Err(Fault::Pending)}
  if !target.is_finite()||target<=self.state.t||target>self.tf{return Err(Fault::Clock)}
  if self.attempts>=self.cap{self.halt=Some(Fault::Budget);return Err(Fault::Budget)}
  if let Err(e)=environment(){self.halt=Some(e);return Err(e)}
  let next=(self.state.t+self.h.min(target-self.state.t)).min(target);
  let h=next-self.state.t;
  if !next.is_finite()||next<=self.state.t||h<=0.{self.halt=Some(Fault::Clock);return Err(Fault::Clock)}
  self.attempts+=1;let lease=Lease{epoch:self.state.epoch,id:self.attempts,h,next};self.pending=Some(lease);Ok(lease)
 }
 pub fn finish(&mut self,lease:Lease,candidate:[f64;30],error:[f64;30],metric:&Metric)->Result<Assessment,Fault>{
  let pending=self.pending.ok_or(Fault::Stale)?;
  if lease.epoch!=pending.epoch||lease.id!=pending.id||lease.h.to_bits()!=pending.h.to_bits()||lease.next.to_bits()!=pending.next.to_bits(){return Err(Fault::Stale)}
  let result=(||{let old=lift(&self.state.y)?;let new=lift(&candidate)?;assess(&error,&old,&new,metric)})();
  self.pending=None;
  let a=match result{Ok(a)=>a,Err(e)=>{self.halt=Some(e);return Err(e)}};
  match a.decision{
   Decision::Accept=>{self.state=Accepted{t:lease.next,y:candidate,epoch:self.state.epoch+1,last_norm:Some(a.proposal_norm)};self.accepted+=1;self.h=propose(lease.h,a.proposal_norm,true).min(self.tf-self.state.t);}
   Decision::Reject=>{self.rejected+=1;self.h=propose(lease.h,a.proposal_norm,false);}
   Decision::Indeterminate=>{self.halt=Some(Fault::NeedsPrecision);}
  }
  Ok(a)
 }
}
/// Deliberately nonauthoritative, simple I proposal, not a stability theorem.
pub fn propose(h:f64,e:f64,accepted:bool)->f64{
 let factor=if e==0.{3.}else{0.9*e.powf(-0.2)};
 h*if accepted{factor.clamp(0.2,3.)}else{factor.clamp(0.1,0.8)}
}
