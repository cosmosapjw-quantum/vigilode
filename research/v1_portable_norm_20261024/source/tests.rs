#[path="portable_norm.rs"] mod portable_norm;
use portable_norm::*;
use std::alloc::{GlobalAlloc,Layout,System};
use std::sync::atomic::{AtomicUsize,Ordering};
struct Count;
static ALLOCS:AtomicUsize=AtomicUsize::new(0);
unsafe impl GlobalAlloc for Count {
 unsafe fn alloc(&self,l:Layout)->*mut u8 { ALLOCS.fetch_add(1,Ordering::Relaxed); unsafe{System.alloc(l)} }
 unsafe fn dealloc(&self,p:*mut u8,l:Layout){unsafe{System.dealloc(p,l)}}
 unsafe fn realloc(&self,p:*mut u8,l:Layout,n:usize)->*mut u8{ALLOCS.fetch_add(1,Ordering::Relaxed);unsafe{System.realloc(p,l,n)}}
}
#[global_allocator]static A:Count=Count;
fn sq(e:[f64;30],s:[f64;33])->Bounds{enclose(&e,&s).unwrap()}
#[test]fn zero(){let b=sq([0.;30],[1.;33]);assert_eq!(b.endpoints().0,0.);assert_eq!(b.endpoints().1,0.);assert_eq!(b.decision(),Decision::Accept);}
#[test]fn correlated(){let mut e=[0.;30];e[24]=2.5;e[25]=2.5;assert_eq!(sq(e,[1.;33]).decision(),Decision::Reject);e[25]=-2.5;assert_eq!(sq(e,[1.;33]).decision(),Decision::Accept);}
#[test]fn denominator(){let mut e=[0.;30];e[0]=5.5;assert_eq!(sq(e,[1.;33]).decision(),Decision::Accept);}
#[test]fn raw_overflow_finite(){let mut e=[0.;30];e[24]=1e308;e[25]=1e308;assert_eq!(sq(e,[1e308;33]).decision(),Decision::Accept);}
#[test]fn max_three(){let mut e=[0.;30];e[26]=f64::MAX;e[27]=f64::MAX;e[28]=f64::MAX;let b=sq(e,[f64::MAX;33]);assert_eq!(b.decision(),Decision::Accept);}
#[test]fn cancellation_order(){let a=sum_abs_scaled(&[f64::MAX,1.,-f64::MAX]);let b=sum_abs_scaled(&[-f64::MAX,f64::MAX,1.]);assert_eq!(a,b);assert_eq!(a,(1.,1.,0));}
#[test]fn cancellation_subnormal(){assert_eq!(sum_abs_scaled(&[f64::MAX,f64::from_bits(1),-f64::MAX]),(1.,1.,-1074));}
#[test]fn scale_extremes(){let mut e=[0.;30];e[0]=f64::MAX;assert_eq!(sq(e,[f64::from_bits(1);33]).decision(),Decision::Reject);e[0]=f64::from_bits(1);assert_eq!(sq(e,[f64::MAX;33]).decision(),Decision::Accept);}
#[test]fn invalid_inputs(){let mut e=[0.;30];e[0]=f64::NAN;assert!(enclose(&e,&[1.;33]).is_err());e[0]=0.;for v in[0.,-1.,f64::INFINITY,f64::NAN]{let mut s=[1.;33];s[0]=v;assert!(enclose(&e,&s).is_err());}}
#[test]fn boundary_is_not_false_accept(){let mut e=[0.;30];e[0]=33_f64.sqrt();let b=sq(e,[1.;33]);assert_eq!(b.decision(),Decision::Indeterminate);}
#[test]fn binary_rescaling(){let mut e=[0.;30];e[24]=0.5;e[25]=-0.25;let a=sq(e,[1.;33]);for v in &mut e{*v*=2_f64.powi(500)}let b=sq(e,[2_f64.powi(500);33]);assert_eq!(a,b);}
#[test]fn no_heap(){let e=[0.03125;30];let s=[1.;33];let before=ALLOCS.load(Ordering::Relaxed);for _ in 0..128{std::hint::black_box(enclose(std::hint::black_box(&e),std::hint::black_box(&s)).unwrap());}let after=ALLOCS.load(Ordering::Relaxed);assert_eq!(after-before,0);}
