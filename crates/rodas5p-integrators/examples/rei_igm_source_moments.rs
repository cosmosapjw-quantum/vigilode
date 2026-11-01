//! VigilODE REI IGM client: original four photon-group / three neutral-moment source.
//! Source owner REI PR332 (immutable), 46080 original cells; frozen at accepted tick160.
//! This integrates a genuine *reduced photoionization-only* physics IVP, NOT the full
//! 26-event REI MPRK/SDIRK chemical/thermal/photon system or first canonical interval.
use std::{io::Write, process::ExitCode, sync::Arc};
use rodas5p_core::{LinearMethod, LinearSolverConfig};
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSamplingPlan, OutputSchedule,
    integrate_sequential_matrix_free_adaptive_dense_observed,
};

const SOURCE: &str = include_str!("../../../exploratory/rei_igm_original_moment_client_20261101/frozen_pr332_source.json");
const REFERENCE: &str = include_str!("../../../exploratory/rei_igm_original_moment_client_20261101/reference_trajectory.json");
const BUDGET: f64 = 2.0e-8;

#[derive(Clone, Debug)]
struct FrozenPhoto {
    s0: [f64;3], j: [f64;4], p: [[f64;3];4], e: [f64;4], h: f64,
}
impl FrozenPhoto {
    fn original() -> Self {
        let f:serde_json::Value=serde_json::from_str(SOURCE).expect("original frozen PR332 source fixture");
        assert_eq!(f["nodes"].as_u64(),Some(46080));
        assert_eq!(f["source_artifact_sha256"].as_str(),Some("83d94b5383ac7c1f36d89486e83bf8b4a56dd1993a25af3ae1f8fed06a695ba9"));
        let s0=std::array::from_fn(|i| f["global_neutral_particle_counts_S0"][i].as_f64().unwrap());
        let j=std::array::from_fn(|g| f["groups"][g]["original_photon_current_per_s"].as_f64().unwrap());
        let p=std::array::from_fn(|g| std::array::from_fn(|i| f["groups"][g]["baseline_species_absorption_fraction"][i].as_f64().unwrap()));
        let e=std::array::from_fn(|g| f["groups"][g]["baseline_external_escape_fraction"].as_f64().unwrap());
        let h=f["dt_s"].as_f64().unwrap();
        for g in 0..4 {assert!((p[g].iter().sum::<f64>()+e[g]-1.0).abs()<5e-14);}
        Self {s0,j,p,e,h}
    }
    fn rates(&self,y:&[f64])->[f64;3]{
        let mut r=[0.0;3];
        for g in 0..4 {
            let den=self.e[g]+(0..3).map(|s|self.p[g][s]*y[s]).sum::<f64>();
            assert!(den>0.0 && den.is_finite());
            for s in 0..3 {r[s]+=self.j[g]*self.p[g][s]*y[s]/den;}
        }
        r
    }
    fn rhs(&self,y:&[f64],out:&mut[f64]) {
        assert_eq!(y.len(),6);assert_eq!(out.len(),6);
        let r=self.rates(y);
        let k: [f64;3]=std::array::from_fn(|s|self.h*r[s]/self.s0[s]);
        out[0]=-k[0];out[1]=-k[1];out[2]=(self.s0[1]/self.s0[2])*k[1]-k[2];
        out[3..6].copy_from_slice(&k);
    }
    fn jvp(&self,y:&[f64],v:&[f64],out:&mut[f64]) {
        assert_eq!(y.len(),6);assert_eq!(v.len(),6);assert_eq!(out.len(),6);
        let mut dr=[0.0;3];
        for g in 0..4 {
            let den=self.e[g]+(0..3).map(|s|self.p[g][s]*y[s]).sum::<f64>();
            assert!(den>0.0);
            let p: [f64;3]=std::array::from_fn(|s|self.p[g][s]*y[s]/den);
            let share=(0..3).map(|s|p[s]*v[s]/y[s]).sum::<f64>();
            for s in 0..3 {dr[s]+=self.j[g]*p[s]*(v[s]/y[s]-share);}
        }
        let k: [f64;3]=std::array::from_fn(|s|self.h*dr[s]/self.s0[s]);
        out[0]=-k[0];out[1]=-k[1];out[2]=(self.s0[1]/self.s0[2])*k[1]-k[2];
        out[3..6].copy_from_slice(&k);
    }
}
fn schedule()->OutputSchedule {
    OutputSchedule::new((0..=8).map(|i| f64::from(i)/8.0).collect()).expect("time grid")
}
fn run() -> Result<(serde_json::Value,bool),Box<dyn std::error::Error>> {
    let source=Arc::new(FrozenPhoto::original());
    let src1=Arc::clone(&source);let src2=Arc::clone(&source);
    let problem=OdeProblem::new("original-rei-46080-source-three-global-moments",6,
        Arc::new(move |_t,y,out| {src1.rhs(y,out); Ok(())}),
        None,None,
        Some(Arc::new(move |_t,y,v,out| {src2.jvp(y,v,out); Ok(())})),
        None,true,None,None)?;
    let linear=LinearSolverConfig {method:LinearMethod::Gmres,..Default::default()};
    let adaptive=AdaptiveStepConfig{rtol:2e-10,atol:2e-13,initial_step:0.02,max_step:0.25,max_attempts:10000,..Default::default()};
    let output=schedule();
    let result=integrate_sequential_matrix_free_adaptive_dense_observed(
        &problem,(0.,1.),&[1.,1.,1.,0.,0.,0.],&linear,&adaptive,
        &OutputSamplingPlan::dense(output.clone()))?;
    let refdata:serde_json::Value=serde_json::from_str(REFERENCE)?;
    let target=refdata["states"].as_array().ok_or("missing original reference states")?;
    let mut max_error=0.0f64;
    for (out,expected) in result.observed.y.iter().zip(target) {
        for (i,x) in out.iter().enumerate() {
            max_error=max_error.max((x-expected["y"][i].as_f64().unwrap()).abs());
        }
    }
    let complete=result.observed.success && result.observed.t==output.times();
    let pass=complete && max_error<=BUDGET;
    let final_state=result.observed.y.last().ok_or("no native physics states")?;
    let photons:[f64;3]=std::array::from_fn(|s|source.s0[s]*final_state[3+s]);
    let electron_delta: f64=photons.iter().sum();
    let identity_h=(final_state[0]+final_state[3]-1.).abs();
    let identity_hei=(final_state[1]+final_state[4]-1.).abs();
    let identity_heii=(final_state[2]-1.-source.s0[1]/source.s0[2]*final_state[4]+final_state[5]).abs();
    let payload=serde_json::json!({
        "model":"VigilODE original RODAS5P matrix-free integration of source-frozen actual REI 3-neutral-moment 4-photon-group photoionization-only IVP",
        "original_REI_source":"PR332: 46080 source cells, accepted tick160, frozen source + forcing",
        "method":"existing sequential matrix-free RODAS5P, analytic source JVP, no assembled Jacobian",
        "not_original_26_event_thermal_igm":true,
        "source_artifact_sha256":"83d94b5383ac7c1f36d89486e83bf8b4a56dd1993a25af3ae1f8fed06a695ba9",
        "original_one_tick_seconds":source.h,
        "success":result.observed.success,"complete":complete,"source_scoped_comparison_pass":pass,
        "max_abs_discrepancy_vs_independent_Radau_reference":max_error,
        "predeclared_comparison_budget":BUDGET,
        "final_dimensionless_state":final_state,
        "absorbed_photon_counts_by_species":photons,
        "source_photoelectron_count":electron_delta,
        "conservation_identity_residual":[identity_h,identity_hei,identity_heii],
        "t":result.observed.t,"y":result.observed.y,
        "diagnostics":result.diagnostics,
        "counters":result.observed.counters,
        "canonical_rei_delta":0,"vigilode_official_delta":0,"production":"HOLD"
    });
    Ok((payload,pass))
}
fn main()->ExitCode {
    match run(){
        Ok((payload,pass))=>{
            let mut out=std::io::stdout().lock();
            if serde_json::to_writer_pretty(&mut out,&payload).is_err(){return ExitCode::FAILURE;}
            let _=out.write_all(b"\n");
            if pass {ExitCode::SUCCESS} else {ExitCode::FAILURE}
        },
        Err(e)=>{eprintln!("native-REI-scope-error: {e}");ExitCode::FAILURE}
    }
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn positive_photon_source_and_conserved_nuclei(){
        let x=FrozenPhoto::original();let mut out=[0.;6];x.rhs(&[1.,1.,1.,0.,0.,0.],&mut out);
        assert!(out[0]<0. && out[1]<0. && out[3]>0. && out[4]>0.);
        assert!((out[0]+out[3]).abs()<1e-15);
        assert!((out[1]+out[4]).abs()<1e-15);
    }
    #[test]fn source_jvp_matches_symmetric_difference(){
        let x=FrozenPhoto::original();let y=[0.9999,1.0001,0.99999,0.,0.,0.];
        let v=[0.2,-0.7,0.1,0.,0.,0.];let mut direct=[0.;6];x.jvp(&y,&v,&mut direct);
        let eps=1e-5;let yp=std::array::from_fn(|i| y[i]+eps*v[i]);
        let ym=std::array::from_fn(|i| y[i]-eps*v[i]);
        let mut rp=[0.;6];let mut rm=[0.;6];x.rhs(&yp,&mut rp);x.rhs(&ym,&mut rm);
        for i in 0..6 {assert!(((rp[i]-rm[i])/(2.*eps)-direct[i]).abs()<2e-10,"{i}");}
    }
    #[test]fn actual_native_integration_matches_independent_scipy_reference(){
        let (payload,pass)=run().unwrap();assert!(pass,"{}",payload["max_abs_discrepancy_vs_independent_Radau_reference"]);
    }
}
