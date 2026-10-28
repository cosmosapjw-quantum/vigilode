use std::sync::Arc;
use rodas5p_core::{CoreResult,DenseMatrix,DenseOperator,Preconditioner,LinearMethod,LinearSolverConfig};
use rodas5p_integrators::{OdeProblem,OutputSchedule,OutputSamplingPlan,AdaptiveStepConfig,IntegrationMethod,OutputBudgetPolicy,integrate_fixed,integrate_adaptive_observed,integrate_sequential_matrix_free_adaptive_dense_observed,audit2_preconditioner_reuse_certificate};
use serde_json::json;
fn flow(v:f64)->OdeProblem {
 OdeProblem::new("constant-velocity",1,Arc::new(move|_,_,o:&mut[f64]|{o[0]=v;Ok(())}),None,Some(Arc::new(|_,_|DenseMatrix::from_rows(&[&[0.0]]))),Some(Arc::new(|_,_,_,o:&mut[f64]|{o[0]=0.0;Ok(())})),None,true,None,None).unwrap()
}
fn fixed(origin:f64,ulps:u64){
 let tf=if origin>0.0 {f64::from_bits(origin.to_bits()+ulps)}else {f64::from_bits(origin.to_bits()-ulps)};
 let span=tf-origin;
 let r=integrate_fixed(&flow(1.0/span),(origin,tf),&[0.0],span/2.0,IntegrationMethod::Sequential,None,None,1e-10,1e-9);
 println!("{}",match r{Ok(r)=>json!({"probe":"fixed_short_representable_span","origin":origin,"tf":tf,"span":span,"ulps":ulps,"success":r.success,"attempts":r.attempts,"last_t":r.t.last(),"last_y":r.y.last(),"exact_final_y":1.0}),Err(e)=>json!({"probe":"fixed_short_representable_span","error":e.to_string()})});
}
fn dense(origin:f64,subnormal:bool){
 let duration=if subnormal {f64::MIN_POSITIVE/2.0}else{1.0};
 let velocity=if subnormal {1e300}else{8192.0};
 let first=origin+duration/4.0;
 let after=first.next_up();
 let ts=if subnormal{vec![origin,first,origin+duration/2.0,origin+duration]}else{vec![origin,first,after,origin+duration]};
 let sampling=OutputSamplingPlan::dense(OutputSchedule::new(ts).unwrap());
 let cfg=AdaptiveStepConfig{atol:1e-10,rtol:1e-9,initial_step:duration/4.0,min_step:if subnormal {f64::from_bits(1)}else{1e-14},max_step:duration/4.0,max_attempts:100,..Default::default()};
 let lin=LinearSolverConfig{method:LinearMethod::Gmres,..Default::default()};
 let result=integrate_sequential_matrix_free_adaptive_dense_observed(&flow(velocity),(origin,origin+duration),&[0.0],&lin,&cfg,&sampling);
 match result {Ok(r)=>{let o=r.observed;let expected:Vec<f64>=o.t.iter().map(|t|velocity*(t-origin)).collect();let errors:Vec<f64>=o.y.iter().zip(&expected).map(|(y,e)|y[0]-e).collect();println!("{}",json!({"probe":"dense_boundary","origin":origin,"subnormal":subnormal,"times":o.t,"values":o.y,"expected":expected,"errors":errors,"success":o.success,"internal_steps":o.internal_steps}));},Err(e)=>println!("{}",json!({"probe":"dense_boundary","origin":origin,"subnormal":subnormal,"error":e.to_string()}))}
}
fn clipped(){
 let origin=1e12_f64;let tf=origin+1.0;
 let output=OutputSchedule::new(vec![origin,origin.next_up(),tf]).unwrap();
 let r=integrate_adaptive_observed(&flow(1.0),(origin,tf),&[0.0],0.25,IntegrationMethod::Sequential,None,None,1e-10,1e-9,1000,0.25,&output);
 println!("{}",json!({"probe":"clipped_adjacent_times","result":format!("{r:?}")}));
}
struct ScalarPc(f64);
impl Preconditioner for ScalarPc {fn dimension(&self)->usize{1}fn apply(&self,x:&[f64],y:&mut[f64])->CoreResult<()>{y[0]=self.0*x[0];Ok(())}}
fn main(){
 fixed(1e12,8);fixed(-1e12,8);fixed(1.0,1);
 dense(1e12,false);dense(0.0,true);clipped();
 for (name,er,hr,h,eta) in [("zero_times_overflow",0.0,1e-308,1e308,10.0),("finite_true_budget",1e-320,1e-160,1.0,10.0)]{
  let p=OutputBudgetPolicy::mixed(eta,er,hr,2).unwrap();
  println!("{}",json!({"probe":"mixed_intermediate","case":name,"epsilon_ref":er,"h_ref":hr,"h":h,"eta":eta,"decision":format!("{:?}",p.decide(5.0,1.0,h))}));
 }
 let p=1.0+f64::EPSILON;let w=1.0-f64::EPSILON;
 let op=DenseOperator::new(DenseMatrix::from_rows(&[&[w]]).unwrap()).unwrap();
 let cert=audit2_preconditioner_reuse_certificate(&op,&ScalarPc(p),false).unwrap();
 println!("{}",json!({"probe":"m08_rounding_bound","w":w,"p":p,"w_bits":w.to_bits(),"p_bits":p.to_bits(),"certificate":cert}));
}
