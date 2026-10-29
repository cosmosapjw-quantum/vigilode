use vigilode::*;
#[path="../../../crates/vigilode/tests/evidence_switch_contracts.rs"] mod switching_contracts;
#[path="../../../crates/vigilode/tests/execution_policy_contracts.rs"] mod execution_contracts;
#[test]
fn reaching_target_on_last_batch_attempt_is_a_successful_batch() {
let p=Problem::new(1,|_,_,d|{d[0]=1.0;Ok(())}).jacobian(Jacobian::dense(|_,_,j|{j[0]=0.0;Ok(())})).autonomous();
let o=Options{h0:0.1,max_steps:100,..Options::default()};
let c=SwitchingConfig{fallback:DerivativeFallback::ProvidedOnly,carry_explicit_step:true,window_count:10,explicit_attempt_budget:1,pressure_steps:usize::MAX,..SwitchingConfig::default()};
let s=SwitchingSolver::solve(&p,(0.0,1.0),&[0.0],&o,&c).unwrap();
let first=&s.stats.segments[0];
assert_eq!(first.reached_time,0.1); assert_eq!(first.attempts,1); assert!(first.success,"{:?}",first);
assert_eq!(first.evidence,SwitchingEvidence::Uncompared);
}
