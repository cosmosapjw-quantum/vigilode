//! Conditional fixed-P policies on declared mathematical fixtures; not ODE solves.
//! cargo run --profile measurement -p rodas5p-integrators --example
//! conditional_continuation_sweep -- --repeats 31 [--case ID] [--list]
use rodas5p_integrators::fixed_point_continuation::*;
use serde::Serialize;
use serde_json::json;
use std::{hint::black_box, time::Instant};

#[derive(Clone, Serialize)]
struct Case {
    id: String,
    n: usize,
    coefficient: f64,
    p: f64,
    tolerance: f64,
    evaluation_uncertainty: f64,
    iterations: usize,
    calls: usize,
    seed: f64,
    root_tolerance: Option<f64>,
    split_patch: bool,
}
impl Case {
    fn identity(&self) -> ContinuationIdentity {
        ContinuationIdentity { target_id: self.coefficient.to_bits(), model_epoch: self.n as u64, branch_id: 1 }
    }
    fn target(&self, i: usize, lambda: f64) -> f64 {
        0.25 + 0.5 * lambda + 0.0078125 * (i as f64 / self.n as f64)
    }
    fn exact_residual(&self, x: f64, i: usize, lambda: f64) -> f64 {
        x + self.coefficient * x * x * x - self.target(i, lambda)
    }
    fn q(&self, low: f64, high: f64) -> f64 {
        (1.0 - self.p * (1.0 + 3.0 * self.coefficient * low * low)).abs()
            .max((1.0 - self.p * (1.0 + 3.0 * self.coefficient * high * high)).abs())
    }
    fn atlas(&self) -> DeclaredFixedPointAtlas {
        let pieces = if self.split_patch {
            vec![(0.0, 0.5, 0.375, 0.375), (0.5, 1.0, 0.625, 0.375)]
        } else { vec![(0.0, 1.0, 0.5, 0.5)] };
        let patches = pieces.into_iter().enumerate().map(|(k, (a,b,c,r))| {
            // These fixture constants have wide slack; the derivation uses
            // all x in the ball and all lambda, not sampled contraction ratios.
            let beta = if self.split_patch { 0.18 * self.p } else { 0.27 * self.p };
            DeclaredFixedPointPatch::new(self.identity(), k as u64 + 1, self.p.to_bits(),
                a,b,vec![c;self.n],r,FixedPointBounds {
                    contraction_upper: self.q(c-r,c+r), center_correction_upper: beta,
                    inverse_action_norm_upper: self.p,
                    inverse_action_inverse_norm_upper: 1.0/self.p,
                }).unwrap()
        }).collect();
        DeclaredFixedPointAtlas::new(self.identity(), patches).unwrap()
    }
    fn config(&self, tolerance:f64) -> FixedPointContinuationConfig {
        FixedPointContinuationConfig { residual_tolerance:tolerance,
            max_iterations_per_station:self.iterations, max_residual_calls:self.calls,
            max_inverse_calls:self.calls, ..Default::default() }
    }
    fn f_error_ceiling(&self) -> f64 {
        self.evaluation_uncertainty + 64.0*f64::EPSILON
    }
    fn planning(&self, atlas:&DeclaredFixedPointAtlas) -> serde_json::Value {
        let b=atlas.patches()[0].bounds(); let q=b.contraction_upper;
        let ef=self.f_error_ceiling();
        // All iterates are in [0,1]; an accepted update is guarded by the kernel.
        let ep=8.0*f64::EPSILON*self.p+f64::MIN_POSITIVE;
        let rho=4.0*f64::EPSILON;
        let eta=ep+self.p*ef+rho;
        let floor=eta/(1.0-q);
        let lipschitz=(1.0+q)/self.p;
        let needed=(self.tolerance-2.0*ef)/lipschitz-floor;
        let k=if needed>0.0 {
            if needed>=1.0 {Some(0)} else if q==0.0 {Some(1)} else {
                Some((needed.ln()/q.ln()).ceil().max(0.0) as usize+2)
            }
        } else {None};
        json!({"q_upper":q,"evaluation_error_upper":ef,"inverse_error_upper":ep,
            "update_rounding_upper":rho,"iterate_perturbation_upper":eta,
            "asymptotic_root_error_sufficient_bound":floor,
            "sufficient_acceptance_floor":2.0*ef+lipschitz*floor,
            "unavoidable_reported_error_floor":self.evaluation_uncertainty,
            "iterations_planning_upper":k,
            "self_map_margin":(1.0-q)*atlas.patches()[0].radius()-b.center_correction_upper,
            "planning_only":"floating logarithms, generous roundoff constants; not an admission certificate or impossibility test"})
    }
}
impl FixedPointHomotopyProvider for Case {
    fn identity(&self)->ContinuationIdentity {self.identity()}
    fn dimension(&self)->usize {self.n}
    fn inverse_identity(&self,_:u64)->u64 {self.p.to_bits()}
    fn residual(&self,x:&[f64],lambda:f64,out:&mut[f64])->Result<f64,String> {
        let mut rounding=0.0_f64;
        for i in 0..self.n {
            // An actual deterministic evaluation bias exercises the enclosure;
            // the declared uncertainty conservatively dominates its magnitude.
            out[i]=self.exact_residual(x[i],i,lambda)+0.5*self.evaluation_uncertainty;
            rounding=rounding.max(32.0*f64::EPSILON*(x[i].abs()
                +self.coefficient*x[i].abs().powi(3)+self.target(i,lambda).abs()
                +self.evaluation_uncertainty));
        }
        Ok(self.evaluation_uncertainty+rounding)
    }
    fn apply_inverse(&self,_:u64,v:&[f64],out:&mut[f64])->Result<f64,String> {
        if self.p==1.0 {out.copy_from_slice(v);return Ok(0.0);}
        let mut error=f64::MIN_POSITIVE;
        for (a,b) in out.iter_mut().zip(v) {
            *a=self.p*b;error=error.max(2.0*f64::EPSILON*a.abs()+f64::MIN_POSITIVE);
        }
        Ok(error)
    }
}

