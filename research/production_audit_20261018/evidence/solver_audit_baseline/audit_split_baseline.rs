use std::sync::Arc;
use rodas5p_core::WorkCounters;
use rodas5p_integrators::{OdeProblem,SparseLinearPart};
fn main(){
 let a=SparseLinearPart::new(1,1,vec![0,1],vec![1],vec![1.]); println!("SPARSE_LINEAR_INVALID_ACCEPTED {}",a.is_ok());
 let p=OdeProblem::new("nan-split",1,Arc::new(|_,_,o|{o[0]=0.;Ok(())}),None,None,Some(Arc::new(|_,_,_,o|{o[0]=0.;Ok(())})),None,true,None,None).unwrap().with_linear_part(SparseLinearPart::new(1,1,vec![0,1],vec![0],vec![0.]).unwrap(),Arc::new(|_,_,o|{o[0]=f64::NAN;Ok(())})).unwrap();
 println!("SPLIT_NAN_ACCEPTED {}",p.verify_linear_part(0.,&[0.],&mut WorkCounters::default()).is_ok());
}
