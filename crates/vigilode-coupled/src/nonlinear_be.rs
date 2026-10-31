//! Opt-in VigilODE scalar nonlinear value-only, source-owned stiff decay lane.
//!
//! y'=-k*y-g*y^3, k>0, g>=0, y(0)>0. The source is copied and private.
//! Backward Euler is L-stable in its linear limit; Picard sees the
//! analytically isolated linear part and evaluates only cubic source values.
//! Each represented stage and full-IVP physical error has a directed outward
//! original-source certificate. Not the general IVP or RODAS5P production path.

use rodas5p_core::directed::{
    Interval, add_up, div_up, mul_down, mul_up, sub_down,
};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SearchOrder { BaselineFirst, FastFirst }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum FailureKind { Input, NoContraction, Truncation, Budget, Domain, Arithmetic, Iteration, Physical }
#[derive(Clone, Debug, Serialize)]
pub struct Failure { pub kind: FailureKind, pub reason: String }
impl Failure { fn new(kind: FailureKind, reason: impl Into<String>) -> Self { Self {kind,reason:reason.into()} } }
fn arithmetic(e: impl std::fmt::Display) -> Failure { Failure::new(FailureKind::Arithmetic, e.to_string()) }
fn ip(x:f64)->Result<Interval,Failure>{Interval::point(x).map_err(arithmetic)}
fn up(a:f64,b:f64)->Result<f64,Failure>{add_up(a,b).map_err(arithmetic)}
fn times(a:f64,b:f64)->Result<f64,Failure>{mul_up(a,b).map_err(arithmetic)}

#[derive(Clone,Copy,Debug,Serialize)]
pub struct Config {
    pub steps:usize,
    pub horizon:f64,
    pub physical_tolerance:f64,
    pub max_source_evaluations:u64,
    pub max_iterations_per_step:usize,
    pub order:SearchOrder,
}
impl Default for Config {
    fn default()->Self { Self {steps:64,horizon:0.1,physical_tolerance:0.0015,
        max_source_evaluations:100_000,max_iterations_per_step:120,order:SearchOrder::FastFirst} }
}
#[derive(Clone,Debug,Default,Serialize)]
pub struct Work { pub rhs_attempted:u64,pub rhs_completed:u64,pub interval_residual_evaluations:u64,
    pub speculative_trials:u64,pub admitted_trials:u64,pub picard_candidates:u64,
    pub refused_trials:u64,pub source_steps:u64 }
#[derive(Clone,Debug,Serialize)]
pub struct Point {pub time:f64,pub state:f64,pub source_q_upper:f64,
    pub original_residual_upper:f64,pub physical_error_upper:f64,
    pub rhs_step:u64}
#[derive(Clone,Debug,Default,Serialize)]
pub struct Attempt {pub completed:bool,pub points:Vec<Point>,pub work:Work,
    pub failure:Option<Failure>,pub truncation_budget_upper:Option<f64>,
    pub stage_original_residual_limit:Option<f64>}
#[derive(Clone,Debug)]
pub struct Source {k:f64,g:f64,y0:f64}
#[derive(Clone,Copy)]
struct Val {x:f64,mapped:f64,op_res:f64,low:f64,high:f64}
struct Stage{y:f64,a:Interval,b:Interval,u:f64,q:f64,op_u:f64,op_delta:f64,residual_limit:f64}