#[derive(Serialize)]
struct Attempt {
    #[serde(skip)]
    seconds: f64,
    status: String,
    residual_upper: Option<f64>,
    root_bound: Option<f64>,
    true_residual_max: Option<f64>,
    error_to_reference: Option<f64>,
    output_unchanged_on_failure: bool,
    effective_tolerance: f64,
    seed_policy: &'static str,
    work: FixedPointContinuationWork,
}
fn continuation(case:&Case,atlas:&DeclaredFixedPointAtlas,conditional:bool,reference:&[f64])->Attempt {
    let timer=Instant::now();
    let mut work=FixedPointContinuationWork::default();
    let mut output=vec![-1234.0;case.n];
    let mut seed=vec![case.seed;case.n];
    let mut tolerance=case.tolerance;
    let mut seed_policy="supplied";
    if conditional {
        // This is fixture-specific a priori knowledge of a reported lower
        // bound, not a lower bound inferred from one noisy sample.
        if tolerance<case.evaluation_uncertainty {
            return Attempt {seconds:timer.elapsed().as_secs_f64(),status:"REFUSED_EVALUATION_FLOOR".into(),residual_upper:None,
                root_bound:None,true_residual_max:None,error_to_reference:None,
                output_unchanged_on_failure:true,effective_tolerance:tolerance,seed_policy,work};
        }
        let first=&atlas.patches()[0];
        if seed.iter().zip(first.center()).any(|(x,c)|!x.is_finite() || (x-c).abs()>=first.radius()) {
            seed.copy_from_slice(first.center());seed_policy="declared_patch_center_fallback";
        }
        if let Some(root_tol)=case.root_tolerance {
            // Reserve half the linear root-error budget for action errors.
            let q=atlas.patches().iter().map(|p|p.bounds().contraction_upper).fold(0.0,f64::max);
            tolerance=tolerance.min(0.5*(1.0-q)*root_tol/case.p);
        }
    }
    // Wrapper output is transactional even if its extra root-error requirement
    // refuses an otherwise acceptable residual result from the kernel.
    let mut candidate=output.clone();
    let result=continue_fixed_point(black_box(case),atlas,&case.config(tolerance),&seed,&mut candidate,&mut work);
    let (status,residual,root)=match result {
        Ok(report)=>{
            let root_ok=case.root_tolerance.is_none_or(|r|report.final_conditional_root_error_upper<=r);
            if root_ok {output.copy_from_slice(&candidate);}
            (if root_ok {"PASS_SCOPED"}else{"ROOT_BUDGET_NOT_MET"}.to_string(),
                Some(report.final_residual_upper),Some(report.final_conditional_root_error_upper))
        },
        Err(error)=>(match error {
            FixedPointContinuationError::Budget(_)=>"WORK_CAP",
            FixedPointContinuationError::IterationLimit{..}=>"ITERATION_CAP",
            FixedPointContinuationError::OutsidePatch{..}=>"OUTSIDE_PATCH",
            _=>"REFUSED_OTHER",
        }.to_string(),None,None),
    };
    let passed=status=="PASS_SCOPED";
    let seconds=timer.elapsed().as_secs_f64();
    Attempt {seconds,status,residual_upper:residual,root_bound:root,
        true_residual_max:passed.then(||output.iter().enumerate().map(|(i,&x)|case.exact_residual(x,i,1.0).abs()).fold(0.0,f64::max)),
        error_to_reference:passed.then(||output.iter().zip(reference).map(|(x,r)|(x-r).abs()).fold(0.0,f64::max)),
        output_unchanged_on_failure:passed||output.iter().all(|x|*x == -1234.0),
        effective_tolerance:tolerance,seed_policy,work}
}

