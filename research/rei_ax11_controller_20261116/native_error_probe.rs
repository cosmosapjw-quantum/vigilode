//! Fixed source comparisons only: no new history, controller, tolerances or geometry.
use rei_microphysics::axbi_driver::{compiled_identity,Driver,DriverConfig,DriverState};
use std::{env,fs,io::Write,path::Path,time::Instant};
type E=Box<dyn std::error::Error>;
fn physical_bytes(s:&DriverState)->Vec<u8>{
 let mut b=Vec::new();
 let l=&s.ledgers;
 for x in [s.a,s.gas.fractions[0],s.gas.fractions[1],s.gas.fractions[2],s.gas.w_erg_per_h,l.source_n,l.source_u,l.absorbed_n,l.escaped_u,l.boundary_n,l.boundary_u,l.thermal_work,l.photon_work,l.cmb_input,l.dr_events,l.ce_cap_ev,l.excluded_dr_ev]{b.extend_from_slice(&x.to_bits().to_le_bytes());}
 for v in [&l.rr_events,&l.ci_events,&l.photo_events,&l.ci_floor_events]{for x in v{b.extend_from_slice(&x.to_bits().to_le_bytes());}}
 for x in &s.photons{b.extend_from_slice(&x.to_bits().to_le_bytes());}
 b
}
fn total_bytes(p:&Path)->Result<u64,E>{let mut total=0;for f in fs::read_dir(p)?{let f=f?;let m=f.metadata()?;total+=if m.is_dir(){total_bytes(&f.path())?}else{m.len()};}Ok(total)}
fn execute(inputs:&Path,out:&Path)->Result<(),E>{
 let process=Instant::now();let mut total_trials=0usize;let mut total_rhs=0usize;let mut probes=0usize;
 fs::write(out.join("instrumented_compiled_identity.txt"),compiled_identity())?;
 let mut csv=fs::File::create(out.join("ERROR_COMPONENTS.csv"))?;
 writeln!(csv,"id,case,window,parent_accepted,parent_attempts,parent_rhs,parent_wall_s,a,dln_a,local_target,HII,HeII,HeIII,thermal_w,photon_number_L1,photon_energy_L1,original_metric,winner,accepted,native_next_dln_a,full_rhs,half1_rhs,half2_rhs,adaptive_rhs,full_ns,half1_ns,half2_ns,adaptive_ns,record_write_ns")?;
 for case in ["ZERO","SHEAR_PLUS"] {
  let cfg=fs::read_to_string(inputs.join(case).join("config.cfg"))?;
  let (d,ic)=Driver::new(DriverConfig::read(&cfg)?)?;
  let late=d.read_checkpoint(&inputs.join(case).join("checkpoint.txt"))?;
  if late.accepted!=199980||late.a.to_bits()!=0.08873100115616298_f64.to_bits(){return Err("SOURCE_PLATEAU_BINDING_CHANGED".into());}
  fs::write(out.join(format!("{case}_energies.txt")),d.energies.iter().map(|x|format!("{x:.17e}\n")).collect::<String>())?;
  for (window,parent,steps) in [
   ("initial",ic.clone(),[7.62939453125e-10,3.814697265625e-10,1.9073486328125e-10]),
   ("plateau",late.clone(),[3.125e-6,1.5625e-6,7.8125e-7])
  ] {
   for (level,h) in steps.into_iter().enumerate(){
    if process.elapsed().as_secs_f64()>28.0||total_trials+6>72||total_rhs>=12000||total_bytes(out)?>7*1024*1024{return Err("BOUNDED_PROBE_CAP".into());}
    let id=format!("{case}_{window}_{level}");
    let orig_text=d.ax11_checkpoint_text(&parent)?;
    let probe=match d.ax11_triplet_probe(&parent,h){Ok(p)=>p,Err(e)=>{fs::write(out.join("FIRST_FAILURE.txt"),format!("{id}: {e:?}"))?;return Err(e.into());}};
    total_trials+=3;total_rhs+=probe.rhs_per_trial.iter().sum::<usize>();
    if d.ax11_checkpoint_text(&parent)?!=orig_text{return Err("SOURCE_INPUT_MUTATED".into());}
    let mut actual=parent.clone();actual.next_dln_a=h;
    let t=Instant::now();let accepted=d.adaptive_step(&mut actual)?;let adaptive_ns=t.elapsed().as_nanos();
    total_trials+=actual.attempts-parent.attempts;total_rhs+=actual.rhs_evaluations-parent.rhs_evaluations;
    let expected=probe.metric<d.config.local_target && probe.metric<2e-4;
    let original_next=if expected{if probe.metric<d.config.local_target*0.25{(h*2.0).min(0.005)}else{h}}else{h*0.5};
    if accepted!=expected||actual.last_error.to_bits()!=probe.metric.to_bits()||actual.next_dln_a.to_bits()!=original_next.to_bits(){return Err("ORIGINAL_CONTROLLER_DECISION_MISMATCH".into());}
    if actual.attempts-parent.attempts!=3||actual.comparisons-parent.comparisons!=1||actual.rhs_evaluations-parent.rhs_evaluations!=probe.rhs_per_trial.iter().sum::<usize>(){return Err("ORIGINAL_CONTROLLER_WORK_MISMATCH".into());}
    let expected_state=if accepted{&probe.twohalf}else{&parent};
    if physical_bytes(&actual)!=physical_bytes(expected_state){return Err("ORIGINAL_CONTROLLER_PHYSICAL_STATE_MISMATCH".into());}
    let names=["HII","HeII","HeIII","thermal_w","photon_number_L1","photon_energy_L1"];
    let mut winner=0;for i in 1..6{if probe.components[i]>probe.components[winner]{winner=i;}}
    let t=Instant::now();
    let dest=out.join(&id);fs::create_dir(&dest)?;
    for (n,state) in [("parent",&parent),("full",&probe.full),("twohalf",&probe.twohalf),("actual",&actual)]{
     fs::write(dest.join(format!("{n}.txt")),d.ax11_checkpoint_text(state)?)?;
    }
    let record_ns=t.elapsed().as_nanos();
    let e=probe.components;let r=probe.rhs_per_trial;let t=probe.trial_ns;
    writeln!(csv,"{id},{case},{window},{},{},{},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{},{},{:.17e},{},{},{},{},{},{},{},{},{}",parent.accepted,parent.attempts,parent.rhs_evaluations,parent.wall_elapsed_s,parent.a,h,d.config.local_target,e[0],e[1],e[2],e[3],e[4],e[5],probe.metric,names[winner],accepted,actual.next_dln_a,r[0],r[1],r[2],actual.rhs_evaluations-parent.rhs_evaluations,t[0],t[1],t[2],adaptive_ns,record_ns)?;
    probes+=1;
    println!("{id} winner={} metric={:.12e} accepted={} rhs={}+{}+{}",names[winner],probe.metric,accepted,r[0],r[1],r[2]);
   }
  }
 }
 csv.flush()?;
 if probes!=12||total_trials!=72||total_rhs>12000{return Err("PROBE_COUNT_OR_RESOURCE_VIOLATION".into());}
 fs::write(out.join("RETURN.txt"),format!("status=COMPLETE_FIXED_SOURCE_DIAGNOSTIC\nprobes={probes}\ncomparisons={}\ntentative_trials={total_trials}\nrhs={total_rhs}\nwall_s={}\nsource_history_mutated=false\nfull_history_starts=0\nAX11_complete=false\n",probes*2,process.elapsed().as_secs_f64()))?;
 Ok(())
}
fn main()->Result<(),E>{
 let a=env::args().collect::<Vec<_>>();if a.len()!=3{return Err("ARGS INPUT_DIRECTORY NEW_OUTPUT_DIRECTORY".into());}
 let inputs=Path::new(&a[1]);let out=Path::new(&a[2]);if out.exists(){return Err("OUTPUT_EXISTS_NO_RETRY".into());}fs::create_dir_all(out)?;
 let result=execute(inputs,out);
 if let Err(e)=&result{fs::write(out.join("FIRST_FAILURE.txt"),format!("{e:?}\n"))?;}
 result
}
