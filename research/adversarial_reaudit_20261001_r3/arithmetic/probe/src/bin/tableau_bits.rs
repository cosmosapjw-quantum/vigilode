use rodas5p_core::{DenseMatrix,load_rodas5p_coefficients};
use serde_json::json;
fn bits(v:&[f64])->Vec<String>{v.iter().map(|x|format!("{:016x}",x.to_bits())).collect()}
fn mat(m:&DenseMatrix)->Vec<Vec<String>>{(0..m.nrows()).map(|i|(0..m.ncols()).map(|j|format!("{:016x}",m[(i,j)].to_bits())).collect()).collect()}
fn main(){let c=load_rodas5p_coefficients().unwrap();println!("{}",json!({"gamma":format!("{:016x}",c.gamma.to_bits()),"alpha":mat(&c.alpha),"l":mat(&c.l),"b":bits(&c.b),"btilde":bits(&c.btilde),"snapshot_schema_version":c.snapshot_schema_version}));}
