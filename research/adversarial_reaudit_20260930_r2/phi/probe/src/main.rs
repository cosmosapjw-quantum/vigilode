use std::sync::Arc;
use rodas5p_core::{DenseMatrix, ClosureOperator, LinearOperator, WorkCounters, dense_fused_phi_action, dense_phi_combination};
use rodas5p_integrators::{krylov_phi_action, ExponentialKrylovConfig, fused_phi_action, fused_phi_linear_combination, FusedPhiTerm, FusedPhiKrylovConfig, FusedPhiPrefixSession};
use serde_json::json;
fn phi(z:f64,k:usize)->f64 {let mut t=1.;for j in 1..=k{t/=j as f64;}let mut s=t;for j in 1..120{t*=z/(k+j) as f64;s+=t;}s}
fn scalar(a:f64)->Arc<dyn LinearOperator>{Arc::new(ClosureOperator::new(1,move|x,y|{y[0]=a*x[0];Ok(())}))}
fn cfg()->FusedPhiKrylovConfig {FusedPhiKrylovConfig{relative_tolerance:1e-12,absolute_tolerance:0.,..Default::default()}}
fn emit(v:serde_json::Value){println!("{v}");}
fn scalar_row(id:&str,h:f64,expected:f64,r:Result<Vec<f64>,impl std::fmt::Display>){match r{Ok(v)=>emit(json!({"id":id,"h":h,"value":v[0],"expected":expected,"relative_error":(v[0]/expected-1.).abs(),"status":"Ok"})),Err(e)=>emit(json!({"id":id,"h":h,"expected":expected,"status":"Err","message":e.to_string()}))}}
// Research prototype: multiplication order keeps intermediate magnitudes
// between |b| and |h^k b|. It avoids eager formation of an unrepresentable h^k.
fn scaled_weights(h:f64,b:&[Vec<f64>])->Vec<Vec<f64>> {b.iter().enumerate().map(|(k,v)|v.iter().map(|x|{let mut w=*x;for _ in 0..k{w*=h;}w}).collect()).collect()}
// Research prototype: common power-of-two amplitude normalization, exploiting
// linearity in all w_k. It does not certify general nonnormal Padé forward error.
fn balanced_dense(a:&DenseMatrix,h:f64,w:&[Vec<f64>])->Result<Vec<f64>,rodas5p_core::CoreError>{
 let max=w.iter().flatten().map(|x|x.abs()).fold(0.,f64::max);
 let s=if max>0.{2f64.powi((max.log2().floor() as i32).clamp(-1022,1023))}else{1.};
 let ws=w.iter().map(|v|v.iter().map(|x|x/s).collect()).collect::<Vec<_>>();
 dense_phi_combination(a,h,&ws).map(|v|v.iter().map(|x|x*s).collect())
}
fn main(){
 let matrix=DenseMatrix::from_rows(&[&[-1.]]).unwrap();let v=[1.,0.5,2.,-1.,0.75];
 for with_b0 in [false,true]{for h in [0.,-0.1,-1e-12,1e-1,1e-4,1e-8,1e-12,1e-14,1e-20,1e-40,1e-70]{
  let first=if with_b0{0}else{1};let exact=(first..=4).map(|k|v[k]*phi(-h,k)).sum::<f64>();
  let terms=(first..=4).map(|k|FusedPhiTerm{coefficient:v[k],phi_index:k,vector:&[1.]}).collect::<Vec<_>>();
  let r=fused_phi_linear_combination(scalar(-1.),h,&terms,cfg(),&mut WorkCounters::default()).unwrap();
  emit(json!({"id":"old_p1_closure","h":h,"b0":with_b0,"value":r.value[0],"expected":exact,"relative_error":(r.value[0]/exact-1.).abs(),"converged":r.converged,"basis":r.convergence_basis}));
  let w=(0..=4).map(|k|vec![if k<first{0.}else{v[k]}]).collect::<Vec<_>>();scalar_row("old_p2_closure",h,exact,dense_phi_combination(&matrix,h,&w));
  if h!=0. {let b=(0..=4).map(|k|vec![w[k][0]/h.powi(k as i32)]).collect::<Vec<_>>();let config=FusedPhiKrylovConfig{maximum_substeps:1,..cfg()};
   let r=FusedPhiPrefixSession::begin(scalar(-1.),h,&b,config,2,&mut WorkCounters::default()).unwrap().finish(&mut WorkCounters::default()).unwrap();
   emit(json!({"id":"prefix_closure","h":h,"b0":with_b0,"value":r.value[0],"expected":exact,"relative_error":(r.value[0]/exact-1.).abs(),"converged":r.converged,"basis":r.convergence_basis}));
  }
 }}
 let op=Arc::new(ClosureOperator::new(8,|x,y|{for(i,v)in x.iter().enumerate(){y[i]=-*v;}Ok(())}))as Arc<dyn LinearOperator>;let mut v8=vec![0.;8];v8[0]=1.;
 let r=krylov_phi_action(op,0.1,1,&v8,ExponentialKrylovConfig::default(),&mut WorkCounters::default()).unwrap();emit(json!({"id":"old_p3_closure","report":r,"expected":phi(-0.1,1)}));
 let zero=DenseMatrix::zeros(1,1);
 for (h,b4,expected) in [(1e-100,1e300,1e-100/24.),(-1e-100,1e300,1e-100/24.),(1e100,1e-300,1e100/24.)]{
  let b=vec![vec![0.],vec![0.],vec![0.],vec![0.],vec![b4]];
  scalar_row("scaled_range_dense",h,expected,dense_fused_phi_action(&zero,h,&b));
  match fused_phi_action(scalar(0.),h,&b,cfg(),&mut WorkCounters::default()){Ok(r)=>emit(json!({"id":"scaled_range_fused","h":h,"expected":expected,"report":r})),Err(e)=>emit(json!({"id":"scaled_range_fused","h":h,"expected":expected,"error":e.to_string()}))}
  let weighted=scaled_weights(h,&b);scalar_row("scaled_range_candidate",h,expected,balanced_dense(&zero,h,&weighted));
 }
 for amplitude in [1e-200,1e-100,1.,1e10,1e20,1e40,1e60,1e100,1e200,1e300]{
  let w=vec![vec![0.],vec![amplitude]];let exact=amplitude*phi(-0.1,1);
  scalar_row("amplitude_dense",amplitude,exact,dense_phi_combination(&matrix,0.1,&w));
  scalar_row("amplitude_candidate",amplitude,exact,balanced_dense(&matrix,0.1,&w));
  let terms=[FusedPhiTerm{coefficient:1.,phi_index:1,vector:&[amplitude]}];let result=fused_phi_linear_combination(scalar(-1.),0.1,&terms,cfg(),&mut WorkCounters::default());
  match result{Ok(r)=>emit(json!({"id":"amplitude_fused","amplitude":amplitude,"expected":exact,"report":r})),Err(e)=>emit(json!({"id":"amplitude_fused","amplitude":amplitude,"error":e.to_string()}))}
 }
}
