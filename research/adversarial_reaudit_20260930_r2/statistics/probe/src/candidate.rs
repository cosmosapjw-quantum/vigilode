//! Bounded admission/ledger candidate, not a production patch.
use rodas5p_core::WorkCounters;
use rodas5p_fair_ab::*;
use std::collections::BTreeSet;

/// Unknown is absorbing under addition. Apply to each source ledger BEFORE merging.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum VectorEvidence { Known(u64), Unknown }
impl VectorEvidence {
    pub fn from_source(w:WorkCounters)->Self {match w.operator_state_vectors(){Some(n)=>Self::Known(n),None=>Self::Unknown}}
    pub fn add(self, other:Self)->Self {match(self,other){(Self::Known(a),Self::Known(b))=>a.checked_add(b).map(Self::Known).unwrap_or(Self::Unknown),_=>Self::Unknown}}
}
/// Balanced complete-design admission only; no claim to cover arbitrary missing cells.
/// IDs remain asserted metadata: authentic runner receipts are a separate requirement.
pub fn balanced_admission(p:&PairedTimingProtocol,cases:&[PairedTimingCase],aa:&[PairedTimingCase])->Result<(),&'static str>{
    if p.bootstrap_resamples<10_000 || p.confidence_level<0.95 {return Err("insufficient_ci_protocol");}
    let first=cases.first().ok_or("missing_cases")?;
    if first.process_blocks.is_empty(){return Err("missing_session_identity");}
    let sessions:BTreeSet<_>=first.process_blocks.iter().copied().collect();
    if sessions.len()<6{return Err("insufficient_sessions");}
    if aa.is_empty(){return Err("missing_aa");}
    for case in cases.iter().chain(aa){
        if case.process_blocks.is_empty(){return Err("missing_session_identity");}
        if case.process_blocks.len()!=case.candidate_seconds.len(){return Err("pair_session_shape");}
        let found:BTreeSet<_>=case.process_blocks.iter().copied().collect();
        if found!=sessions{return Err("unmatched_sessions");}
        let counts:Vec<_>=sessions.iter().map(|s|case.process_blocks.iter().filter(|x|*x==s).count()).collect();
        if counts.windows(2).any(|w|w[0]!=w[1]){return Err("unbalanced_sessions");}
    }
    Ok(())
}
