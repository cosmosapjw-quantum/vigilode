#![allow(dead_code)]
include!("support.rs");
mod directed;
mod domain;
mod integer_gate;
mod admission;
mod trajectory;
mod stage_safe;
#[cfg(d3_red)] #[path="dense_red.rs"] mod dense;
#[cfg(not(d3_red))] mod dense;

#[cfg(test)] mod d3_contracts {
 use super::*;
 fn valid(degree:usize)->dense::Curve {
  admission::environment().unwrap();
  let z=trajectory::initial();
  dense::Curve::new(&dense::Model::new(0.).unwrap(),degree,0.,1e-5,&z,&z).unwrap()
 }
 #[test] fn d3_model_rejects_unsupported_parameters(){
  for x in [-1.,1.,f64::NAN,f64::INFINITY]{assert!(dense::Model::new(x).is_err())}
  assert!(dense::Model::new(0.).is_ok());assert!(dense::Model::new(4.).is_ok());
 }
 #[test] fn d3_valid_cubic_and_quintic_queries(){
  for n in [3,5]{let cert=valid(n).certify(&[0.;30],1.).unwrap();
   for t in [0.,3e-6,1e-5]{let q=cert.query(t).unwrap();assert!(q.center.iter().all(|v|v.is_finite()&&*v>=0.));assert!(q.radius.iter().all(|v|v.is_finite()&&*v>=0.&&*v<=1.));}
  }
 }
 #[test] fn d3_degree_and_clock_validation(){
  let m=dense::Model::new(0.).unwrap();let z=trajectory::initial();
  for n in [0,1,2,4,6]{assert!(dense::Curve::new(&m,n,0.,0.01,&z,&z).is_err())}
  for (a,b) in [(0.,0.),(1.,0.),(0.,f64::INFINITY),(f64::NAN,1.)]{assert!(dense::Curve::new(&m,3,a,b,&z,&z).is_err())}
 }
 #[test] fn d3_endpoint_binding_and_immutability(){
  let z=trajectory::initial();let c=valid(5);let before=c.controls();
  assert_eq!(before[0].map(f64::to_bits),z.map(f64::to_bits));assert_eq!(before[5].map(f64::to_bits),z.map(f64::to_bits));
  let mut copy=c.controls();copy[0][0]=123.;assert_ne!(copy,before);assert_eq!(c.controls(),before);
 }
 #[test] fn d3_incoming_radius_is_preserved(){
  let c=valid(3);let mut e=[0.;30];e[0]=1e-3;
  let cert=c.certify(&e,1.).unwrap();let d=cert.diagnostics();
  assert_eq!(d.incoming.map(f64::to_bits),e.map(f64::to_bits));
  assert!(d.uniform[0]>=e[0]);assert!(d.endpoint[0]>0.00099);
 }
 #[test] fn d3_invalid_incoming_and_budget_refused(){
  let c=valid(3);let mut e=[0.;30];e[4]=-1.;assert!(c.certify(&e,1.).is_err());e[4]=f64::NAN;assert!(c.certify(&e,1.).is_err());
  for b in [0.,-1.,f64::NAN,f64::INFINITY]{assert!(c.certify(&[0.;30],b).is_err())}
  assert!(c.certify(&[0.;30],1e-30).is_err());
 }
 #[test] fn d3_query_outside_span_refused(){
  let cert=valid(3).certify(&[0.;30],1.).unwrap();for t in [-1e-9,0.1,f64::NAN,f64::INFINITY]{assert!(cert.query(t).is_err())}
 }
 #[test] fn d3_sampling_trap_rejected_continuously(){
  let m=dense::Model::new(0.).unwrap();let z=trajectory::initial();let mut b=[z;6];
  b[0][0]=0.015;b[1][0]=0.015-0.25/3.;b[2][0]=0.015-0.5/3.+1./3.;b[3][0]=0.765;
  for t in [0.,0.25,0.5,0.75,1.]{assert!((t-0.1)*(t-0.15)>0.)}
  assert!((0.125-0.1)*(0.125-0.15)<0.);
  let r=dense::Curve::from_controls(&m,3,0.,0.01,b).and_then(|c|c.certify(&[0.;30],10.));assert!(r.is_err());
 }
 #[test] fn d3_dependent_control_domain_refused(){
  let m=dense::Model::new(0.).unwrap();let z=trajectory::initial();let mut b=[z;6];b[1][24]=0.4;b[1][25]=0.2;
  let r=dense::Curve::from_controls(&m,3,0.,0.01,b).and_then(|c|c.certify(&[0.;30],10.));assert!(r.is_err());
 }
 #[test] fn d3_nonfinite_controls_refused(){
  let m=dense::Model::new(0.).unwrap();let mut b=[trajectory::initial();6];b[2][8]=f64::NAN;
  assert!(dense::Curve::from_controls(&m,3,0.,0.01,b).is_err());
 }
 #[test] fn d3_exact_duration_uses_stored_endpoints(){
  let m=dense::Model::new(0.).unwrap();let z=trajectory::initial();let t0=1.;let t1=1.1;
  let d=dense::Curve::from_controls(&m,3,t0,t1,[z;6]).unwrap().certify(&[0.;30],1000.).unwrap().diagnostics();
  assert!(d.delta[0]<=t1-t0&&d.delta[1]>=t1-t0);assert_ne!((t1-t0).to_bits(),0.1f64.to_bits());
 }
 #[test] fn d3_full33_radius_uses_affine_sums(){
  let mut e=[0.;30];e[24]=0.01;e[25]=0.02;e[26]=0.03;e[27]=0.04;e[28]=0.05;
  let cert=valid(5).certify(&e,1.).unwrap();let d=cert.diagnostics();let q=cert.query(0.).unwrap();
  assert!(q.radius[25]>=d.uniform[24]+d.uniform[25]);assert!(q.radius[28]>=d.uniform[26]+d.uniform[27]+d.uniform[28]);
  assert!(q.radius.iter().zip(q.rounding).all(|(r,a)|*r>=a));
 }
 #[test] fn d3_model_binding_changes_defect(){
  let z=trajectory::initial();let b=[z;6];
  let a=dense::Curve::from_controls(&dense::Model::new(0.).unwrap(),3,0.,0.01,b).unwrap().certify(&[0.;30],100.).unwrap().diagnostics();
  let c=dense::Curve::from_controls(&dense::Model::new(4.).unwrap(),3,0.,0.01,b).unwrap().certify(&[0.;30],100.).unwrap().diagnostics();
  assert_eq!(a.alpha,0.);assert_eq!(c.alpha,4.);for j in 0..30{assert!(c.rho[j]>a.rho[j])}
 }
 #[test] fn d3_dense_refusal_preserves_accepted_prefix_and_budget(){
  let z=trajectory::initial();let mut s=admission::Session::new_nonnegative(0.,0.1,z,0.001,2).unwrap();let initial=s.snapshot();let radius=[0.01;30];
  for _ in 0..2 {let lease=s.begin().unwrap();let c=dense::Curve::new(&dense::Model::new(0.).unwrap(),3,initial.t,lease.next,&initial.y,&initial.y).unwrap();
   assert!(c.certify(&radius,1e-30).is_err());s.abort_stage(lease,true).unwrap();assert_eq!(s.snapshot(),initial);assert_eq!(radius,[0.01;30]);}
  assert_eq!(s.attempts(),2);assert_eq!(s.accepted(),0);assert_eq!(s.rejected(),2);assert!(matches!(s.begin(),Err(admission::Fault::Budget)));
 }
 #[test] fn d3_query_rounding_only_refusal_preserves_transaction(){
  let z=trajectory::initial();let mut s=admission::Session::new_nonnegative(0.,0.1,z,1e-5,4).unwrap();
  let before=s.snapshot();let incoming=[0.;30];let cursor=0usize;let history:Vec<f64>=Vec::new();let lease=s.begin().unwrap();
  let c=dense::Curve::new(&dense::Model::new(0.).unwrap(),3,before.t,lease.next,&before.y,&before.y).unwrap();
  let d=c.certify(&incoming,1.).unwrap().diagnostics();
  let dep2=directed::add_up(d.uniform[24],d.uniform[25]).unwrap();
  let dep3=directed::add_up(directed::add_up(d.uniform[26],d.uniform[27]).unwrap(),d.uniform[28]).unwrap();
  let peak=d.uniform.iter().copied().fold(0.,f64::max).max(dep2).max(dep3);
  let tight=c.certify(&incoming,peak).unwrap();
  assert!(matches!(tight.query(lease.next*0.37),Err(dense::DenseError::Budget{..})));
  s.abort_stage(lease,true).unwrap();
  assert_eq!(s.snapshot(),before);assert_eq!(incoming,[0.;30]);assert_eq!(cursor,0);assert!(history.is_empty());assert_eq!(s.attempts(),1);
 }
}
