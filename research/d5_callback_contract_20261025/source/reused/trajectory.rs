//! Standalone autonomous diagonal/Riccati RODAS5P realization, not native crate.
#[path="table.rs"]mod table;
use table::*;
pub fn retained(j:usize)->usize{match j{0..=22=>j,23=>24,24..=25=>j+2,26..=28=>j+3,_=>32}}
pub fn rate(j:usize)->f64{[1.,20.,300.][j%3]}
pub fn initial()->[f64;30]{let mut z=[0.;30];for j in 0..23{z[j]=0.1+0.03*(j%5)as f64;}z[23]=0.3;z[24]=0.2;z[25]=0.1;z[26]=0.05;z[27]=0.03;z[28]=0.02;z[29]=0.7;z}
pub fn exact(z0:&[f64;30],t:f64,alpha:f64)->[f64;30]{std::array::from_fn(|j|{let l=rate(j);let e=(-l*t).exp();z0[j]*e/(1.+alpha*z0[j]*(-(-l*t).exp_m1())/l)})}
pub fn reduced_step(z:&[f64;30],h:f64,alpha:f64)->([f64;30],[f64;30]){
 let mut stages=[[0.;30];8];let w:[f64;30]=std::array::from_fn(|j|1./(h*GAMMA)+rate(j)+2.*alpha*z[j]);
 for i in 0..8{let mut y=*z;
  for k in 0..i{if A[i][k]!=0.{for j in 0..30{y[j]+=A[i][k]*stages[k][j];}}}
  for j in 0..30{stages[i][j]=-rate(j)*y[j]-alpha*y[j]*y[j];}
  for k in 0..i{let c=C[i][k]/h;if c!=0.{for j in 0..30{stages[i][j]+=c*stages[k][j];}}}
  for j in 0..30{stages[i][j]/=w[j];}
 }
 let mut out=*z;for i in 0..8{if B[i]!=0.{for j in 0..30{out[j]+=B[i]*stages[i][j];}}}(out,stages[7])
}
/// Independent full33 assembly: includes dependent RHS and shifted triangular rows.
/// No reconstruction from the reduced endpoint is used to propagate dependent rows.
pub fn full_step(y:&[f64;33],h:f64,alpha:f64)->([f64;33],[f64;33]){
 let ihg=1./(h*GAMMA);let jac:[f64;30]=std::array::from_fn(|j|-rate(j)-2.*alpha*y[retained(j)]);
 let mut stages=[[0.;33];8];
 for i in 0..8{
  let state:[f64;33]=std::array::from_fn(|j|{let mut v=y[j];for k in 0..i{if A[i][k]!=0.{v+=A[i][k]*stages[k][j];}}v});
  let mut rhs=[0.;33];for j in 0..30{let p=retained(j);rhs[p]=-rate(j)*state[p]-alpha*state[p]*state[p];}
  rhs[23]=-rhs[24];rhs[25]=-rhs[26]-rhs[27];rhs[28]=-rhs[29]-rhs[30]-rhs[31];
  for j in 0..33{for k in 0..i{if C[i][k]!=0.{rhs[j]+=(C[i][k]/h)*stages[k][j];}}}
  let mut solution=[0.;33];for j in 0..30{let p=retained(j);solution[p]=rhs[p]/(ihg-jac[j]);}
  for (dep,start,count) in[(23,23,1),(25,24,2),(28,26,3)]{let mut v=rhs[dep];for j in start..start+count{v-=jac[j]*solution[retained(j)];}solution[dep]=v/ihg;}
  stages[i]=solution;
 }
 let out=std::array::from_fn(|j|{let mut v=y[j];for i in 0..8{if B[i]!=0.{v+=B[i]*stages[i][j];}}v});(out,stages[7])
}