// Strong scalar-structure comparator. It solves the final target only, carries
// the same evaluation-error allowance, and never uses ambiguous interval signs.
fn bisection(case:&Case,tolerance:f64,max_calls:usize,uncertain:bool)->(Vec<f64>,usize,bool,f64) {
    let mut lo=vec![0.0;case.n];let mut hi=vec![1.0;case.n];
    let mut out=vec![0.0;case.n];let mut f=out.clone();
    let mut bound=f64::INFINITY;
    for call in 1..=max_calls {
        for i in 0..case.n {out[i]=0.5*(lo[i]+hi[i]);}
        let error=if uncertain {case.residual(&out,1.0,&mut f).unwrap()} else {
            for i in 0..case.n {f[i]=case.exact_residual(out[i],i,1.0);}
            0.0
        };
        bound=f.iter().map(|x|x.abs()).fold(0.0,f64::max)+error;
        if bound<=tolerance {return(out,call,true,bound);}
        for i in 0..case.n {
            if f[i]>error {hi[i]=out[i];}
            else if f[i]< -error {lo[i]=out[i];}
            // Otherwise this component's midpoint enclosure contains zero.
            // Keep that bracket; no uncertified sign update is made.
        }
    }
    (out,max_calls,false,bound)
}
fn cases()->Vec<Case> {
    let mut cases=Vec::new();
    for n in [1,128] {for p in [1.0,0.0625] {for tolerance in [1e-6,1e-10] {
        cases.push(Case {id:format!("n{n}_p{p}_tol{tolerance}"),n,coefficient:0.015625,p,
            tolerance,evaluation_uncertainty:0.0,iterations:768,calls:2048,
            seed:0.25,root_tolerance:None,split_patch:false});
    }}}
    let base=cases[2].clone();
    cases.push(Case{id:"near_contraction_iteration_cap".into(),iterations:16,..base.clone()});
    cases.push(Case{id:"finite_cumulative_work_cap".into(),calls:3,..base.clone()});
    let vector=cases[4].clone();
    cases.push(Case{id:"outside_seed_center_fallback".into(),seed:2.0,..vector.clone()});
    cases.push(Case{id:"center_initial_iterate".into(),seed:0.5,..vector.clone()});
    cases.push(Case{id:"uncertain_evaluation_feasible".into(),evaluation_uncertainty:1e-8,..vector.clone()});
    cases.push(Case{id:"uncertain_evaluation_impossible".into(),evaluation_uncertainty:1e-8,
        tolerance:1e-10,iterations:32,..vector.clone()});
    cases.push(Case{id:"root_budget_refinement".into(),root_tolerance:Some(1e-10),..vector.clone()});
    cases.push(Case{id:"two_regular_patches".into(),split_patch:true,..vector});
    cases
}

