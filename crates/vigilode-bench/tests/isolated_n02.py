#!/usr/bin/env python3
"""Compile actual harness/corpus/report with mock solver adapters; never numerical evidence.

No Cargo/native libraries. Requires rustc and existing, mutually compatible serde
and serde_json rlibs via --deps. Work products go outside the repository.
"""
import argparse
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--deps', type=Path, required=True)
args = parser.parse_args()
crate = Path(__file__).resolve().parents[1]
source = (crate/'src/arms.rs').read_text()
prefix = source[source.index('/// A solver arm.'):source.index('\nfn u(')]
mock = r'''
use serde::{Serialize, Deserialize};
use serde_json::Value;
use crate::{alloc::Mark, model::Model};
use std::sync::atomic::{AtomicUsize, Ordering};
pub static MODE: AtomicUsize = AtomicUsize::new(0);
'''+prefix+r'''
pub fn run(a: Arm, m: &Model, g: &[f64], r: f64, at: f64) -> RunOutput { run_with(a,m,g,r,at,None) }
pub fn run_with(a: Arm, m: &Model, g: &[f64], r: f64, _at: f64, _max: Option<usize>) -> RunOutput {
    let mode=MODE.load(Ordering::SeqCst);
    let coarse = r == 1e-11;
    let fail = mode == 1 && coarse && a == Arm::CvodeBdfDense;
    let mut states=vec![m.y0();g.len()];
    if mode == 2 && coarse { for row in &mut states { row[0] += 1e-3; } }
    if mode == 3 && a == Arm::CvodeBdfDense && coarse { states.pop(); }
    RunOutput { status: if fail { RunStatus::SolverFailure { code: -1, message: "injected coarse failure".into() } } else { RunStatus::Success }, states, work:Work::default(), native:Value::Null, end:None }
}
'''
with tempfile.TemporaryDirectory(prefix='vigilode-n02-mock-') as tmp:
    tmp=Path(tmp)
    (tmp/'arms.rs').write_text(mock)
    modules=''.join(f'#[path="{crate}/src/{m}.rs"] pub mod {m};\n' for m in ['alloc','model','metrics','corpus','harness','reference_policy','report'])
    root=modules+f'#[path="{tmp}/arms.rs"] pub mod arms;\n'+r'''
#[cfg(test)] mod isolated_tests {
use super::*;
use std::sync::atomic::Ordering;
fn corpus() -> corpus::Corpus {
    let text = std::fs::read_to_string(std::env::var("N02_CORPUS").unwrap()).unwrap();
    let mut c = corpus::Corpus::from_json(&text).unwrap();
    c.problems.truncate(1); c.rtols=vec![1e-4]; c.arms=vec![arms::Arm::VigilodeDense];
    c.grid_points=3; c.warmups=0; c.repetitions=1; c
}
fn execute(mode: usize) -> serde_json::Value {
    arms::MODE.store(mode,Ordering::SeqCst);
    harness::run_corpus(&corpus(), &mut |_| {}).unwrap()
}
#[test] fn complete_ladder_is_passed_and_finest_reference_is_selected() {
    let doc=execute(0); let reference=&doc["references"][0];
    assert_eq!(reference["independent_protocol"]["status"],"PASS");
    assert_eq!(reference["rtol"],1e-13);
    assert_eq!(reference["independent_protocol"]["levels"].as_array().unwrap().len(),3);
    assert_eq!(reference["crosscheck"]["attempts"].as_array().unwrap().len(),3);
    assert_eq!(reference["disputed"],false);
    assert!(report::render(&doc).unwrap().contains("Independent empirical reference tier"));
}
#[test] fn failed_coarse_level_is_not_hidden_by_finest_success() {
    let doc=execute(1); let reference=&doc["references"][0];
    assert_eq!(reference["independent_protocol"]["status"],"REFERENCE_DISPUTED");
    assert_eq!(reference["crosscheck"]["attempts"].as_array().unwrap().len(),3);
    assert_eq!(reference["disputed"],true);
}
#[test] fn agreeing_families_with_unstable_refinement_are_disputed() {
    let doc=execute(2); let protocol=&doc["references"][0]["independent_protocol"];
    assert_eq!(protocol["levels"][0]["cross_family_disagreement"],0.0);
    assert_eq!(protocol["status"],"REFERENCE_DISPUTED");
}
#[test] fn short_grid_with_success_status_is_disputed() {
    let doc=execute(3);
    assert_eq!(doc["references"][0]["independent_protocol"]["status"],"REFERENCE_DISPUTED");
}
#[test] fn legacy_corpus_preserves_reference_shape() {
    let mut c=corpus(); c.reference.independent_protocol=None;
    assert!(!serde_json::to_value(c).unwrap()["reference"].as_object().unwrap().contains_key("independent_protocol"));
}
}
'''
    (tmp/'isolated.rs').write_text(root)
    command=['rustc','--edition=2024','--test',str(tmp/'isolated.rs'),'-L',f'dependency={args.deps}']
    for dep in ['serde','serde_json']:
        candidates=list(args.deps.glob(f'lib{dep}-*.rlib'))
        if len(candidates)!=1: raise SystemExit(f'need one unambiguous {dep} rlib, found {len(candidates)}')
        command += ['--extern',f'{dep}={candidates[0]}']
    command += ['-o',str(tmp/'tests')]
    subprocess.run(command,check=True)
    import os
    env=dict(os.environ,N02_CORPUS=str(crate/'protocols/n02_independent_reference_v1.json'))
    subprocess.run([str(tmp/'tests'),'--test-threads=1'],check=True,env=env)
