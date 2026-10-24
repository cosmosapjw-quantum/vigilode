//! Fixed manufactured diagonal/Riccati RODAS step with pre-RHS stage checks.
//! Callback errors carry partial work; no state/clock commit occurs in this module.
//! A diagnostic unguarded mode is NOT a safe domain-restricted integration.
#![forbid(unsafe_code)]
#[path="table.rs"]mod table;
use table::*;
use crate::{admission,domain,trajectory};
#[derive(Debug,Clone,Copy,PartialEq,Eq)]pub enum CallbackFault{Recoverable,Fatal}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]pub enum FailureKind{StageDomain,Callback(CallbackFault),InvalidInput,NonFinite}
#[derive(Debug,Clone,Copy,Default,PartialEq,Eq)]pub struct Work{pub rhs:u64,pub solves:u64,pub checked:u64}
#[derive(Debug,Clone,Copy)]pub struct Failure{pub kind:FailureKind,pub stage:usize,pub work:Work}
#[derive(Debug,Clone,Copy)]pub struct Trial{pub candidate:[f64;30],pub error:[f64;30],pub work:Work}
/// `rhs` must represent the stated manufactured model; the frozen Jacobian below
/// is analytic for that model, not an arbitrary callback or Jacobian interface.
/// `visit` is diagnostic only; its external side effects are not rolled back.
pub fn attempt<F,V>(z:&[f64;30],h:f64,alpha:f64,guard:bool,rhs:&mut F,visit:&mut V)->Result<Trial,Failure>
where F:FnMut(usize,&[f64;30],&mut[f64;30])->Result<(),CallbackFault>,V:FnMut(usize,&[f64;30],bool,bool){
 let mut work=Work::default();
 if admission::environment().is_err() || z.iter().any(|v|!v.is_finite()) || !h.is_finite() || h<=0. || !alpha.is_finite() || alpha<0. {
  return Err(Failure{kind:FailureKind::InvalidInput,stage:0,work});
 }
 let w:[f64;30]=std::array::from_fn(|j|1./(h*GAMMA)+trajectory::rate(j)+2.*alpha*z[j]);
 if w.iter().any(|v|!v.is_finite()||*v<=0.){return Err(Failure{kind:FailureKind::NonFinite,stage:0,work});}
 let mut stages=[[0.;30];8];
 for i in 0..8 {
  let mut y=*z;
  for k in 0..i{if A[i][k]!=0.{for j in 0..30{y[j]+=A[i][k]*stages[k][j];}}}
  let physical=admission::lift(&y).map_err(|_|Failure{kind:FailureKind::NonFinite,stage:i,work})?;
  work.checked+=1;
  let inside=domain::check(&y,&physical).is_ok();
  let called=!guard||inside;
  visit(i,&y,inside,called);
  if !called{return Err(Failure{kind:FailureKind::StageDomain,stage:i,work});}
  let mut f=[f64::NAN;30];work.rhs+=1;
  if let Err(e)=rhs(i,&y,&mut f){return Err(Failure{kind:FailureKind::Callback(e),stage:i,work});}
  if f.iter().any(|v|!v.is_finite()){return Err(Failure{kind:FailureKind::NonFinite,stage:i,work});}
  stages[i]=f;
  for k in 0..i{let c=C[i][k]/h;if c!=0.{for j in 0..30{stages[i][j]+=c*stages[k][j];}}}
  for j in 0..30{stages[i][j]/=w[j];}
  work.solves+=1;
  if stages[i].iter().any(|v|!v.is_finite()){return Err(Failure{kind:FailureKind::NonFinite,stage:i,work});}
 }
 let mut candidate=*z;
 for i in 0..8{if B[i]!=0.{for j in 0..30{candidate[j]+=B[i]*stages[i][j];}}}
 if candidate.iter().any(|v|!v.is_finite()){return Err(Failure{kind:FailureKind::NonFinite,stage:8,work});}
 Ok(Trial{candidate,error:stages[7],work})
}
