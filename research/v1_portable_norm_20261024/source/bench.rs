#[path="portable_norm.rs"]mod portable_norm;
use std::hint::black_box;
use std::time::Instant;
extern "C" {fn bassrei_full33_error_norm(e:*const f64,s:*const f64,n:*mut f64)->i32;}
fn main(){
 let reps=4000;println!("case,block,arm,repeats,total_ns,checksum");
 for c in 0..3 {let mut e=[0.03125;30];let mut s=[1.;33];
  if c==1{e[26]=1e308;e[27]=-1e308;e[28]=1.;s.fill(1e308);}
  if c==2{e[24]=1e308;e[25]=1e308;s.fill(1e308);}
  let mut cval=0.;assert_eq!(unsafe{bassrei_full33_error_norm(e.as_ptr(),s.as_ptr(),&mut cval)},0);
  assert!(cval.is_finite());let b=portable_norm::enclose(&e,&s).unwrap();black_box(b.decision());
  for block in 0..6 {for offset in 0..2 {let arm=(block+offset)%2;
   for _ in 0..100 {if arm==0{black_box(portable_norm::enclose(black_box(&e),black_box(&s)).unwrap());}else{let mut v=0.;black_box(unsafe{bassrei_full33_error_norm(black_box(e.as_ptr()),black_box(s.as_ptr()),&mut v)});black_box(v);}}
   let t=Instant::now();let mut checksum=0.;
   for _ in 0..reps {if arm==0 {let b=portable_norm::enclose(black_box(&e),black_box(&s)).unwrap();checksum+=black_box(b.endpoints().1);}else{let mut v=0.;let status=unsafe{bassrei_full33_error_norm(black_box(e.as_ptr()),black_box(s.as_ptr()),&mut v)};assert_eq!(status,0);checksum+=black_box(v);}}
   let ns=t.elapsed().as_nanos();println!("{c},{block},{arm},{reps},{ns},{checksum}");
  }}
 }
}