struct TwoBranches;
impl FixedPointHomotopyProvider for TwoBranches {
    fn identity(&self)->ContinuationIdentity {ContinuationIdentity{target_id:901,model_epoch:1,branch_id:1}}
    fn dimension(&self)->usize {1}
    fn inverse_identity(&self,p:u64)->u64 {p+100}
    fn residual(&self,x:&[f64],lambda:f64,out:&mut[f64])->Result<f64,String> {
        out[0]=x[0]*x[0]-(1.0+0.25*lambda);Ok(16.0*f64::EPSILON*(x[0]*x[0]+1.25))
    }
    fn apply_inverse(&self,p:u64,v:&[f64],out:&mut[f64])->Result<f64,String> {
        out[0]=if p==1 {0.5*v[0]}else{-0.5*v[0]};Ok(f64::MIN_POSITIVE)
    }
}
fn bad_branch()->serde_json::Value {
    let p=TwoBranches;
    let patches=[(1,0.0,0.5,1.0),(2,0.5,1.0,-1.0)].into_iter().map(|(id,a,b,c)|
        DeclaredFixedPointPatch::new(p.identity(),id,id+100,a,b,vec![c],0.25,
            FixedPointBounds{contraction_upper:0.25,center_correction_upper:0.13,
                inverse_action_norm_upper:0.5,inverse_action_inverse_norm_upper:2.0}).unwrap()).collect();
    let atlas=DeclaredFixedPointAtlas::new(p.identity(),patches).unwrap();
    let mut output=[777.0];let mut work=FixedPointContinuationWork::default();
    let error=continue_fixed_point(&p,&atlas,&Default::default(),&[1.0],&mut output,&mut work).unwrap_err();
    let pass=matches!(error,FixedPointContinuationError::BranchOverlap{patch_id:2})&&output==[777.0];
    json!({"kind":"guard","case_id":"disjoint_branch_refusal","expected_refusal":true,
        "guard_pass":pass,"status":error.to_string(),"output_unchanged":output==[777.0],"work":work})
}
fn median(values:&mut[f64])->f64 {values.sort_by(f64::total_cmp);values[values.len()/2]}
fn main()->Result<(),String> {
    let args:Vec<String>=std::env::args().skip(1).collect();
    let mut repeats=31usize;let mut selected=None;let mut list=false;let mut i=0;
    while i<args.len() {match args[i].as_str() {
        "--list"=>list=true,
        "--repeats"=>{i+=1;repeats=args.get(i).ok_or("missing repeats")?.parse().map_err(|_|"invalid repeats")?;},
        "--case"=>{i+=1;selected=Some(args.get(i).ok_or("missing case")?.clone());},
        _=>return Err(format!("unknown argument {}",args[i])),
    }i+=1;}
    if repeats==0||repeats>10000 {return Err("repeats must be 1..10000".into());}
    let all=cases();
    if list {println!("{}",json!({"kind":"catalog","cases":all,"guard_case":"disjoint_branch_refusal"}));return Ok(());}
    if let Some(id)=&selected {if !all.iter().any(|c|c.id==*id)&&id!="disjoint_branch_refusal" {return Err("unknown case".into());}}
    println!("{}",json!({"kind":"header","schema_version":1,"study":"conditional_fixed_p_continuation",
        "repeats":repeats,"threads":1,"default_dispatch_changed":false,
        "scope":"root continuation only, not ODE trajectory accuracy; bisection is final-target-only with known monotone separable structure",
        "timing":"all solve allocations and policy guards included; immutable model/atlas construction, tight reference, diagnostic comparisons and JSON construction excluded",
        "evaluation_error":"biased callbacks use a declared enclosure; true model residual and independent tight bisection are diagnostic checks"}));
    for case in all.into_iter().filter(|c|selected.as_ref().is_none_or(|id|*id==c.id)) {
        let atlas=case.atlas();let(reference,_,ref_ok,_)=bisection(&case,2e-15,80,false);assert!(ref_ok);
        let mut times=[Vec::new(),Vec::new(),Vec::new()];
        let mut raw=None;let mut conditional=None;let mut comparator=None;
        for repeat in 0..repeats {for slot in 0..3 {
            let arm=(slot+repeat)%3;
            match arm {
                0=>{let a=continuation(&case,&atlas,false,&reference);times[arm].push(a.seconds);raw=Some(a);},
                1=>{let a=continuation(&case,&atlas,true,&reference);times[arm].push(a.seconds);conditional=Some(a);},
                _=>{let start=Instant::now();
                    let(o,c,ok,bound)=bisection(black_box(&case),case.tolerance,case.calls.min(128),true);
                    times[arm].push(start.elapsed().as_secs_f64());
                    comparator=Some(json!({"success":ok,"f_calls":c,"residual_upper":bound,
                        "error_to_reference":o.iter().zip(&reference).map(|(x,r)|(x-r).abs()).fold(0.0,f64::max)}));},
            }
        }}
        let medians=times.each_mut().map(|t|median(t));
        let raw=raw.unwrap();let conditional=conditional.unwrap();
        let true_ok=|a:&Attempt|a.status!="PASS_SCOPED" || (a.residual_upper.is_some_and(|x|x<=a.effective_tolerance)
            &&a.true_residual_max.is_some_and(|x|x<=a.effective_tolerance)
            &&a.output_unchanged_on_failure);
        assert!(true_ok(&raw)&&true_ok(&conditional)&&raw.output_unchanged_on_failure&&conditional.output_unchanged_on_failure);
        println!("{}",json!({"kind":"result","schema_version":1,"case_id":case.id,"config":case,
            "planning":case.planning(&atlas),"raw":raw,"conditional":conditional,"bisection_final_only":comparator,
            "seconds_median":{"raw":medians[0],"conditional":medians[1],"bisection_final_only":medians[2]},
            "conditional_over_raw_time":medians[1]/medians[0],
            "bisection_over_conditional_time":medians[2]/medians[1],
            "accuracy_contract_pass":true,
            "speed_comparison_eligible":raw.status=="PASS_SCOPED"&&conditional.status=="PASS_SCOPED"&&case.root_tolerance.is_none(),
            "warning":"short in-process timings are descriptive; refusals and changed root-error requirements are not speedups"}));
    }
    if selected.as_ref().is_none_or(|id|id=="disjoint_branch_refusal") {let row=bad_branch();assert_eq!(row["guard_pass"],true);println!("{row}");}
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conditional_floor_and_center_policy_keep_output_transactional() {
        for id in ["uncertain_evaluation_impossible","outside_seed_center_fallback","root_budget_refinement"] {
            let c=cases().into_iter().find(|c|c.id==id).unwrap();let atlas=c.atlas();
            let(r,_,ok,_)=bisection(&c,2e-15,80,false);assert!(ok);
            let a=continuation(&c,&atlas,true,&r);assert!(a.output_unchanged_on_failure);
            if id=="uncertain_evaluation_impossible" {assert_eq!(a.status,"REFUSED_EVALUATION_FLOOR");assert_eq!(a.work.residual_calls,0);}
            else {assert_eq!(a.status,"PASS_SCOPED");}
            if id=="root_budget_refinement" {assert!(a.root_bound.unwrap()<=c.root_tolerance.unwrap());}
        }
    }
    #[test]
    fn near_contraction_and_work_caps_refuse_without_output() {
        for id in ["near_contraction_iteration_cap","finite_cumulative_work_cap"] {
            let c=cases().into_iter().find(|c|c.id==id).unwrap();let atlas=c.atlas();
            let(r,_,_,_)=bisection(&c,2e-15,80,false);let a=continuation(&c,&atlas,true,&r);
            assert!(a.status=="ITERATION_CAP"||a.status=="WORK_CAP");
            assert!(a.work.residual_calls<=c.calls&&a.work.inverse_calls<=c.calls);
            assert!(a.output_unchanged_on_failure);
        }
    }
    #[test]
    fn distinct_roots_are_not_connected_by_identical_branch_labels() {
        assert_eq!(bad_branch()["guard_pass"],true);
    }
    #[test]
    fn changed_inverse_cannot_reuse_previous_patch_identity() {
        let original=cases()[0].clone();let atlas=original.atlas();
        let changed=Case{p:0.0625,..original};
        let mut output=[777.0];let mut work=FixedPointContinuationWork::default();
        let error=continue_fixed_point(&changed,&atlas,&changed.config(changed.tolerance),
            &[0.25],&mut output,&mut work).unwrap_err();
        assert_eq!(error,FixedPointContinuationError::IdentityMismatch);
        assert_eq!(output,[777.0]);assert_eq!(work.residual_calls,0);assert_eq!(work.inverse_calls,0);
    }
    #[test]
    fn all_default_successes_have_actual_final_residual_and_bounded_error() {
        for c in cases() {let atlas=c.atlas();let(r,_,ok,_)=bisection(&c,2e-15,80,false);assert!(ok);
            for policy in [false,true] {let a=continuation(&c,&atlas,policy,&r);
                if a.status=="PASS_SCOPED" {
                    assert!(a.true_residual_max.unwrap()<=a.effective_tolerance);
                    assert!(a.error_to_reference.unwrap()<=a.root_bound.unwrap()+3e-15);
                }
                assert!(a.output_unchanged_on_failure);
            }
        }
    }
}
