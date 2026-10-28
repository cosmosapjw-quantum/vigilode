use rodas5p_fair_ab::*;
use serde_json::json;
use std::{fs, path::Path};
fn records(p: &PairedTimingProtocol) -> Vec<SessionRecord> {
 (0..6).map(|s| {
  let case = |id: &str, ratio: f64| PairedTimingCase::from_samples(id,p,vec![0.005,0.001],vec![0.001;p.pairs],vec![0.001*ratio;p.pairs]).unwrap().with_process_blocks(vec![s;p.pairs]);
  SessionRecord{campaign_id:"r4-synthetic-contract".into(),provenance:SessionProvenance{session:s,process_id:1000+s,started_unix_seconds:1000.0+s as f64,finished_unix_seconds:1000.5+s as f64,clock:PAIRED_TIMING_MONOTONIC_CLOCK.into(),host:detect_timing_host_metadata(1)},cases:vec![case("case-0",1.3)],aa_cases:vec![case("case-0#aa",1.0)],failures:vec![]}
 }).collect()
}
fn evaluate(name:&str,p:&PairedTimingProtocol,records:Vec<SessionRecord>) {
 let arm=|id:&str|ArmIdentity{arm_id:id.into(),executable_sha256:"a".repeat(64),workload_id:"synthetic-r4-source-contract-only".into()};
 let result=PairedTimingReceipt::from_sessions("r4-synthetic-contract",arm("candidate"),arm("reference"),p.clone(),records).and_then(PairedTimingEvidence::from_receipt).and_then(|e|e.verified_decision());
 println!("{}",json!({"kind":"receipt_probe","name":name,"result":format!("{result:?}")}));
}
fn main(){
 let p=PairedTimingProtocol::authoritative(20261011);
 let original=records(&p);
 evaluate("valid_control_under_documented_authority_hold",&p,original.clone());
 for name in ["later_empty_warmups","later_one_warmup","shift_candidate_counts","shift_reference_counts","end_before_start","wrong_batch","missing_later_case"] {
  let mut r=original.clone();
  match name {
   "later_empty_warmups"=>r[1].cases[0].warmup_seconds.clear(),
   "later_one_warmup"=>r[1].cases[0].warmup_seconds.truncate(1),
   "shift_candidate_counts"=>{let x=r[1].cases[0].candidate_seconds.pop().unwrap();r[2].cases[0].candidate_seconds.push(x);},
   "shift_reference_counts"=>{let x=r[1].cases[0].reference_seconds.pop().unwrap();r[2].cases[0].reference_seconds.push(x);},
   "end_before_start"=>r[1].provenance.finished_unix_seconds=0.0,
   "wrong_batch"=>r[1].cases[0].batch_iterations+=1,
   "missing_later_case"=>{r[1].cases.clear();},
   _=>unreachable!()
  } evaluate(name,&p,r);
 }
 for name in ["empty_order","wrong_order","wrong_batch","negative_warmup","nan_warmup"] {
  let mut c=original[0].cases[0].clone();
  match name {
   "empty_order"=>c.order.clear(),
   "wrong_order"=>c.order.fill([PairedArm::Candidate,PairedArm::Reference]),
   "wrong_batch"=>c.batch_iterations+=1,
   "negative_warmup"=>c.warmup_seconds[0]=-1.0,
   "nan_warmup"=>c.warmup_seconds[0]=f64::NAN,
   _=>unreachable!()
  }
  println!("{}",json!({"kind":"r3_closure","name":name,"result":format!("{:?}",c.admit(&p))}));
 }
 let args:Vec<String>=std::env::args().collect(); if args.len()>1 {
  for study in ["poly03","hom06"] {
   let path=Path::new(&args[1]).join(format!("research/r3_matched_accuracy_{study}_20261001/CAMPAIGN.json"));
   let raw:serde_json::Value=serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
   for (arm,value) in raw["evidence"].as_object().unwrap(){
    let result=serde_json::from_value::<PairedTimingEvidence>(value.clone()).map_err(|e|e.to_string()).and_then(|e|e.verified_decision().map(|d|format!("{d:?}")).map_err(|e|e.to_string()));
    println!("{}",json!({"kind":"published_replay","study":study,"arm":arm,"result":result}));
   }
  }
 }
}
