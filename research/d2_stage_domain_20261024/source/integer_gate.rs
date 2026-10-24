//! Integer-only admission for the fixed PR79 full33/compact30 physical norm.
//! Inputs are IEEE binary64 encodings, not floating operands. No FP arithmetic,
//! heap, callback, state mutation, native ODE admission or scale construction.
#![forbid(unsafe_code)]
use std::cmp::Ordering;
const WORDS:usize=34;
const MASK:u64=(1u64<<52)-1;
pub const PRECISION:u32=56;
pub const THRESHOLD:u128=33u128<<(2*PRECISION);
#[derive(Debug,Copy,Clone,PartialEq,Eq)]pub enum Decision{Accept,Reject,Indeterminate}
#[derive(Debug,Copy,Clone,PartialEq,Eq)]pub enum Error{NonFiniteError{index:usize},InvalidScale{index:usize}}
/// Evidence returned by evaluate, not an unforgeable driver-commit capability.
#[derive(Debug,Copy,Clone,PartialEq,Eq)]pub enum Certificate{Bound{lo:u128,hi:u128},LargeRow{row:usize}}
impl Certificate{
 pub fn decision(self)->Decision{match self{
  Self::LargeRow{..}=>Decision::Reject,
  Self::Bound{lo,hi}=>if hi<=THRESHOLD{Decision::Accept}else if lo>THRESHOLD{Decision::Reject}else{Decision::Indeterminate}
 }}
}
#[derive(Clone,Copy)]struct Big([u64;WORDS]);
impl Big{
 fn zero()->Self{Self([0;WORDS])}
 fn is_zero(&self)->bool{self.0.iter().all(|&v|v==0)}
 fn compare(&self,b:&Self)->Ordering{for i in(0..WORDS).rev(){let c=self.0[i].cmp(&b.0[i]);if c!=Ordering::Equal{return c}}Ordering::Equal}
 fn add_word(&mut self,mut i:usize,mut value:u64){while value!=0{assert!(i<WORDS);let(v,c)=self.0[i].overflowing_add(value);self.0[i]=v;value=u64::from(c);i+=1;}}
 fn add_bits(&mut self,b:u64){
  let eb=((b>>52)&2047)as usize;let f=b&MASK;let sig=if eb==0{f}else{f|(1u64<<52)};let shift=eb.saturating_sub(1);
  let v=(sig as u128)<<(shift%64);self.add_word(shift/64,v as u64);self.add_word(shift/64+1,(v>>64)as u64);
 }
 fn shifted(&self,n:u32)->Self{
  assert!(n<64);if n==0{return *self}let mut r=Self::zero();let mut carry=0;
  for i in 0..WORDS{r.0[i]=(self.0[i]<<n)|carry;carry=self.0[i]>>(64-n);}assert_eq!(carry,0);r
 }
 fn subtract(&mut self,b:&Self){let mut borrow=false;for i in 0..WORDS{let(a,b1)=self.0[i].overflowing_sub(b.0[i]);let(a,b2)=a.overflowing_sub(u64::from(borrow));self.0[i]=a;borrow=b1||b2;}assert!(!borrow);}
}
fn row_abs(xs:&[u64])->Big{
 debug_assert!(!xs.is_empty()&&xs.len()<=3);let mut pos=Big::zero();let mut neg=Big::zero();
 for &b in xs{if b>>63==0{pos.add_bits(b)}else{neg.add_bits(b)}}
 if pos.compare(&neg)==Ordering::Less{neg.subtract(&pos);neg}else{pos.subtract(&neg);pos}
}
/// Precondition A/B<8, B=M*2^k with M<=53bits from the scale encoding.
/// floor(N/(M*2^k)) = floor((N>>k)/M). Dropped low bits affect only
/// the exact remainder flag. The quotient numerator is <M*2^59<2^112.
fn quotient(a:&Big,scale:u64)->(u64,bool){
 let eb=((scale>>52)&2047)as usize;
 let m=if eb==0{scale&MASK}else{(scale&MASK)|(1u64<<52)};
 let shift=eb.saturating_sub(1);let n=a.shifted(PRECISION);
 let word=shift/64;let bits=shift%64;
 let low=(n.0[word]>>bits)|if bits==0{0}else{n.0[word+1]<<(64-bits)};
 let high=(n.0[word+1]>>bits)|if bits==0{0}else{n.0[word+2]<<(64-bits)};
 let t=(low as u128)|((high as u128)<<64);
 let lost=n.0[..word].iter().any(|&x|x!=0)||(bits!=0&&n.0[word]&((1u64<<bits)-1)!=0);
 debug_assert!(n.0[word+2..].iter().enumerate().all(|(i,&x)|if i==0&&bits!=0{x>>bits==0}else{x==0}));
 let q=t/(m as u128);let rem=t%(m as u128);
 debug_assert!(q<(1u128<<59));(q as u64,rem!=0||lost)
}
/// All inputs are checked before early rejection. Scales represent positive finite
/// supplied physical values; upstream arithmetic/environment is NOT repaired.
/// Bound: lo/(33*2^112) <= exact E² <= hi/(33*2^112).
/// LargeRow is an exact witness E²>=64/33, not a full norm enclosure.
pub fn evaluate(error:&[u64;30],scale:&[u64;33])->Result<Certificate,Error>{
 for(i,&b)in error.iter().enumerate(){if (b>>52)&2047==2047{return Err(Error::NonFiniteError{index:i})}}
 for(i,&b)in scale.iter().enumerate(){if b>>63!=0||b==0||(b>>52)&2047==2047{return Err(Error::InvalidScale{index:i})}}
 let(mut lo,mut hi)=(0u128,0u128);let mut j=0;
 for i in 0..33{
  let count=match i{25=>2,28=>3,_=>1};let a=row_abs(&error[j..j+count]);if ![23,25,28].contains(&i){j+=1}
  if a.is_zero(){continue}let mut b=Big::zero();b.add_bits(scale[i]);
  if a.compare(&b.shifted(3))!=Ordering::Less{return Ok(Certificate::LargeRow{row:i})}
  let(q,lost)=quotient(&a,scale[i]);let u=q+u64::from(lost);
  lo+=(q as u128)*(q as u128);hi+=(u as u128)*(u as u128);
 }
 debug_assert_eq!(j,30);Ok(Certificate::Bound{lo,hi})
}
