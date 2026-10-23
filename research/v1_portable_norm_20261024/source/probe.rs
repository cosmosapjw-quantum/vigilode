#[path="portable_norm.rs"] mod portable_norm;
use portable_norm::*;
use std::io::{self,BufRead};
fn main(){
 for line in io::stdin().lock().lines(){
  let line=line.unwrap();let mut it=line.split_whitespace();let mode=it.next().unwrap();
  let v:Vec<f64>=it.map(|s|f64::from_bits(u64::from_str_radix(s,16).unwrap())).collect();
  if mode=="S" {let(a,b,e)=sum_abs_scaled(&v);println!("{:016x} {:016x} {} S",a.to_bits(),b.to_bits(),e);}
  else {assert_eq!(v.len(),63);let e:&[f64;30]=v[..30].try_into().unwrap();let s:&[f64;33]=v[30..].try_into().unwrap();
   match enclose(e,s){Ok(b)=>{let(l,h,k)=b.endpoints();println!("{:016x} {:016x} {} {:?}",l.to_bits(),h.to_bits(),k,b.decision());},Err(e)=>println!("ERROR {:?}",e)}
  }
 }
}
