//! Bounded fail-closed candidate; not a full-range stable power evaluator.
use rodas5p_core::CoreResult;
use rodas5p_integrators::OutputBudgetPolicy;
use serde_json::json;
fn guarded_mixed(eta:f64,er:f64,hr:f64,p:u32,e:f64,h:f64)->CoreResult<f64>{
    OutputBudgetPolicy::mixed(eta,er,hr,p)?.validate()?;
    // Evaluate independently with each validated public policy, propagating
    // failure before the minimum reduction. Preserve exact zero budgets.
    let relative=OutputBudgetPolicy::embedded_relative(eta)?.budget(e,h)?;
    let step=if er==0.0 {OutputBudgetPolicy::absolute(0.0)?.budget(e,h)?}
             else {OutputBudgetPolicy::step_power(er,hr,p)?.budget(e,h)?};
    Ok(relative.min(step))
}
fn main(){
 for (name,er,hr,h) in [("zero_times_overflow",0.0,1e-308,1e308),("finite_true_budget",1e-320,1e-160,1.0),("ordinary",1.0,1.0,0.5)]{
  let r=guarded_mixed(10.0,er,hr,2,1.0,h);
  let accept=r.as_ref().is_ok_and(|b|5.0<=*b);
  assert!(!accept);
  println!("{}",json!({"case":name,"result":format!("{r:?}"),"output":5.0,"accepted":accept,"claim":"fail-closed only; full-range evaluation unresolved"}));
 }
}
