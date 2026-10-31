use vigilode_coupled::nonlinear_be::{Source,Config,SearchOrder,FailureKind};

fn source()->Source{Source::new(100.0,1e6,0.01).unwrap()}
fn cfg(order:SearchOrder)->Config{Config{steps:64,horizon:0.1,physical_tolerance:0.0015,
    max_source_evaluations:100_000,max_iterations_per_step:120,order}}
fn exact(k:f64,g:f64,y:f64,t:f64)->f64{
    let e=(-k*t).exp();
    y*e/(1.0+(g/k)*y*y*(1.0-e*e)).sqrt()
}
#[test]
fn nonlinear_full_ivp_source_certified_physics_and_fewer_evaluations(){
 let s=source();
 let a=s.run(cfg(SearchOrder::BaselineFirst));
 let b=s.run(cfg(SearchOrder::FastFirst));
 assert!(a.completed&&b.completed,"{:?} {:?}",a.failure,b.failure);
 assert_eq!(a.points.len(),65);assert_eq!(b.points.len(),65);
 assert_eq!(a.work.rhs_attempted,a.work.interval_residual_evaluations);
 assert_eq!(b.work.rhs_attempted,b.work.interval_residual_evaluations);
 assert!(b.work.rhs_attempted<a.work.rhs_attempted);
 for arm in [&a,&b]{
   for point in &arm.points {
    let error=(point.state-exact(100.0,1e6,0.01,point.time)).abs();
    assert!(error<=point.physical_error_upper+2e-15,"{point:?}");
    assert!(point.physical_error_upper<=0.0015);
   }
   for w in arm.points.windows(2){assert!(w[1].state>=0.0&&w[1].state<=w[0].state);}
 }
 println!("NLB_NATIVE_FULL_IVP baseline_rhs={} fast_rhs={} baseline_interval={} fast_interval={}",
  a.work.rhs_attempted,b.work.rhs_attempted,a.work.interval_residual_evaluations,b.work.interval_residual_evaluations);
}
#[test]
fn nonlinear_dissipation_respects_the_scaled_stiff_parameter_source(){
 let source=Source::new(1000.0,1e9,0.001).unwrap();
 let old=source.run(Config{steps:64,horizon:0.01,physical_tolerance:0.00015,
  ..cfg(SearchOrder::BaselineFirst)});
 let fast=source.run(Config{steps:64,horizon:0.01,physical_tolerance:0.00015,
  ..cfg(SearchOrder::FastFirst)});
 assert!(old.completed&&fast.completed,"{:?} {:?}",old.failure,fast.failure);
 assert!(fast.work.rhs_attempted<old.work.rhs_attempted);
 for p in &fast.points{
  assert!(p.physical_error_upper<=0.00015);
  assert!((p.state-exact(1000.0,1e9,0.001,p.time)).abs()<=p.physical_error_upper+1e-15);
 }
}
#[test]
fn mixed_nonlinear_rate_positive_decreasing_trajectory_and_physical_accuracy(){
 let src=Source::new(10.0,1e5,0.01).unwrap();
 let b=src.run(Config{steps:32,horizon:0.3,physical_tolerance:0.0009,..cfg(SearchOrder::FastFirst)});
 assert!(b.completed,"{:?}",b.failure);
 for p in b.points.iter().skip(1){assert!(p.physical_error_upper<=0.0009);}
 assert_eq!(b.work.source_steps,32);
}
#[test]
fn invalid_physical_target_fails_closed_before_numerical_callback(){
 let src=Source::new(100.0,1e7,0.01).unwrap();
 let out=src.run(Config{steps:64,horizon:0.1,physical_tolerance:0.01,..cfg(SearchOrder::FastFirst)});
 assert!(!out.completed);assert_eq!(out.failure.unwrap().kind,FailureKind::Truncation);
 assert_eq!(out.work.rhs_attempted,0);assert!(out.points.is_empty());
}
#[test]
fn invalid_physical_grid_and_source_are_rejected_without_work(){
 for (k,g,y) in [(0.0,1.0,1.0),(1.0,-1.0,1.0),(1.0,0.0,0.0),
  (f64::NAN,0.0,1.0)] {assert_eq!(Source::new(k,g,y).unwrap_err().kind,FailureKind::Input);}
 let src=source();
 for cfg in [Config{steps:63,..cfg(SearchOrder::FastFirst)},
  Config{steps:0,..cfg(SearchOrder::FastFirst)},
  Config{max_source_evaluations:0,..cfg(SearchOrder::FastFirst)}] {
  let out=src.run(cfg);assert_eq!(out.failure.unwrap().kind,FailureKind::Input);
  assert_eq!(out.work.rhs_attempted,0);
 }
}
#[test]
fn resource_refusals_preserve_exact_attempted_work_and_prefix(){
 let src=source();
 for cap in [1,2,3]{
  let out=src.run(Config{max_source_evaluations:cap,..cfg(SearchOrder::FastFirst)});
  assert!(!out.completed);assert_eq!(out.failure.unwrap().kind,FailureKind::Budget);
  assert_eq!(out.work.rhs_attempted,cap);assert_eq!(out.work.rhs_completed,cap);
  assert!(out.points.len()<=2);
 }
}
#[test]
fn exactly_zero_cubic_term_is_source_stable_and_no_fitted_q(){
 let src=Source::new(1000.0,0.0,0.01).unwrap();
 let out=src.run(Config{steps:512,horizon:0.1,physical_tolerance:0.002,..cfg(SearchOrder::FastFirst)});
 assert!(out.completed,"{:?}",out.failure);
 for p in out.points{assert_eq!(p.source_q_upper,0.0);assert!(p.physical_error_upper<=0.002);}
}
