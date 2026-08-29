use rodas5p_fair_ab::{AccuracyBudgetVerdict as V, assess_error_budget};
#[test] fn accurate_but_policy_sensitive_is_not_accuracy_failure(){
 let (ec,ed,d)=(0.001,0.002,0.001);assert!(d>0.1*ed);
 assert_eq!(assess_error_budget(ec,0.0,Some(0.01)).unwrap().verdict,V::WithinBudget);
 assert_eq!(assess_error_budget(ed,0.0,Some(0.01)).unwrap().verdict,V::WithinBudget);
}
#[test] fn equal_bad_trajectories_do_not_certify_accuracy(){
 let d=0.0;assert!(d<=0.1*10.0);assert_eq!(assess_error_budget(10.0,0.0,Some(0.01)).unwrap().verdict,V::OutsideBudget);
}
#[test] fn missing_budget_is_not_pass(){assert_eq!(assess_error_budget(0.0,0.0,None).unwrap().verdict,V::BudgetNotSpecified);}
#[test] fn uncertain_reference_does_not_pass_boundary(){let r=assess_error_budget(0.9,0.2,Some(1.0)).unwrap();assert_eq!(r.verdict,V::ReferenceUnresolved);assert_eq!(r.lower_error_wrms,0.7);assert_eq!(r.upper_error_wrms,1.1);}
#[test] fn nonfinite_negative_and_overflow_rejected(){for v in [f64::NAN,f64::INFINITY,-1.0]{assert!(assess_error_budget(v,0.0,Some(1.0)).is_err());assert!(assess_error_budget(1.0,v,Some(1.0)).is_err());assert!(assess_error_budget(1.0,0.0,Some(v)).is_err());}assert!(assess_error_budget(f64::MAX,f64::MAX,None).is_err());}
#[test] fn signed_zero_zero_budget_is_well_defined(){assert_eq!(assess_error_budget(-0.0,0.0,Some(0.0)).unwrap().verdict,V::WithinBudget);}
#[test] fn scalar_error_gap_is_not_trajectory_distance(){let (a,b)=(1.0_f64,-1.0_f64);assert_eq!((a.abs()-b.abs()).abs(),0.0);assert_eq!((a-b).abs(),2.0);}
#[test] fn boundary_and_lower_bound_are_distinct(){assert_eq!(assess_error_budget(0.8,0.2,Some(1.0)).unwrap().verdict,V::WithinBudget);assert_eq!(assess_error_budget(1.3,0.2,Some(1.0)).unwrap().verdict,V::OutsideBudget);}
