mod candidate;
use rodas5p_core::WorkCounters;
use rodas5p_fair_ab::*;
use serde_json::json;
use std::cell::Cell;

fn measured(id: &str, ratio: f64, p: &PairedTimingProtocol) -> PairedTimingCase {
    let clock = Cell::new(0.0_f64);
    measure_paired_case(id,p,|| {clock.set(clock.get()+0.001); Ok(())},
        || {clock.set(clock.get()+0.001*ratio); Ok(())},|| clock.get()).unwrap()
}
fn samples(id: &str, ratios: &[f64], p: &PairedTimingProtocol, offset: u32) -> PairedTimingCase {
    let labels: Vec<_>=(0..p.pairs).map(|k| (k*ratios.len()/p.pairs) as u32+offset).collect();
    let refs = labels.iter().map(|s| 0.001*ratios[(*s-offset) as usize]).collect();
    PairedTimingCase::from_samples(id,p,vec![0.001,0.001],vec![0.001;p.pairs],refs).unwrap().with_process_blocks(labels)
}
fn row(id: &str, counters: WorkCounters) -> IntegratorRunRecord {
    IntegratorRunRecord {record_id:id.into(),candidate_id:id.into(),comparator_fidelity:ComparatorFidelity::Production,
      problem_id:"synthetic-ledger".into(),step_size:0.1,status:IntegratorRunStatus::Success,message:"synthetic counter fixture".into(), errors:None,
      work:IntegratorWorkReport {counters,internal_steps:0,output_clipped_steps:0,stored_state_bytes:0},
      timing:IntegratorTimingReport {authoritative:false,batch_iterations:0,wall_samples_seconds:vec![],wall_median_seconds:None,wall_q25_seconds:None,wall_q75_seconds:None},
      reference_checksum:String::new(),output_grid_id:String::new() }
}
fn main() {
  let p=PairedTimingProtocol::authoritative(7);
  let a:Vec<_>=(0..6).map(|k| measured(&format!("measured-{k}"),1.3,&p)).collect();
  let aa:Vec<_>=(0..6).map(|k| measured(&format!("aa-{k}"),1.0,&p)).collect();
  let unlabeled=assess_paired_timing(&p,&a,Some(&aa),detect_timing_host_metadata(1)).unwrap();
  let labeled:Vec<_>=a.iter().cloned().map(|x|x.with_process_blocks(vec![77;p.pairs])).collect();
  let aalabeled:Vec<_>=aa.iter().cloned().map(|x|x.with_process_blocks(vec![77;p.pairs])).collect();
  let explicit=assess_paired_timing(&p,&labeled,Some(&aalabeled),detect_timing_host_metadata(1)).unwrap();
  assert_eq!(unlabeled.corpus.independent_blocks,6); assert_eq!(unlabeled.gate_decision,PairedTimingDecision::Promote);
  assert_eq!(explicit.corpus.independent_blocks,1); assert_eq!(explicit.gate_decision,PairedTimingDecision::Inconclusive);
  println!("{}",json!({"test_id":"R2STAT_SESSION_DEFAULT","actual_processes":1,"pid":std::process::id(),"clock":"synthetic","unlabeled":unlabeled,"explicit_session":explicit}));
  assert_eq!(candidate::balanced_admission(&p,&a,&aa),Err("missing_session_identity"));
  let candidate_good=[samples("case",&[1.3;6],&p,0)]; let aa_good=[samples("aa",&[1.0;6],&p,0)];
  assert_eq!(candidate::balanced_admission(&p,&candidate_good,&aa_good),Ok(()));
  let six=samples("candidate",&[1.3;6],&p,0); let other=samples("aa",&[1.0;6],&p,100);
  let unmatched=assess_paired_timing(&p,&[six],Some(&[other]),detect_timing_host_metadata(1)).unwrap();
  assert_eq!(unmatched.gate_decision,PairedTimingDecision::Promote);
  assert_eq!(candidate::balanced_admission(&p,&candidate_good,&[samples("aa",&[1.0;6],&p,100)]),Err("unmatched_sessions"));
  println!("{}",json!({"test_id":"R2STAT_AA_UNMATCHED_SESSIONS","candidate_sessions":[0,1,2,3,4,5],"aa_sessions":[100,101,102,103,104,105],"assessment":unmatched}));
  let seq=WorkCounters {linear_matvecs:16,linear_matvec_vectors:16,..Default::default()};
  let block=WorkCounters {linear_matvecs:3,linear_matvec_vectors:24,..Default::default()};
  assert_eq!(row("seq",seq).cost(ParetoCostMetric::OperatorStateVectors),Some(16.0));
  assert_eq!(row("block",block).cost(ParetoCostMetric::OperatorStateVectors),Some(24.0));
  let mut old_json=serde_json::to_value(seq).unwrap(); old_json.as_object_mut().unwrap().remove("linear_matvec_vectors");
  let legacy:WorkCounters=serde_json::from_value(old_json).unwrap();
  assert_eq!(legacy.operator_state_vectors(),None);
  let modern=WorkCounters {linear_matvecs:1,linear_matvec_vectors:1,..Default::default()};
  let mut mixed=legacy; mixed.checked_accumulate(modern).unwrap();
  let mixedrow=row("mixed",mixed);
  assert_eq!(mixedrow.cost(ParetoCostMetric::OperatorStateVectors),Some(1.0));
  println!("{}",json!({"test_id":"R2STAT_PARETO_AND_MIXED","sequential_cost":row("seq",seq).cost(ParetoCostMetric::OperatorStateVectors),"block_cost":row("block",block).cost(ParetoCostMetric::OperatorStateVectors),"legacy_cost":row("legacy",legacy).cost(ParetoCostMetric::OperatorStateVectors),"mixed_cost":mixedrow.cost(ParetoCostMetric::OperatorStateVectors),"mixed_counters":mixed,"minimum_physical_vectors_if_legacy_single_vector":17}));
  use candidate::VectorEvidence;
  assert_eq!(VectorEvidence::from_source(legacy).add(VectorEvidence::from_source(modern)),VectorEvidence::Unknown);
  assert_eq!(VectorEvidence::from_source(seq).add(VectorEvidence::from_source(block)),VectorEvidence::Known(40));
  let mut p1=p.clone(); p1.bootstrap_resamples=1;
  assert_eq!(candidate::balanced_admission(&p1,&candidate_good,&aa_good),Err("insufficient_ci_protocol"));
  println!("{}",json!({"test_id":"R2STAT_CANDIDATE_ADMISSION_AND_LEDGER","assertions":6,"status":"PASS","claim":"balanced-design admission and absorbing unknown only; no confidence coverage proof"}));
  for seed in 0..1000 {p1.seed=seed; let cases=[samples("case",&[0.8,0.9,1.0,1.1,1.4,1.5],&p1,0)];let aa=[samples("aa",&[1.0;6],&p1,0)];
    let r=assess_paired_timing(&p1,&cases,Some(&aa),detect_timing_host_metadata(1)).unwrap();
    if r.gate_decision==PairedTimingDecision::Promote {assert!(r.corpus.point<1.15);
      let mut proper=p1.clone();proper.bootstrap_resamples=10000; let normal=assess_paired_timing(&proper,&cases,Some(&aa),detect_timing_host_metadata(1)).unwrap();
      println!("{}",json!({"test_id":"R2STAT_SINGLE_RESAMPLE","assessment":r,"normal_protocol":normal}));break; }
  }
}
