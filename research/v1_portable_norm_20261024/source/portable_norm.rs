//! Fixed PR79 error30 -> physical33 arithmetic enclosure, not an ODE certificate.
//! IEEE binary64, round-to-nearest, gradual underflow, no fast-math/FTZ required.
//! No heap allocation, state mutation, Jacobian, JVP, or external dependencies.
use std::cmp::Ordering;
const MASK:u64=(1_u64<<52)-1;
const LIMBS:usize=33;

/// Squared norm lies in [lo,hi] * 2^exponent. Fields are private: use endpoints().
#[derive(Debug,Clone,Copy,PartialEq)]
pub struct Bounds { lo:f64, hi:f64, exponent:i32 }
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Decision { Accept, Reject, Indeterminate }
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum NormError { NonFiniteError, InvalidScale }
impl Bounds {
    pub fn endpoints(self)->(f64,f64,i32){(self.lo,self.hi,self.exponent)}
    /// Indeterminate never authorizes committing an accepted timestep.
    pub fn decision(self)->Decision {
        if cmp_one(self.hi,self.exponent)!=Ordering::Greater {Decision::Accept}
        else if cmp_one(self.lo,self.exponent)==Ordering::Greater {Decision::Reject}
        else {Decision::Indeterminate}
    }
}
fn down(x:f64)->f64 { if x==0.0 {0.0} else {x.next_down().max(0.0)} }
fn up(x:f64)->f64 { x.next_up() }
/// Exact decomposition of finite positive x as m*2^k, 1<=m<2.
fn parts(x:f64)->(f64,i32) {
    let b=x.to_bits() & 0x7fff_ffff_ffff_ffff;
    let f=b&MASK; let eb=((b>>52)&2047) as i32;
    if eb>0 {(f64::from_bits((1023_u64<<52)|f),eb-1023)}
    else {let h=63-f.leading_zeros() as i32;
          let sig=f<<((52-h) as u32);
          (f64::from_bits((1023_u64<<52)|(sig&MASK)),h-1074)}
}
fn cmp_one(x:f64,e:i32)->Ordering {
    if x==0.0{return Ordering::Less}
    let (m,k)=parts(x);let total=k+e;
    match total.cmp(&0){Ordering::Equal=>m.partial_cmp(&1.0).unwrap(),other=>other}
}
/// Exact floor/ceiling of positive finite x * 2^e in binary64.
fn shift(x:f64,e:i32,upper:bool)->f64 {
    if x==0.0{return 0.0}
    let (m,k)=parts(x);let k=k+e;
    if k>1023 {return if upper {f64::INFINITY}else{f64::MAX}}
    let sig=(m.to_bits()&MASK)|(1_u64<<52);
    if k>=-1022 {return f64::from_bits((((k+1023) as u64)<<52)|(sig&MASK))}
    let n=(-1022-k) as u32;
    let q=if n>=64 {0}else{sig>>n};
    let lost=if n>=64 {sig!=0}else{sig&((1_u64<<n)-1)!=0};
    f64::from_bits(q+u64::from(upper&&lost))
}
fn add_word(a:&mut[u64;LIMBS],mut i:usize,mut v:u64) {
    while v!=0 {let (r,c)=a[i].overflowing_add(v);a[i]=r;v=u64::from(c);i+=1;}
}
fn compare(a:&[u64;LIMBS],b:&[u64;LIMBS])->Ordering {
    for i in(0..LIMBS).rev(){let c=a[i].cmp(&b[i]);if c!=Ordering::Equal{return c}}
    Ordering::Equal
}
/// Internal helper: inputs already checked, length1..3. Exact sum, then enclosure.
pub(crate) fn sum_abs_scaled(xs:&[f64])->(f64,f64,i32) {
    debug_assert!(!xs.is_empty()&&xs.len()<=3&&xs.iter().all(|x|x.is_finite()));
    if xs.len()==1 {let x=xs[0].abs();return if x==0.0{(0.,0.,0)}else{let(m,e)=parts(x);(m,m,e)}}
    let mut pos=[0_u64;LIMBS];let mut neg=[0_u64;LIMBS];
    for &x in xs {
        let b=x.to_bits();let f=b&MASK;let eb=((b>>52)&2047) as usize;
        let sig=if eb==0{f}else{f|(1_u64<<52)};
        let offset=if eb==0{0}else{eb-1};
        let value=(sig as u128)<<(offset%64);
        let a=if b>>63==0{&mut pos}else{&mut neg};
        add_word(a,offset/64,value as u64);
        add_word(a,offset/64+1,(value>>64) as u64);
    }
    let (a,b)=match compare(&pos,&neg){Ordering::Equal=>return(0.,0.,0),Ordering::Greater=>(pos,neg),Ordering::Less=>(neg,pos)};
    let mut mag=[0_u64;LIMBS];let mut borrow=false;
    for i in 0..LIMBS {
        let(r,b1)=a[i].overflowing_sub(b[i]);let(r,b2)=r.overflowing_sub(u64::from(borrow));
        mag[i]=r;borrow=b1||b2;
    }
    debug_assert!(!borrow);
    let word=(0..LIMBS).rev().find(|&i|mag[i]!=0).unwrap();
    let high=word*64+(63-mag[word].leading_zeros() as usize);
    let cut=high.saturating_sub(52);let mut q=0_u64;
    for bit in 0..53 {let index=cut+bit;q|=((mag[index/64]>>(index%64))&1)<<bit;}
    if high<52 {q<<=52-high;}
    let lost=mag[..cut/64].iter().any(|&x|x!=0)
        ||(cut%64!=0 && mag[cut/64]&((1_u64<<(cut%64))-1)!=0);
    let lo=(q as f64)*f64::from_bits(971_u64<<52); // exact2^-52
    (lo,if lost{lo.next_up()}else{lo},high as i32-1074)
}
/// Enclose the fixed physical WRMS squared using caller-owned full physical scales.
/// Errors and scales are not modified. This does not construct scales or certify
/// that a supplied embedded error estimates the true integration error.
pub fn enclose(error:&[f64;30],scale:&[f64;33])->Result<Bounds,NormError> {
    if error.iter().any(|x|!x.is_finite()){return Err(NormError::NonFiniteError)}
    if scale.iter().any(|x|!x.is_finite()||*x<=0.0){return Err(NormError::InvalidScale)}
    let mut rows=[(0.0,0.0,0_i32);33];let mut j=0;let mut largest=i32::MIN;
    for i in 0..33 {
        let count=match i {23=>1,25=>2,28=>3,_=>1};
        let (lo,hi,e)=sum_abs_scaled(&error[j..j+count]);
        if ![23,25,28].contains(&i){j+=1;}
        if hi==0.0 {continue}
        let (sm,se)=parts(scale[i]);let re=e-se;
        rows[i]=(down(lo/sm),up(hi/sm),re);largest=largest.max(re);
    }
    if largest==i32::MIN{return Ok(Bounds{lo:0.,hi:0.,exponent:0})}
    let(mut low,mut high)=(0.0,0.0);
    for (lo,hi,e) in rows {
        if hi==0.0{continue}
        let a=shift(lo,e-largest,false);let b=shift(hi,e-largest,true);
        low=down(low+down(a*a));high=up(high+up(b*b));
    }
    Ok(Bounds{lo:down(low/33.0),hi:up(high/33.0),exponent:2*largest})
}
