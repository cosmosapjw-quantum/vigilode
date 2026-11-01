//! Original-source REI 46,080-cell five-clock photoionization client for VigilODE.
//! Reuses the *existing* matrix-free Rust RODAS5P implementation (PR #173).
//! This is NOT original 26-event coupled H/He thermal/OTS evolution and it does
//! NOT certify original REI first canonical interval or claim production readiness.
use std::{io::Write,process::ExitCode,sync::Arc};
use rodas5p_core::{LinearMethod,LinearSolverConfig};
use rodas5p_integrators::{
    AdaptiveStepConfig,OdeProblem,OutputSamplingPlan,OutputSchedule,
    integrate_sequential_matrix_free_adaptive_dense_observed,
};
const SOURCE:&str=include_str!("../../../exploratory/rei_igm_timeforced_photoionization_20261101/five_clock_pr342_original_source.json");
const REFERENCE:&str=include_str!("../../../exploratory/rei_igm_timeforced_photoionization_20261101/reference_trajectory_15state.json");
const DIM:usize=15;

#[derive(Clone)]
struct Source {
    s0:[f64;3], h:f64, energies:[f64;3],
    times:[f64;5], j:[[f64;4];5], w:[[[f64;4];4];5],
}
impl Source {
    fn new()->Self {
        let data:serde_json::Value=serde_json::from_str(SOURCE).expect("frozen source JSON");
        assert_eq!(data["N"].as_u64(),Some(46080));
        assert_eq!(data["source_pr332_artifact_sha256"].as_str(),Some("83d94b5383ac7c1f36d89486e83bf8b4a56dd1993a25af3ae1f8fed06a695ba9"));
        assert_eq!(data["source_pr342_artifact_sha256"].as_str(),Some("88f7e5bb1c6c44c0ca0c7f2ef2c81ac70098cc7ed773096868b1e7b4db25c035"));
        let s0=std::array::from_fn(|s|data["S0_particles"][s].as_f64().unwrap());
        let energies=std::array::from_fn(|s|data["ionization_potentials_eV"][s].as_f64().unwrap());
        let h=data["h_seconds"].as_f64().unwrap();
        let times=std::array::from_fn(|i|data["sites"][i]["u"].as_f64().unwrap());
        let j=std::array::from_fn(|i|std::array::from_fn(|g|data["sites"][i]["J_per_s"][g].as_f64().unwrap()));
        let w=std::array::from_fn(|i|std::array::from_fn(|g|std::array::from_fn(|s|data["sites"][i]["weights"][g][s].as_f64().unwrap())));
        for i in 1..5 {assert!(times[i]>times[i-1]);}
        assert!((times[0]).abs()<1e-14 && (times[4]-1.).abs()<1e-13);
        for i in 0..5 {for g in 0..4 {
            let norm:f64=w[i][g].iter().sum();assert!((norm-1.).abs()<5e-14);
            assert!(j[i][g]>=0. && w[i][g].iter().all(|x|*x>=0.));
        }}
        Self{s0,h,energies,times,j,w}
    }
    fn forcing(&self,u:f64,frozen:bool)->([f64;4],[[f64;4];4]){
        if frozen {return (self.j[0],self.w[0]);}
        assert!(u.is_finite() && u>=-1e-13 && u<=1.+1e-13);
        let mut k=3;for i in 0..4 {if u<self.times[i+1]{k=i;break;}}
        let l=((u-self.times[k])/(self.times[k+1]-self.times[k])).clamp(0.,1.);
        let currents=std::array::from_fn(|g|self.j[k][g]*(1.-l)+self.j[k+1][g]*l);
        let weights=std::array::from_fn(|g|std::array::from_fn(|s|self.w[k][g][s]*(1.-l)+self.w[k+1][g][s]*l));
        (currents,weights)
    }
    fn shares(&self,u:f64,y:&[f64],frozen:bool)->([f64;3],[f64;4],[f64;4],[[f64;3];4]) {
        assert_eq!(y.len(),DIM);
        assert!(y[0..3].iter().all(|x|x.is_finite()&&*x>0.));
        let (j,weights)=self.forcing(u,frozen);
        let mut photo=[0.;3];let mut esc=[0.;4];let mut pp=[[0.;3];4];
        for g in 0..4 {
            let den=weights[g][3]+(0..3).map(|s|weights[g][s]*y[s]).sum::<f64>();
            assert!(den>0. && den.is_finite());
            esc[g]=weights[g][3]/den;
            for s in 0..3 {pp[g][s]=weights[g][s]*y[s]/den;photo[s]+=j[g]*pp[g][s];}
        }
        (photo,esc,j,pp)
    }
    fn rhs(&self,u:f64,y:&[f64],out:&mut[f64],frozen:bool){
        let (rates,esc,j,_)=self.shares(u,y,frozen);
        let k:[f64;3]=std::array::from_fn(|s|self.h*rates[s]/self.s0[s]);
        out.fill(0.);out[0]=-k[0];out[1]=-k[1];
        out[2]=(self.s0[1]/self.s0[2])*k[1]-k[2];
        out[3..6].copy_from_slice(&k);
        for g in 0..4 {out[6+g]=self.h*j[g]*esc[g]/self.s0[0];out[10+g]=self.h*j[g]/self.s0[0];}
        out[14]=self.h*(0..3).map(|s|self.energies[s]*rates[s]).sum::<f64>()/self.s0[0];
    }
    fn jvp(&self,u:f64,y:&[f64],v:&[f64],out:&mut[f64],frozen:bool){
        let (_,esc,j,pp)=self.shares(u,y,frozen);
        let mut dr=[0.;3];let mut de=[0.;4];
        for g in 0..4 {
            let common=(0..3).map(|s|pp[g][s]*v[s]/y[s]).sum::<f64>();
            for s in 0..3 {dr[s]+=j[g]*pp[g][s]*(v[s]/y[s]-common);}
            de[g]=-j[g]*esc[g]*common;
        }
        let dk:[f64;3]=std::array::from_fn(|s|self.h*dr[s]/self.s0[s]);
        out.fill(0.);out[0]=-dk[0];out[1]=-dk[1];
        out[2]=(self.s0[1]/self.s0[2])*dk[1]-dk[2];
        out[3..6].copy_from_slice(&dk);
        for g in 0..4 {out[6+g]=self.h*de[g]/self.s0[0];}
        out[14]=self.h*(0..3).map(|s|self.energies[s]*dr[s]).sum::<f64>()/self.s0[0];
    }
    fn invariant(&self,y:&[f64])->[f64;5]{
        let h=y[0]+y[3]-1.;let hei=y[1]+y[4]-1.;
        let heii=y[2]-1.-(self.s0[1]/self.s0[2])*y[4]+y[5];
        let photons=(10..14).map(|i|y[i]).sum::<f64>()-(6..10).map(|i|y[i]).sum::<f64>()
            -(0..3).map(|s|y[s+3]*self.s0[s]/self.s0[0]).sum::<f64>();
        let binding=y[14]-(0..3).map(|s|self.energies[s]*y[s+3]*self.s0[s]/self.s0[0]).sum::<f64>();
        [h,hei,heii,photons,binding]
    }
}
fn run_case(src:Arc<Source>,frozen:bool,samples:&[f64],expected:&serde_json::Value)
        ->Result<(serde_json::Value,f64),Box<dyn std::error::Error>> {
    let s=Arc::clone(&src);let v=Arc::clone(&src);
    let pr=OdeProblem::new("original-REI-five-clock-photochemical-three-moment",DIM,
        Arc::new(move |u,y,out| {s.rhs(u,y,out,frozen);Ok(())}),
        None,None,
        Some(Arc::new(move |u,y,tan,out| {v.jvp(u,y,tan,out,frozen);Ok(())})),
        None,true,None,None)?;
    let linear=LinearSolverConfig{method:LinearMethod::Gmres,..Default::default()};
    let config=AdaptiveStepConfig{rtol:2e-11,atol:2e-14,initial_step:0.01,max_step:0.1,max_attempts:20000,..Default::default()};
    let time=OutputSchedule::new(samples.to_vec())?;
    let mut init=[0.;DIM];init[0..3].fill(1.);
    let res=integrate_sequential_matrix_free_adaptive_dense_observed(
        &pr,(0.,1.),&init,&linear,&config,&OutputSamplingPlan::dense(time.clone()))?;
    let complete=res.observed.success && res.observed.t==time.times();
    let mut maxdiff: f64=0.0;let mut maxinv: f64=0.0;
    for (i,y) in res.observed.y.iter().enumerate() {
        for (j,v) in y.iter().enumerate() {
            maxdiff=maxdiff.max((v-expected[i][j].as_f64().ok_or("expected reference f64")?).abs());
        }
        maxinv=maxinv.max(src.invariant(y).iter().map(|x|x.abs()).fold(0.,f64::max));
    }
    let final_state=res.observed.y.last().ok_or("missing states")?;
    let particles:[f64;3]=std::array::from_fn(|s|src.s0[s]*final_state[s+3]);
    let data=serde_json::json!({
        "case":if frozen {"frozen"} else {"timevarying"},
        "complete":complete,"success":res.observed.success,
        "max_abs_native_minus_Radau":maxdiff,"max_photon_nuclei_binding_invariant":maxinv,
        "final_neutral_ratios":&final_state[0..3],"absorbed_photons_by_species_particles":particles,
        "ionization_binding_eV_per_initial_HI":final_state[14],
        "t":res.observed.t,"y":res.observed.y,
        "diagnostics":res.diagnostics,"counters":res.observed.counters
    });
    if !complete {return Err("incomplete native photo source IVP".into());}
    Ok((data,maxdiff))
}
fn run()->Result<(serde_json::Value,bool),Box<dyn std::error::Error>>{
    let s=Arc::new(Source::new());
    let refdata:serde_json::Value=serde_json::from_str(REFERENCE)?;
    let samples:Vec<f64>=refdata["samples_u"].as_array().ok_or("missing samples")?.iter()
        .map(|x|x.as_f64().ok_or("invalid time")).collect::<Result<_,_>>()?;
    let (forced,err_forced)=run_case(Arc::clone(&s),false,&samples,&refdata["forced_states"])?;
    let (fixed,err_fixed)=run_case(Arc::clone(&s),true,&samples,&refdata["frozen_states"])?;
    let maxerr=err_forced.max(err_fixed);
    let budget=refdata["predeclared_native_state_budget"].as_f64().ok_or("missing budget")?;
    let x=forced["ionization_binding_eV_per_initial_HI"].as_f64().unwrap();
    let y=fixed["ionization_binding_eV_per_initial_HI"].as_f64().unwrap();
    let payload=serde_json::json!({
        "model":"original REI 46k-cell 3 global neutral moment 4-group five-clock time-interpolated photon source, no thermal/OTS",
        "method":"existing VigilODE matrix-free RODAS5P with analytic JVP, original PR173 owner",
        "source_pr332_artifact_sha256":"83d94b5383ac7c1f36d89486e83bf8b4a56dd1993a25af3ae1f8fed06a695ba9",
        "source_pr342_artifact_sha256":"88f7e5bb1c6c44c0ca0c7f2ef2c81ac70098cc7ed773096868b1e7b4db25c035",
        "max_native_reference_gap":maxerr,"predeclared_native_budget":budget,
        "source_scoped_pass":maxerr<=budget,
        "ionization_binding_eV_per_original_HI_timeforce_minus_frozen":x-y,
        "cases":[forced,fixed],
        "original_REI_F05_first_interval":"BLOCKED_MINIMUM_STEP",
        "original_owner_IVPs":0,"canonical_REI_delta":0,"vigilode_admission_delta":0,"production":"HOLD"
    });
    Ok((payload,maxerr<=budget))
}
fn main()->ExitCode {
    match run(){
        Ok((p,pass))=>{
            let mut out=std::io::stdout().lock();
            if serde_json::to_writer_pretty(&mut out,&p).is_err(){return ExitCode::FAILURE;}
            let _=out.write_all(b"\n");
            if pass {ExitCode::SUCCESS} else {ExitCode::FAILURE}
        },
        Err(e)=>{eprintln!("REI five-clock RODAS5P source error: {e}");ExitCode::FAILURE}
    }
}
#[cfg(test)] mod tests {
    use super::*;
    #[test]fn source_knot_provenance(){let a=Source::new();for k in 0..5 {let (j,w)=a.forcing(a.times[k],false);for g in 0..4 {assert!((j[g]-a.j[k][g]).abs()<1e-13*a.j[k][g].abs().max(1.));for s in 0..4 {assert!((w[g][s]-a.w[k][g][s]).abs()<3e-14);}}}}
    #[test]fn common_photon_and_nuclear_conservation(){let a=Source::new();let mut y=[0.;DIM];y[0..3].fill(1.);let mut d=[0.;DIM];a.rhs(.42,&y,&mut d,false);let mut v=y;for i in 0..DIM {v[i]+=0.01*d[i];}let x=a.invariant(&v);assert!(x.iter().all(|z|z.abs()<2e-14),"{:?}",x);}
    #[test]fn analytic_15_state_jvp(){let a=Source::new();let mut y=[0.;DIM];y[0..3].copy_from_slice(&[.9998,.9995,1.000001]);let v:[f64;DIM]=std::array::from_fn(|i|(i as f64)/20.-.3);let eps=1e-5;let yp:[f64;DIM]=std::array::from_fn(|i|y[i]+eps*v[i]);let ym:[f64;DIM]=std::array::from_fn(|i|y[i]-eps*v[i]);let mut dp=[0.;DIM];let mut dm=[0.;DIM];let mut jv=[0.;DIM];a.rhs(.37,&yp,&mut dp,false);a.rhs(.37,&ym,&mut dm,false);a.jvp(.37,&y,&v,&mut jv,false);for i in 0..DIM {assert!(((dp[i]-dm[i])/(2.*eps)-jv[i]).abs()<2e-9,"{i}");}}
    #[test]fn native_real_source_two_arm_science(){let (_,pass)=run().unwrap();assert!(pass);}
}
