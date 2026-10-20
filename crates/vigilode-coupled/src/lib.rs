//! Fail-closed RED scaffold. No candidate is produced before implementation.
#![forbid(unsafe_code)]
use rodas5p_integrators::{OdeProblem,ParallelExecution};
#[derive(Clone,Debug)]
pub struct Config { pub initial_basis:usize,pub max_basis:usize,pub atol:f64,pub rtol:f64 }
impl Default for Config {fn default()->Self{Self{initial_basis:8,max_basis:32,atol:1e-10,rtol:1e-6}}}
#[derive(Clone,Debug)]
pub struct Candidate {pub y_new:Vec<f64>,pub stages_u:Vec<Vec<f64>>,pub full_residual_wrms:f64,pub basis_dimension:usize}
#[derive(Clone,Debug,Default)]
pub struct Attempt {pub candidate:Option<Candidate>,pub failure:Option<String>}
pub fn coupled_step(_p:&OdeProblem,_t:f64,_y:&[f64],_h:f64,_cfg:&Config,_execution:&ParallelExecution)->Attempt{
 Attempt{candidate:None,failure:Some("NOT_IMPLEMENTED".into())}
}