impl Source {
    pub fn new(k:f64,g:f64,y0:f64)->Result<Self,Failure>{
        if ![k,g,y0].iter().all(|v|v.is_finite())||k<=0.0||g<0.0||y0<=0.0 {
            return Err(Failure::new(FailureKind::Input,"invalid source-owned cubic parameters"));
        }
        Ok(Self{k,g,y0})
    }
    pub fn source_bits(&self)->[u64;3]{[self.k.to_bits(),self.g.to_bits(),self.y0.to_bits()]}
    pub fn run(&self,cfg:Config)->Attempt{
        let mut out=Attempt::default();
        let result=self.integrate(cfg,&mut out);
        match result {Ok(())=>out.completed=true,Err(e)=>out.failure=Some(e)}
        out
    }
    fn integrate(&self,cfg:Config,out:&mut Attempt)->Result<(),Failure>{
        if !cfg.steps.is_power_of_two()||cfg.steps>(1_usize<<52)||cfg.max_source_evaluations==0
          ||cfg.max_iterations_per_step==0||!cfg.horizon.is_finite()||cfg.horizon<=0.0
          ||!cfg.physical_tolerance.is_finite()||cfg.physical_tolerance<=0.0 {
            return Err(Failure::new(FailureKind::Input,"invalid physical grid, budget or work"));
        }
        let h=cfg.horizon/cfg.steps as f64;
        if h<=0.0||!h.is_finite()||h*(cfg.steps as f64)!=cfg.horizon {
            return Err(Failure::new(FailureKind::Input,"horizon not an exactly represented dyadic grid"));
        }
        let h_i=ip(h)?;
        let k_i=ip(self.k)?;
        let g_i=ip(self.g)?;
        let a=ip(1.0)?.add(h_i.mul(k_i).map_err(arithmetic)?).map_err(arithmetic)?;
        let b=h_i.mul(g_i).map_err(arithmetic)?;
        let a_lo=a.lo;
        if !(a_lo>1.0){return Err(Failure::new(FailureKind::Arithmetic,"a_lower not >1"));}
        let p0=up(times(self.k,self.y0)?,times(self.g,times(self.y0,times(self.y0,self.y0)?)?)?)?;
        let mut major=self.y0;
        let mut t_bound=0.0;
        let mut max_tc=0.0_f64;
        let mut local_defects=Vec::with_capacity(cfg.steps);
        let mut clock_bound=Vec::with_capacity(cfg.steps);
        for i in 1..=cfg.steps{
            let major2=times(major,major)?;
            let l=up(self.k,times(3.0,times(self.g,major2)?)?)?;
            let p=up(times(self.k,major)?,times(self.g,times(major,major2)?)?)?;
            let defect=mul_up(mul_up(h,h).map_err(arithmetic)?,times(l,p)?).and_then(|z|div_up(z,2.0)).map_err(arithmetic)?;
            t_bound=div_up(up(t_bound,defect)?,a_lo).map_err(arithmetic)?;
            let label=h*(i as f64);
            let ti=h_i.scale(i as f64).map_err(arithmetic)?;
            let displacement=ti.sub(ip(label)?).map_err(arithmetic)?.mag();
            let clock=times(p0,displacement)?;
            max_tc=max_tc.max(up(t_bound,clock)?);
            local_defects.push(defect);
            clock_bound.push(clock);
            major=div_up(major,a_lo).map_err(arithmetic)?;
        }
        out.truncation_budget_upper=Some(max_tc);
        if max_tc>=cfg.physical_tolerance {
            return Err(Failure::new(FailureKind::Truncation,"global source truncation plus clock exceeds caller physical accuracy"));
        }
        let rem=sub_down(cfg.physical_tolerance,max_tc).map_err(arithmetic)?;
        let kh=mul_down(self.k,h).map_err(arithmetic)?;
        let residual_limit=mul_down(rem,kh).map_err(arithmetic)?;
        if residual_limit<=0.0 {
            return Err(Failure::new(FailureKind::Truncation,"no positive algebraic residual budget"));
        }
        out.stage_original_residual_limit=Some(residual_limit);
        let mut y=self.y0;
        let mut physical=0.0;
        out.points.push(Point{time:0.0,state:y,source_q_upper:0.0,
            original_residual_upper:0.0,physical_error_upper:0.0,rhs_step:0});
        for i in 0..cfg.steps {
            let prior=out.work.rhs_attempted;
            let stage=Stage::new(self,h,y,a,b,residual_limit)?;
            let finalval=stage.solve(cfg,&mut out.work)?;
            physical=div_up(up(up(physical,local_defects[i])?,finalval.high)?,a_lo).map_err(arithmetic)?;
            let physical_at_label=up(physical,clock_bound[i])?;
            if physical_at_label>cfg.physical_tolerance {
                return Err(Failure::new(FailureKind::Physical,"directed whole-IVP error above requested physical budget"));
            }
            if finalval.x<0.0||finalval.x>y||!finalval.x.is_finite(){
                return Err(Failure::new(FailureKind::Domain,"accepted full-IVP state not positive decreasing"));
            }
            y=finalval.x;
            out.work.source_steps+=1;
            out.points.push(Point{time:h*(i as f64+1.0),state:y,
                source_q_upper:stage.q,original_residual_upper:finalval.high,
                physical_error_upper:physical_at_label,
                rhs_step:out.work.rhs_attempted-prior});
        }
        Ok(())
    }
}

