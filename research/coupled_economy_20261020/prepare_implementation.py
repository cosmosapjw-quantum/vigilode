#!/usr/bin/env python3
"""Bind an opt-in residual-first path without changing eager formulas/defaults.
Exact parent blobs are checked; caller commits resulting source BEFORE builds.
"""
import hashlib,json,sys
from pathlib import Path
root=Path.cwd();out=Path(sys.argv[1]);out.mkdir(parents=True,exist_ok=True)
records=[]
def blob(s):
 b=s.encode();return hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest()
def edit(path,expected,transform):
 p=root/path;s=p.read_text();old=blob(s)
 if old!=expected:raise RuntimeError('UNEXPECTED_SOURCE:'+path+':'+old)
 s=transform(s);p.write_text(s);records.append({'path':path,'before_blob':old,'after_blob':blob(s),'after_sha256':hashlib.sha256(s.encode()).hexdigest()})
def replace(s,a,b):
 if s.count(a)!=1:raise RuntimeError('NONUNIQUE_ANCHOR:'+a[:120])
 return s.replace(a,b,1)
def lib(s):
 s=replace(s,'pub use fallback::{FallbackResult, SelectedStep, coupled_or_sequential};',
   'pub use fallback::{FallbackResult, SelectedStep, coupled_or_sequential, coupled_or_sequential_budgeted};')
 s=replace(s,'    pub stage_batches: u64,',
 '''    /// Separate derivative dispatches used by the residual-first policy.
    pub derivative_batches: u64,
    /// Converged/exhausted iterates at which no Newton derivative was needed.
    pub residual_only_stops: u64,
    /// Counterfactual eager JVP vectors avoided, not performed work.
    pub unused_stage_jvp_avoided: u64,
    pub stage_batches: u64,''')
 s=replace(s,'if self.stage_batches + self.validation_batches >= c.max_batches as u64 {',
   'if self.stage_batches + self.derivative_batches + self.validation_batches >= c.max_batches as u64 {')
 anchor='''        self.stage_tasks_scheduled += 8;
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct TracePoint'''
 new='''        self.stage_tasks_scheduled += 8;
        Ok(())
    }
    pub(crate) fn derivative_batch(&mut self, c: &Config) -> Result<()> {
        if self.stage_batches + self.derivative_batches + self.validation_batches >= c.max_batches as u64 {
            return Err(Failure::new(FailureKind::Budget, "stage-batch budget exhausted before derivative dispatch"));
        }
        self.derivative_batches += 1;
        self.stage_tasks_scheduled += 8;
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct TracePoint'''
 s=replace(s,anchor,new)
 s=replace(s,'match solver::run(p, t, y, h, cfg, execution, &mut out) {',
   'match solver::run(p, t, y, h, cfg, execution, EvaluationMode::Eager, &mut out) {')
 s+='''
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EvaluationMode { Eager, ResidualFirst }

/// Opt-in residual-first version of `coupled_step` with identical mathematical
/// target and residual/embedded tolerances. An iteration that will not update z
/// does not request a current-stage Jacobian. Full-space verification still runs.
/// Callbacks must be pure at a fixed state/epoch: operation order across phases
/// differs from the eager path. Actual derivative dispatches consume max_batches.
pub fn coupled_step_residual_first(
    p: &OdeProblem, t: f64, y: &[f64], h: f64, cfg: &Config,
    execution: &ParallelExecution,
) -> Attempt {
    let mut out = Attempt::default();
    match solver::run(p, t, y, h, cfg, execution, EvaluationMode::ResidualFirst, &mut out) {
        Ok(candidate) => out.candidate = Some(candidate),
        Err(error) => out.failure = Some(error),
    }
    out
}
'''
 return s
def solver(s):
 s=replace(s,'const S: usize = 8;', 'const S: usize = 8;\n#[path = "residual_first.rs"]\nmod residual_first;')
 s=replace(s,'    epoch: Option<u64>,\n}', '    epoch: Option<u64>,\n    mode: crate::EvaluationMode,\n}')
 s=replace(s,'    let mut z = seed.to_vec();', '''    if f.mode == crate::EvaluationMode::ResidualFirst {
        return residual_first::correct(f, basis, seed, lambda, out);
    }
    let mut z = seed.to_vec();''')
 s=replace(s,'''    execution: &ParallelExecution,
    out: &mut Attempt,
) -> Result<Candidate>''','''    execution: &ParallelExecution,
    mode: crate::EvaluationMode,
    out: &mut Attempt,
) -> Result<Candidate>''')
 s=replace(s,'        f0,\n        epoch,','        f0,\n        epoch,\n        mode,')
 return s
def fallback(s):
 s=replace(s,'''    let attempt = coupled_step(p, t, y, h, c, execution);
    let mut result = SelectedStep {''','''    let attempt = coupled_step(p, t, y, h, c, execution);
    select(p, t, y, h, c, linear, epoch, attempt)
}

#[allow(clippy::too_many_arguments)]
fn select(
    p: &OdeProblem, t: f64, y: &[f64], h: f64, c: &Config,
    linear: &LinearSolverConfig, epoch: Option<u64>, attempt: Attempt,
) -> SelectedStep {
    let mut result = SelectedStep {''')
 s+='''
/// Residual-first candidate with a hard JVP-attempt cap, followed by the SAME
/// protected sequential fallback. The cap only restricts the candidate; fallback
/// work remains separately accounted and is not subject to this cap. It never
/// enlarges a stricter caller budget or changes any numerical acceptance test.
/// A JVP cap is not a wall-time or total-work bound: RHS, projected LU and setup
/// still cost work. This is explicit bounded speculation, not optimal routing.
#[allow(clippy::too_many_arguments)]
pub fn coupled_or_sequential_budgeted(
    p: &OdeProblem, t: f64, y: &[f64], h: f64, c: &Config,
    execution: &ParallelExecution, linear: &LinearSolverConfig,
    candidate_jvp_cap: u64,
) -> SelectedStep {
    let epoch = p.model_epoch();
    let mut limited = c.clone();
    limited.max_jvp = limited.max_jvp.min(candidate_jvp_cap);
    let attempt = crate::coupled_step_residual_first(p, t, y, h, &limited, execution);
    select(p, t, y, h, c, linear, epoch, attempt)
}
'''
 return s
edit('crates/vigilode-coupled/src/lib.rs','314c28899700d532301d7053029cae4d7906fdd6',lib)
edit('crates/vigilode-coupled/src/solver.rs','f90ee1882e33798ac5cc3f48f19bf0c2603e6bb8',solver)
edit('crates/vigilode-coupled/src/fallback.rs','efe42ed7168a6fcac66b19da66e8b2c16790cc98',fallback)
p=root/'crates/vigilode-coupled/tests/economy_contracts.rs';s=p.read_text();s=replace(s,'use vigilode_coupled::coupled_step as subject;','use vigilode_coupled::coupled_step_residual_first as subject;');p.write_text(s)
(out/'SOURCE_PATCH.json').write_text(json.dumps({'changes':records,'default_eager_path_preserved':True,'acceptance_thresholds_changed':False},indent=2)+'\n')
print(json.dumps(records,indent=2))
