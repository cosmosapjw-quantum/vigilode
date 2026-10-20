//! Explicit protected fallback. Raw-U candidate and native-K fallback are not
//! labelled bit-identical targets. Work of the declined candidate remains visible.
use crate::{Attempt,Config,Failure,FailureKind,coupled_step};
use rodas5p_core::{LinearMethod,LinearSolverConfig,PreconditionerKind,WorkCounters};
use rodas5p_integrators::{OdeProblem,ParallelExecution,sequential_step};
use serde::Serialize;

#[derive(Debug,Serialize)]
pub struct FallbackResult {
    pub y_new:Vec<f64>,pub error_vector:Vec<f64>,pub embedded_wrms:f64,
    pub accepted_by_existing_driver:bool,pub counters:WorkCounters,
    pub target_id:&'static str,
}
#[derive(Debug,Serialize)]
pub struct SelectedStep {
    pub selection:&'static str,pub candidate_attempt:Attempt,
    pub fallback:Option<FallbackResult>,pub fallback_failure:Option<Failure>,
    pub fallback_attempted:bool,
}
/// Numerical/budget rejection may use the existing sequential JVP-only GMRES
/// path. Bad input, failed callbacks or a changing model never trigger hidden
/// retries. `accepted_by_existing_driver` is its LTE test, not a global bound.
#[allow(clippy::too_many_arguments)]
pub fn coupled_or_sequential(p:&OdeProblem,t:f64,y:&[f64],h:f64,c:&Config,execution:&ParallelExecution,linear:&LinearSolverConfig)->SelectedStep{
    let epoch=p.model_epoch();let attempt=coupled_step(p,t,y,h,c,execution);
    let mut result=SelectedStep{selection:if attempt.candidate.is_some(){"coupled-operational-candidate"}else{"declined"},candidate_attempt:attempt,fallback:None,fallback_failure:None,fallback_attempted:false};
    if result.candidate_attempt.candidate.is_some(){return result;}
    let eligible=result.candidate_attempt.failure.as_ref().is_some_and(|e|matches!(e.kind,FailureKind::Budget|FailureKind::ProjectionLeakage|FailureKind::ProjectionSingular|FailureKind::RootIteration));
    if !eligible{return result;}
    if p.model_epoch()!=epoch{result.fallback_failure=Some(Failure::new(FailureKind::ModelChanged,"model changed before fallback"));return result;}
    if linear.method!=LinearMethod::Gmres||linear.preconditioner!=PreconditionerKind::None {
        result.fallback_failure=Some(Failure::new(FailureKind::Unsupported,"protected fallback requires GMRES with no assembled preconditioner"));return result;
    }
    let problem=match p.jvp_only_clone(){Ok(p)=>p,Err(e)=>{result.fallback_failure=Some(Failure::new(FailureKind::Unsupported,e.to_string()));return result;}};
    let mut counters=WorkCounters::default();result.fallback_attempted=true;
    match sequential_step(&problem,t,y,h,linear,None,c.atol,c.rtol,false,&mut counters){
        Ok(step) if p.model_epoch()==epoch=>{
            result.selection="sequential-matrix-free-fallback";
            result.fallback=Some(FallbackResult{y_new:step.y_new,error_vector:step.error_vector,embedded_wrms:step.error_norm,
                accepted_by_existing_driver:step.accepted,counters,target_id:"rodas5p-native-k-sequential"});
        }
        Ok(_)=>result.fallback_failure=Some(Failure::new(FailureKind::ModelChanged,"model changed in fallback")),
        Err(e)=>result.fallback_failure=Some(Failure::new(FailureKind::RootIteration,format!("fallback failed: {e}; attempted counters: {counters:?}"))),
    }result
}