impl Stage {
    fn new(src:&Source,h:f64,y:f64,a:Interval,b:Interval,residual_limit:f64)->Result<Stage,Failure>{
        if !y.is_finite()||y<0.0{return Err(Failure::new(FailureKind::Domain,"source y outside positive domain"));}
        let u=div_up(y,a.lo).map_err(arithmetic)?;
        let u2=times(u,u)?;
        let q=div_up(times(times(3.0,b.hi)?,u2)?,a.lo).map_err(arithmetic)?;
        if !(q<1.0){return Err(Failure::new(FailureKind::NoContraction,"nonlinear source q>=1"));}
        // Check the extended *represented* domain, not the exact unrounded U.
        let zero_image=ip(y)?.div(a).map_err(arithmetic)?;
        let u_i=ip(u)?;
        let image_at_u=zero_image.sub(b.div(a).map_err(arithmetic)?
            .mul(u_i.mul(u_i).map_err(arithmetic)?.mul(u_i).map_err(arithmetic)?).map_err(arithmetic)?).map_err(arithmetic)?;
        if image_at_u.lo<0.0||zero_image.hi>u {
            return Err(Failure::new(FailureKind::NoContraction,"invariant interval not certified"));
        }
        let op_a=1.0+h*src.k;
        let op_u=y/op_a;
        let op_delta=(h*src.g)/op_a;
        if !op_u.is_finite()||!op_delta.is_finite(){return Err(Failure::new(FailureKind::Arithmetic,"operational source map overflow"));}
        Ok(Stage{y,a,b,u,q,op_u,op_delta,residual_limit})
    }
    fn evaluate(&self,x:f64,cfg:Config,work:&mut Work)->Result<Val,Failure>{
        if !x.is_finite()||x<0.0||x>self.u {return Err(Failure::new(FailureKind::Domain,"candidate outside invariant domain"));}
        if work.rhs_attempted>=cfg.max_source_evaluations {
            return Err(Failure::new(FailureKind::Budget,"global source dispatch budget exceeded"));
        }
        work.rhs_attempted+=1;
        let mapped=self.op_u-self.op_delta*x*x*x;
        if !mapped.is_finite(){return Err(Failure::new(FailureKind::Arithmetic,"operational RHS overflow"));}
        let mapped=mapped.clamp(0.0,self.u);
        let z=ip(x)?;
        let z3=z.mul(z).and_then(|v|v.mul(z)).map_err(arithmetic)?;
        let r=self.a.mul(z).and_then(|v|v.add(self.b.mul(z3)?)).and_then(|v|v.sub(Interval::point(self.y)?)).map_err(arithmetic)?;
        work.rhs_completed+=1;
        work.interval_residual_evaluations+=1;
        Ok(Val{x,mapped,op_res:mapped-x,low:r.mig(),high:r.mag()})
    }
    fn solve(&self,cfg:Config,work:&mut Work)->Result<Val,Failure>{
        let mut current=self.evaluate(0.0,cfg,work)?;
        let mut previous:Option<Val>=None;
        for _ in 0..cfg.max_iterations_per_step {
            if current.high<=self.residual_limit{return Ok(current);}
            let mut baseline=None;
            let mut chosen=None;
            if cfg.order==SearchOrder::BaselineFirst {
                work.picard_candidates+=1;
                let p=self.evaluate(current.mapped,cfg,work)?;
                if p.high<=self.residual_limit{chosen=Some(p);} else {baseline=Some(p);}
            }
            if chosen.is_none(){
                if let Some(old)=previous {
                    let rd=current.op_res-old.op_res;
                    if rd!=0.0&&rd.is_finite(){
                        let gamma=current.op_res/rd;
                        let trial=current.mapped-gamma*(current.mapped-old.mapped);
                        if trial.is_finite()&&trial>=0.0&&trial<=self.u {
                            if cfg.order==SearchOrder::BaselineFirst || cfg.max_source_evaluations.saturating_sub(work.rhs_attempted)>=2 {
                                work.speculative_trials+=1;
                                let candidate=self.evaluate(trial,cfg,work)?;
                                let verified=match cfg.order{
                                    SearchOrder::BaselineFirst=>candidate.high<=baseline.expect("baseline-first candidate").low,
                                    SearchOrder::FastFirst=>candidate.high<=self.residual_limit ||
                                         candidate.high<=mul_down(self.q,current.low).map_err(arithmetic)?,
                                };
                                if verified {work.admitted_trials+=1;chosen=Some(candidate);}
                                else {work.refused_trials+=1;}
                            }
                        }
                    }
                }
            }
            if chosen.is_none() {
                if let Some(b)=baseline {chosen=Some(b);} else {
                    work.picard_candidates+=1;
                    chosen=Some(self.evaluate(current.mapped,cfg,work)?);
                }
            }
            previous=Some(current);
            current=chosen.expect("every successful stage update must select a value");
        }
        Err(Failure::new(FailureKind::Iteration,"source-backed nonlinear iteration cap"))
    }
}
