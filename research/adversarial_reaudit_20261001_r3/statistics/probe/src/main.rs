use rodas5p_core::WorkCounters;
use rodas5p_fair_ab::*;
use serde_json::json;
use std::cell::Cell;

fn measured(id: &str, ratio: f64, p: &PairedTimingProtocol) -> PairedTimingCase {
    let clock=Cell::new(0.0_f64);
    measure_paired_case(id,p,|| {clock.set(clock.get()+0.001);Ok(())},
        || {clock.set(clock.get()+0.001*ratio);Ok(())},|| clock.get()).unwrap()
}
fn samples(id: &str, ratios: &[f64], p: &PairedTimingProtocol, offset: u32) -> PairedTimingCase {
    let labels:Vec<_>=(0..p.pairs).map(|k| (k*ratios.len()/p.pairs) as u32+offset).collect();
    let refs=labels.iter().map(|s|0.001*ratios[(*s-offset) as usize]).collect();
    PairedTimingCase::from_samples(id,p,vec![0.001,0.001],vec![0.001;p.pairs],refs).unwrap().with_process_blocks(labels)
}
fn assess(p:&PairedTimingProtocol,c:&[PairedTimingCase],aa:&[PairedTimingCase])->PairedTimingAssessment {
    assess_paired_timing(p,c,Some(aa),detect_timing_host_metadata(1)).unwrap()
}
fn row(counters:WorkCounters)->IntegratorRunRecord {
    IntegratorRunRecord {record_id:"fixture".into(),candidate_id:"fixture".into(),comparator_fidelity:ComparatorFidelity::Production,
      problem_id:"synthetic-ledger".into(),step_size:0.1,status:IntegratorRunStatus::Success,message:"synthetic counter fixture".into(),errors:None,
      work:IntegratorWorkReport {counters,internal_steps:0,output_clipped_steps:0,stored_state_bytes:0},
      timing:IntegratorTimingReport {authoritative:false,batch_iterations:0,wall_samples_seconds:vec![],wall_median_seconds:None,wall_q25_seconds:None,wall_q75_seconds:None},
      reference_checksum:String::new(),output_grid_id:String::new()}
}
// Bounded candidate: validate observable receipt consistency. This cannot
// prove the callbacks ran or that declarations correspond to independent OS sessions.
fn candidate_receipt_check(c:&PairedTimingCase,p:&PairedTimingProtocol)->Result<(),&'static str> {
    let expected=calibrate_batch_iterations(&c.warmup_seconds,p).map_err(|_|"invalid_warmups")?;
    if expected!=c.batch_iterations {return Err("batch_calibration_mismatch");}
    if c.order!=abba_pair_order(c.candidate_seconds.len(),p.seed) {return Err("abba_order_mismatch");}
    Ok(())
}
fn main() {
    let p=PairedTimingProtocol::authoritative(7);
    let a:Vec<_>=(0..6).map(|k|measured(&format!("case{k}"),1.3,&p)).collect();
    let aa:Vec<_>=(0..6).map(|k|measured(&format!("aa{k}"),1.0,&p)).collect();
    let unlabeled=assess(&p,&a,&aa);
    assert_eq!(unlabeled.corpus.independent_blocks,0);
    assert_eq!(unlabeled.gate_decision,PairedTimingDecision::Inconclusive);
    let labeled:Vec<_>=a.iter().cloned().map(|x|x.with_process_blocks(vec![77;p.pairs])).collect();
    let aalabeled:Vec<_>=aa.iter().cloned().map(|x|x.with_process_blocks(vec![77;p.pairs])).collect();
    let one=assess(&p,&labeled,&aalabeled);
    assert_eq!(one.corpus.independent_blocks,1);
    assert_eq!(one.gate_decision,PairedTimingDecision::Inconclusive);
    println!("{}",json!({"id":"R3STAT_SESSION_CLOSURE","unlabelled":unlabeled,"one_session":one,"actual_pid":std::process::id(),"synthetic_clock":true}));
    let good=[samples("case",&[1.3;6],&p,0)];
    let aa_good=[samples("aa",&[1.0;6],&p,0)];
    let other=[samples("aa",&[1.0;6],&p,100)];
    let unmatched=assess(&p,&good,&other);
    assert_eq!(unmatched.gate_decision,PairedTimingDecision::Inconclusive);
    let genuine=assess(&p,&good,&aa_good);
    assert_eq!(genuine.verify_against_raw(&good,Some(&aa_good)).unwrap(),PairedTimingDecision::Promote);
    println!("{}",json!({"id":"R3STAT_MATCH_CLOSURE","unmatched":unmatched,"genuine":genuine}));
    let mut preview=p.clone();preview.bootstrap_resamples=1;preview.seed=1;
    let mixed=[samples("case",&[0.8,0.9,1.0,1.1,1.4,1.5],&preview,0)];
    let prev_aa=[samples("aa",&[1.0;6],&preview,0)];
    let prev=assess(&preview,&mixed,&prev_aa);
    assert_eq!(prev.gate_decision,PairedTimingDecision::Inconclusive);assert!(prev.verified_gate_decision().is_err());
    let mut proper=preview.clone();proper.bootstrap_resamples=10000;
    let full=assess(&proper,&mixed,&prev_aa);
    println!("{}",json!({"id":"R3STAT_PREVIEW_CLOSURE","preview":prev,"full":full}));
    let legacy=WorkCounters {linear_matvecs:16,..Default::default()};
    let modern=WorkCounters {linear_matvecs:1,linear_matvec_vectors:1,..Default::default()};
    let mut merged=legacy;merged.checked_accumulate(modern).unwrap();
    let mut reverse=modern;reverse.checked_accumulate(legacy).unwrap();
    assert_eq!(merged,reverse);assert_eq!(row(merged).cost(ParetoCostMetric::OperatorStateVectors),None);
    assert_eq!(merged.operator_applications(),17);assert_eq!(merged.unknown_vector_calls(),16);
    let mut roundtrip:WorkCounters=serde_json::from_str(&serde_json::to_string(&merged).unwrap()).unwrap();roundtrip.accumulate(modern);
    assert_eq!(roundtrip.operator_state_vectors(),None);
    println!("{}",json!({"id":"R3STAT_UNKNOWN_CLOSURE","merged":merged,"cost":row(merged).cost(ParetoCostMetric::OperatorStateVectors),"roundtrip_then_add":roundtrip}));
    assert_eq!(candidate_receipt_check(&good[0],&p),Ok(()));
    for mutation in ["empty_order","wrong_order","wrong_batch","negative_warmup","nan_warmup"] {
        let mut bad=good.clone();
        match mutation {
            "empty_order"=>bad[0].order.clear(),
            "wrong_order"=>bad[0].order=vec![[PairedArm::Candidate,PairedArm::Reference];p.pairs],
            "wrong_batch"=>bad[0].batch_iterations=1,
            "negative_warmup"=>bad[0].warmup_seconds=vec![-1.0;p.warmups],
            "nan_warmup"=>bad[0].warmup_seconds=vec![f64::NAN;p.warmups],
            _=>unreachable!(),
        }
        let r=assess(&p,&bad,&aa_good);
        let raw=r.verify_against_raw(&bad,Some(&aa_good));
        assert_eq!(r.gate_decision,PairedTimingDecision::Promote);
        assert_eq!(raw.unwrap(),PairedTimingDecision::Promote);
        assert!(candidate_receipt_check(&bad[0],&p).is_err());
        println!("{}",json!({"id":"R3STAT_MALFORMED_RAW","mutation":mutation,"case":bad,"assessment":r,"raw_verified_gate":"promote","candidate":candidate_receipt_check(&bad[0],&p).unwrap_err()}));
    }
    let encoded=serde_json::to_string(&(genuine.clone(),good.clone(),aa_good.clone())).unwrap();
    let (decoded,c,aa):(PairedTimingAssessment,[PairedTimingCase;1],[PairedTimingCase;1])=serde_json::from_str(&encoded).unwrap();
    println!("{}",json!({"id":"R3STAT_JSON_ROUNDTRIP","assessment_equal":decoded==genuine,"raw_equal":c==good&&aa==aa_good,"verify":format!("{:?}",decoded.verify_against_raw(&c,Some(&aa)))}));
}
