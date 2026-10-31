use vigilode_coupled::coupled_inverse_be::{Config, FailureKind, Order, Source};

fn coupled3() -> Source {
    Source::new(
        vec![6.0, 8.0, 10.0], vec![60.0, 100.0, 30.0],
        vec![vec![0.0, 30.0, 4.0], vec![15.0, 0.0, 10.0], vec![10.0, 8.0, 0.0]],
        vec![0.04, 0.06, 0.02],
    ).unwrap()
}
fn cfg(order: Order) -> Config {
    Config { steps: 128, horizon: 0.1, physical_tolerance: 0.00712,
        max_map_evaluations: 100_000, max_iterations_per_step: 90, order }
}
#[test]
fn coupled_three_nonlinear_full_ivp_passes_same_original_physical_budget() {
    let src=coupled3();
    let a=src.run(cfg(Order::BaselineFirst));
    let b=src.run(cfg(Order::FastFirst));
    assert!(a.completed && b.completed,"baseline={:?} fast={:?}",a.failure,b.failure);
    assert_eq!(a.points.len(),129);assert_eq!(b.points.len(),129);
    assert!(b.work.map_attempted<a.work.map_attempted,"{} vs {}",b.work.map_attempted,a.work.map_attempted);
    for r in [&a,&b] {
        assert_eq!(r.work.map_attempted,r.work.map_completed);
        assert_eq!(r.work.inverse_interval_components,3*r.work.map_completed);
        assert_eq!(r.work.original_interval_components,3*r.work.map_completed);
        assert_eq!(r.work.source_steps,128);
        for p in &r.points {
            assert!(p.physical_error_upper<=cfg(Order::FastFirst).physical_tolerance);
            assert!(p.state.iter().all(|v|v.is_finite()&&*v>=0.0&&*v<=0.06));
            assert!(p.source_q_upper<1.0);
        }
    }
    // This is genuinely coupled: unequal populations transfer through source c_ij.
    assert!(b.points[1].state[0]!=a.points[0].state[0]);
    assert!(b.work.algebraic_cubic_candidates>0);
    println!("COUPLED_NATIVE baseline={} fast={} gamma_accepted={} phys={:.17e}",a.work.map_attempted,b.work.map_attempted,b.work.accepted_trials,b.points.last().unwrap().physical_error_upper);
}
#[test]
fn coupled_two_mode_asymmetric_stiff_decay_has_bounded_global_error() {
    let s=Source::new(vec![4.0,7.0],vec![20.0,40.0],vec![vec![0.0,12.0],vec![8.0,0.0]],vec![0.2,0.12]).unwrap();
    let a=s.run(Config{steps:64,horizon:0.1,physical_tolerance:0.00890,..cfg(Order::BaselineFirst)});
    let b=s.run(Config{steps:64,horizon:0.1,physical_tolerance:0.00890,..cfg(Order::FastFirst)});
    assert!(a.completed && b.completed,"{:?} {:?}",a.failure,b.failure);
    assert!(b.work.map_attempted<a.work.map_attempted);
    assert!(a.points.iter().all(|p|p.physical_error_upper<=0.00890));
    assert!(b.points.iter().all(|p|p.physical_error_upper<=0.00890));
    assert!(b.points.last().unwrap().original_be_residual_upper>0.0);
}
#[test]
fn coupled_linear_two_mode_exact_eigenmodes_independent_oracle() {
    let k=4.0;let c=6.0;let t=0.1;
    let s=Source::new(vec![k,k],vec![0.0,0.0],vec![vec![0.0,c],vec![c,0.0]],vec![0.1,0.05]).unwrap();
    let r=s.run(Config{steps:128,horizon:t,physical_tolerance:0.006,..cfg(Order::FastFirst)});
    assert!(r.completed,"{:?}",r.failure);
    for p in &r.points {
        let m=0.075*(-k*p.time).exp();
        let d=0.025*(-(k+2.0*c)*p.time).exp();
        let exact=[m+d,m-d];
        for (v,t) in p.state.iter().zip(exact) {
            assert!((v-t).abs()<=p.physical_error_upper+2e-14);
        }
        assert!(p.physical_error_upper<=0.006);
    }
    assert_eq!(r.work.algebraic_cubic_candidates,0);
}
#[test]
fn source_copies_all_coefficients_and_rejects_external_mutations() {
    let mut c=vec![vec![0.0,12.0],vec![8.0,0.0]];
    let mut v=vec![0.2,0.12];
    let s=Source::new(vec![4.0,7.0],vec![20.0,40.0],c.clone(),v.clone()).unwrap();
    let prior=s.source_bits();
    c[0][1]=0.0;v[0]=0.0;
    assert_eq!(s.source_bits(),prior);
    assert_eq!(prior.len(),2*3+4);
    assert_eq!(prior[6+1],12.0_f64.to_bits());
}
#[test]
fn invalid_signs_coupling_and_nonfinite_source_never_admit() {
    for (k,g,c,y) in [
        (vec![0.0,1.0],vec![1.0,0.0],vec![vec![0.0,1.0],vec![1.0,0.0]],vec![0.1,0.2]),
        (vec![1.0,1.0],vec![-1.0,0.0],vec![vec![0.0,1.0],vec![1.0,0.0]],vec![0.1,0.2]),
        (vec![1.0,1.0],vec![1.0,0.0],vec![vec![0.0,-1.0],vec![1.0,0.0]],vec![0.1,0.2]),
        (vec![1.0,1.0],vec![1.0,0.0],vec![vec![1.0,0.0],vec![1.0,0.0]],vec![0.1,0.2]),
        (vec![1.0,1.0],vec![1.0,0.0],vec![vec![0.0,1.0],vec![1.0,0.0]],vec![f64::NAN,0.2]),
    ] {
        assert_eq!(Source::new(k,g,c,y).unwrap_err().kind,FailureKind::Input);
    }
}
#[test]
fn impossible_physical_target_fails_before_callback() {
    let out=coupled3().run(Config{physical_tolerance:1e-9,..cfg(Order::FastFirst)});
    assert!(!out.completed);
    assert_eq!(out.failure.unwrap().kind,FailureKind::Truncation);
    assert_eq!(out.work.map_attempted,0);
    assert!(out.points.is_empty());
}
#[test]
fn hard_source_budget_preserves_attempted_work_and_accepted_prefix() {
    let s=coupled3();
    for cap in [1,2,3,4,5] {
        let out=s.run(Config{max_map_evaluations:cap,..cfg(Order::FastFirst)});
        assert!(!out.completed,"cap {cap}");
        assert_eq!(out.failure.unwrap().kind,FailureKind::Budget);
        assert_eq!(out.work.map_attempted,cap);
        assert!(out.work.map_completed<=cap);
        assert!(out.points.len()<=2);
    }
}
#[test]
fn invalid_grid_and_positive_domain_inputs_fail_without_source_work() {
    let s=coupled3();
    for bad in [
        Config{steps:0,..cfg(Order::FastFirst)},
        Config{steps:3,..cfg(Order::FastFirst)},
        Config{physical_tolerance:0.0,..cfg(Order::FastFirst)},
        Config{horizon:f64::NAN,..cfg(Order::FastFirst)},
        Config{max_iterations_per_step:0,..cfg(Order::FastFirst)},
    ] {
        let x=s.run(bad);
        assert!(!x.completed);assert_eq!(x.failure.unwrap().kind,FailureKind::Input);
        assert_eq!(x.work.map_attempted,0);
    }
}
