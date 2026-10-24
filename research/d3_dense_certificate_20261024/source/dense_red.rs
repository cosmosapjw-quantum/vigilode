//! S1 feature-absent RED scaffold. Never used for scientific runs.
#[derive(Debug)] pub struct NotImplemented;
pub struct Model;
pub struct Curve;
pub struct Certified;
pub struct Query{pub center:[f64;33],pub radius:[f64;33],pub rounding:[f64;33]}
pub struct Diagnostics{pub alpha:f64,pub incoming:[f64;30],pub endpoint:[f64;30],pub uniform:[f64;30],pub rho:[f64;30],pub delta:[f64;2]}
impl Model{pub fn new(_:f64)->Result<Self,NotImplemented>{Err(NotImplemented)}}
impl Curve{
 pub fn new(_: &Model,_:usize,_:f64,_:f64,_:&[f64;30],_:&[f64;30])->Result<Self,NotImplemented>{Err(NotImplemented)}
 pub fn from_controls(_: &Model,_:usize,_:f64,_:f64,_:[[f64;30];6])->Result<Self,NotImplemented>{Err(NotImplemented)}
 pub fn controls(&self)->[[f64;30];6]{panic!("feature absent")}
 pub fn certify(&self,_:&[f64;30],_:f64)->Result<Certified,NotImplemented>{Err(NotImplemented)}
}
impl Certified{pub fn query(&self,_:f64)->Result<Query,NotImplemented>{Err(NotImplemented)} pub fn diagnostics(&self)->Diagnostics{panic!("feature absent")}}
