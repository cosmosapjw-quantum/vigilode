//! Source-matched complete IVP benchmark; all algebraic predictors and intervals charged.
use serde_json::json;
use std::hint::black_box;
use std::time::Instant;
use vigilode_coupled::coupled_inverse_be::{Config,Order,Source};
fn measure(s:&Source,cfg:Config,reps:usize)->(u128,u64,u64,u64,u64){
    let start=Instant::now();let(mut calls,mut intervals,mut inverse,mut check)=(0u64,0u64,0u64,0u64);
    for _ in 0..reps {
        let r=black_box(s.run(black_box(cfg)));
        assert!(r.completed,"{:?}",r.failure);
        assert!(r.points.iter().all(|p|p.physical_error_upper<=cfg.physical_tolerance));
        calls+=r.work.map_attempted;
        intervals+=r.work.original_interval_components;
        inverse+=r.work.inverse_interval_components;
        for v in &r.points.last().unwrap().state{check=check.wrapping_add(v.to_bits());}
        black_box(&r);
    }
    (start.elapsed().as_nanos(),calls,intervals,inverse,check)
}
fn main(){
    let cases=[
        ("asymmetric_2",Source::new(vec![4.0,7.0],vec![20.0,40.0],vec![vec![0.0,12.0],vec![8.0,0.0]],vec![0.2,0.12]).unwrap(),64usize,0.1,0.00890),
        ("asymmetric_2_refined",Source::new(vec![4.0,7.0],vec![20.0,40.0],vec![vec![0.0,12.0],vec![8.0,0.0]],vec![0.2,0.12]).unwrap(),128usize,0.1,0.00445),
        ("coupled_3_cubic",Source::new(vec![6.0,8.0,10.0],vec![60.0,100.0,30.0],vec![vec![0.0,30.0,4.0],vec![15.0,0.0,10.0],vec![10.0,8.0,0.0]],vec![0.04,0.06,0.02]).unwrap(),128usize,0.1,0.00712),
        ("linear_exchange_control",Source::new(vec![4.0,4.0],vec![0.0,0.0],vec![vec![0.0,6.0],vec![6.0,0.0]],vec![0.1,0.05]).unwrap(),128usize,0.1,0.006),
    ];
    for (name,src,n,t,eps) in cases {
        let c=Config{steps:n,horizon:t,physical_tolerance:eps,max_map_evaluations:200000,max_iterations_per_step:90,order:Order::FastFirst};
        for order in [Order::BaselineFirst,Order::FastFirst] {
            let run=src.run(Config{order,..c});
            assert!(run.completed,"{:?}",run.failure);
            println!("VCB_TRACE={}",json!({
                "name":name,"fast":order==Order::FastFirst,"source_bits":src.source_bits(),
                "n":src.dimension(),"h_bits":(t/n as f64).to_bits(),"eps_bits":eps.to_bits(),
                "points":run.points.iter().map(|p| json!({
                    "time":p.time.to_bits(),"state":p.state.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),
                    "rho":p.original_be_residual_upper.to_bits(),"q":p.source_q_upper.to_bits(),
                    "E":p.physical_error_upper.to_bits(),"callbacks":p.map_evaluations
                })).collect::<Vec<_>>(),
                "map_attempted":run.work.map_attempted,"map_completed":run.work.map_completed,
                "original_components":run.work.original_interval_components,
                "inverse_components":run.work.inverse_interval_components,
                "algebraic_candidates":run.work.algebraic_cubic_candidates,
                "truncation_majorant":run.truncation_majorant.unwrap().to_bits(),
                "stage_original_residual_target":run.original_stage_residual_target.unwrap().to_bits()
            }));
        }
        black_box(measure(&src,c,3));black_box(measure(&src,Config{order:Order::BaselineFirst,..c},3));
        for block in 0..5 {
            let order=if block%2==0{[false,true,true,false]}else{[true,false,false,true]};
            for fast in order {
                let cfg=Config{order:if fast{Order::FastFirst}else{Order::BaselineFirst},..c};
                let (ns,calls,intervals,inverse,checksum)=measure(&src,cfg,20);
                println!("VCB_BENCH name={name} block={block} fast={fast} ns={ns} map={calls} original_intervals={intervals} inverse_intervals={inverse} checksum={checksum}");
            }
        }
    }
}
