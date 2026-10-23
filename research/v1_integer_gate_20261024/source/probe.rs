#[path="integer_gate.rs"]mod gate;
use std::io::{self,BufRead};
fn main(){for line in io::stdin().lock().lines(){let line=line.unwrap();let v:Vec<u64>=line.split_whitespace().map(|s|u64::from_str_radix(s,16).unwrap()).collect();assert_eq!(v.len(),63);let e:&[u64;30]=v[..30].try_into().unwrap();let s:&[u64;33]=v[30..].try_into().unwrap();match gate::evaluate(e,s){Ok(c)=>match c{gate::Certificate::Bound{lo,hi}=>println!("B {lo} {hi} {:?}",c.decision()),gate::Certificate::LargeRow{row}=>println!("L {row} 0 {:?}",c.decision())},Err(e)=>println!("E {:?}",e)}}}
