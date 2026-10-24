#[path="integer_gate.rs"]mod integer_gate;
#[path="admission.rs"]mod admission;
use std::io::{self,BufRead};
fn main(){for line in io::stdin().lock().lines(){let line=line.unwrap();let v:Vec<f64>=line.split_whitespace().map(|s|f64::from_bits(u64::from_str_radix(s,16).unwrap())).collect();assert_eq!(v.len(),4);let m=admission::Metric{atol:[v[2];33],rtol:[v[3];33]};match admission::scales(&[v[0];33],&[v[1];33],&m){Ok(s)=>println!("S {:016x} {:016x}",s.lo[0].to_bits(),s.hi[0].to_bits()),Err(e)=>println!("E {:?}",e)}}}
