use std::sync::Arc;
use rodas5p_core::{ClosureOperator,DenseMatrix,LinearOperator,WorkCounters,dense_fused_phi_action,dense_phi_combination_report,times_power,weight_phi_vectors};
use rodas5p_integrators::{FusedPhiKrylovConfig,FusedPhiPrefixSession,fused_phi_action,OutputBudgetPolicy};
use serde_json::json;
fn config()->FusedPhiKrylovConfig {FusedPhiKrylovConfig{relative_tolerance:1e-12,absolute_tolerance:0.0,..Default::default()}}
fn main(){
 for &(id,h,off,b0,b2) in &[("nilpotent_partial_underflow",1e-8,1e308,1e-300,1e-310),("nilpotent_negative_h",-1e-8,1e308,1e-300,1e-310),("zero_operator_partial_underflow",1e-8,0.0,1e-300,1e-310)] {
  let vectors=vec![vec![b0,0.0],vec![0.0,0.0],vec![0.0,b2]];
  let matrix=DenseMatrix::from_rows(&[&[0.0,off],&[0.0,0.0]]).unwrap();
  let op=Arc::new(ClosureOperator::new(2,move|x,y|{y[0]=off*x[1];y[1]=0.0;Ok(())})) as Arc<dyn LinearOperator>;
  let mut work=WorkCounters::default(); let fused=fused_phi_action(op.clone(),h,&vectors,config(),&mut work);
  let mut prefix_work=WorkCounters::default();let prefix=FusedPhiPrefixSession::begin(op,h,&vectors,FusedPhiKrylovConfig{maximum_substeps:1,..config()},0,&mut prefix_work).and_then(|s|s.finish(&mut prefix_work));
  let dense=dense_fused_phi_action(&matrix,h,&vectors);
  let weighted=weight_phi_vectors(h,&vectors);
  let guarded=match &weighted {Ok((_,lost)) if *lost>0=>"Err: weighting underflow requires propagated error bound",Ok(_)=>"Admitted",Err(_)=>"Err: weighting failure"};
  let nilpotent_candidate=vec![b0+times_power(times_power(b2,off,1),h,3)/6.0,times_power(b2,h,2)/2.0];
  println!("{}",json!({"id":id,"inputs":{"h":h,"off":off,"b0":b0,"b2":b2},"fused":fused.as_ref().ok(),"fused_error":fused.as_ref().err().map(ToString::to_string),"work":work,"prefix":prefix.as_ref().ok(),"prefix_error":prefix.as_ref().err().map(ToString::to_string),"prefix_work":prefix_work,"dense":dense.as_ref().ok(),"dense_error":dense.as_ref().err().map(ToString::to_string),"weighting":weighted.as_ref().ok(),"guarded":guarded,"nilpotent_candidate":nilpotent_candidate}));
 }
 for &(id,a,b) in &[("dense_amplified_mixed_range",700.0,-700.0),("dense_mixed_zero",0.0,0.0)] {
  let matrix=DenseMatrix::from_rows(&[&[a,0.0],&[0.0,b]]).unwrap();let w=vec![vec![1e-300,1e300]];
  let r=dense_phi_combination_report(&matrix,1.0,&w);
  println!("{}",json!({"id":id,"diagonal":[a,b],"weights":w,"value":r.as_ref().ok().map(|r|&r.value),"mixed_range":r.as_ref().ok().map(|r|r.mixed_range),"output_below_input_half_precision":r.as_ref().ok().map(|r|r.output_below_input_half_precision),"error":r.as_ref().err().map(ToString::to_string)}));
 }
 for &(id,eps,href,p,h) in &[("original_budget",1e-320,1e-160,2,1.0),("zero_budget",0.0,1e-308,2,1e308),("step_underflow",1e-300,1.0,2,1e-20),("large_exponent",1.0,1.0000000000000002,2147483647,1.0)] {
  let policy=OutputBudgetPolicy::mixed(10.0,eps,href,p).unwrap();let r=policy.decide(5.0,1.0,h);
  println!("{}",json!({"id":id,"epsilon_ref":eps,"h_ref":href,"exponent":p,"h":h,"decision":r.as_ref().ok(),"error":r.as_ref().err().map(ToString::to_string)}));
 }
 // Structured and deterministic random binary64 product inputs. Export bits
 // so the external Fraction oracle uses represented inputs exactly.
 let mut seed=0x52e314a9713da635u64;
 for i in 0..512 {
  seed^=seed<<13;seed^=seed>>7;seed^=seed<<17;let vb=seed & 0xffefffffffffffff;
  seed^=seed<<13;seed^=seed>>7;seed^=seed<<17;let sb=seed & 0xffefffffffffffff;
  let k=(i%5)as u32;let v=f64::from_bits(vb);let s=f64::from_bits(sb);let r=times_power(v,s,k);
  println!("{}",json!({"id":"power_fraction","i":i,"value_bits":vb,"scale_bits":sb,"k":k,"result_bits":r.to_bits()}));
 }
 for (i,(v,s,k)) in [(f64::from_bits(1),1.0,4),(f64::MIN_POSITIVE,0.5,1),(f64::MAX,0.5,1),(1.0,2f64.powi(-537),2),(-f64::from_bits(1),-1.0,3),(1e300,1e-100,4),(1e-300,1e100,4),(2.0,-3.0,3)].into_iter().enumerate(){let r=times_power(v,s,k);println!("{}",json!({"id":"power_fraction","i":512+i,"value_bits":v.to_bits(),"scale_bits":s.to_bits(),"k":k,"result_bits":r.to_bits()}));}
}
