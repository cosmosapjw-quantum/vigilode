#[path="integer_gate.rs"]mod gate;
use gate::*;
use std::alloc::{GlobalAlloc,Layout,System};
use std::sync::atomic::{AtomicUsize,Ordering};
struct Count;static N:AtomicUsize=AtomicUsize::new(0);
unsafe impl GlobalAlloc for Count{unsafe fn alloc(&self,l:Layout)->*mut u8{N.fetch_add(1,Ordering::Relaxed);unsafe{System.alloc(l)}}unsafe fn dealloc(&self,p:*mut u8,l:Layout){unsafe{System.dealloc(p,l)}}unsafe fn realloc(&self,p:*mut u8,l:Layout,n:usize)->*mut u8{N.fetch_add(1,Ordering::Relaxed);unsafe{System.realloc(p,l,n)}}}
#[global_allocator]static A:Count=Count;
fn one()->[u64;33]{[1f64.to_bits();33]}
fn check(e:[u64;30],s:[u64;33])->Decision{evaluate(&e,&s).unwrap().decision()}
#[test]fn zero(){assert_eq!(check([0;30],one()),Decision::Accept)}
#[test]fn signed_zero(){assert_eq!(check([1u64<<63;30],one()),Decision::Accept)}
#[test]fn correlation(){let mut e=[0;30];e[24]=2.5f64.to_bits();e[25]=e[24];assert_eq!(check(e,one()),Decision::Reject);e[25]^=1u64<<63;assert_eq!(check(e,one()),Decision::Accept)}
#[test]fn denominator(){let mut e=[0;30];e[0]=5.5f64.to_bits();assert_eq!(check(e,one()),Decision::Accept)}
#[test]fn overflow_row(){let mut e=[0;30];e[24]=1e308f64.to_bits();e[25]=e[24];assert_eq!(check(e,[1e308f64.to_bits();33]),Decision::Accept)}
#[test]fn max_three(){let mut e=[0;30];e[26..29].fill(f64::MAX.to_bits());assert_eq!(check(e,[f64::MAX.to_bits();33]),Decision::Accept)}
#[test]fn cancellation(){let mut e=[0;30];e[26]=f64::MAX.to_bits();e[27]=(-f64::MAX).to_bits();e[28]=1;assert_eq!(check(e,[f64::MAX.to_bits();33]),Decision::Accept)}
#[test]fn tiny_scale_reject(){let mut e=[0;30];e[0]=f64::MAX.to_bits();assert_eq!(check(e,[1;33]),Decision::Reject)}
#[test]fn huge_scale_accept(){let mut e=[0;30];e[0]=1;assert_eq!(check(e,[f64::MAX.to_bits();33]),Decision::Accept)}
#[test]fn exact_boundary(){let mut e=[0;30];e[..21].fill(1f64.to_bits());e[23]=1f64.to_bits();e[24]=1f64.to_bits();e[26]=2f64.to_bits();let c=evaluate(&e,&one()).unwrap();assert_eq!(c,Certificate::Bound{lo:33u128<<112,hi:33u128<<112});assert_eq!(c.decision(),Decision::Accept)}
#[test]fn validate_before_shortcut(){let mut e=[0;30];e[0]=f64::MAX.to_bits();e[29]=f64::NAN.to_bits();assert!(evaluate(&e,&[1;33]).is_err());e[29]=0;let mut s=one();s[32]=0;assert!(evaluate(&e,&s).is_err())}
#[test]fn invalid_scales(){for b in [0,1u64<<63,(-1f64).to_bits(),f64::INFINITY.to_bits(),f64::NAN.to_bits()]{let mut s=one();s[5]=b;assert!(evaluate(&[0;30],&s).is_err())}}
#[test]fn input_not_changed(){let e=[0.03125f64.to_bits();30];let s=one();let before=(e,s);evaluate(&e,&s).unwrap();assert_eq!(before,(e,s))}
#[test]fn zero_allocations(){let e=[0.03125f64.to_bits();30];let s=one();let before=N.load(Ordering::Relaxed);for _ in 0..128{std::hint::black_box(evaluate(std::hint::black_box(&e),std::hint::black_box(&s)).unwrap());}assert_eq!(N.load(Ordering::Relaxed)-before,0)}
