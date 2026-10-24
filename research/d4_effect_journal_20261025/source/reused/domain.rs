//! Fixed affine population-domain predicate, not a general model invariant proof.
//! Checks the exact real sum of supplied nonnegative binary64 compact values,
//! and separately checks the actually represented full33 returned state.
#![forbid(unsafe_code)]
use std::cmp::Ordering;
const WORDS:usize=33;
const MASK:u64=(1_u64<<52)-1;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Violation { NonFinite, NegativeCompact{index:usize}, GroupOverflow{row:usize}, RepresentedNegative{row:usize} }
fn finite(b:u64)->bool{(b>>52)&2047!=2047}
fn negative(b:u64)->bool{b>>63!=0 && b<<1!=0}
fn add_word(a:&mut[u64;WORDS],mut i:usize,mut v:u64){
    while v!=0{assert!(i<WORDS);let(r,c)=a[i].overflowing_add(v);a[i]=r;v=u64::from(c);i+=1;}
}
fn add(a:&mut[u64;WORDS],b:u64){
    let e=((b>>52)&2047)as usize;let f=b&MASK;
    let m=if e==0{f}else{f|(1_u64<<52)};let k=e.saturating_sub(1);
    let v=(m as u128)<<(k%64);
    add_word(a,k/64,v as u64);add_word(a,k/64+1,(v>>64)as u64);
}
fn sum_exceeds(xs:&[f64],limit:f64)->bool{
    let mut a=[0_u64;WORDS];let mut b=[0_u64;WORDS];
    for &x in xs{add(&mut a,x.to_bits());}add(&mut b,limit.to_bits());
    for i in(0..WORDS).rev(){match a[i].cmp(&b[i]){Ordering::Less=>return false,Ordering::Greater=>return true,Ordering::Equal=>{}}}false
}
/// Fixed PR79 compact ordering; -0 is accepted and never rewritten.
/// `physical` is a supplied represented reconstruction. This function checks
/// its signs, not its binding to z; Session provides that binding via lift().
pub fn check(z:&[f64;30],physical:&[f64;33])->Result<(),Violation>{
    if z.iter().chain(physical).any(|x|!finite(x.to_bits())){return Err(Violation::NonFinite)}
    for(i,&x)in z.iter().enumerate(){if negative(x.to_bits()){return Err(Violation::NegativeCompact{index:i})}}
    for(row,start,count,total)in[(23,23,1,1.0),(25,24,2,0.5),(28,26,3,0.2)]{
        if sum_exceeds(&z[start..start+count],total){return Err(Violation::GroupOverflow{row})}
    }
    for(row,&x)in physical.iter().enumerate(){if negative(x.to_bits()){return Err(Violation::RepresentedNegative{row})}}
    Ok(())
}
