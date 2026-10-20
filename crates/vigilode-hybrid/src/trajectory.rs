use super::{Iv,Curve,DerivativeMode,Model,Result,Segment,Work};
#[derive(Clone,Debug)]
#[cfg_attr(feature="native",derive(serde::Serialize))]
pub struct IntegrationConfig {pub t_start:f64,pub t_end:f64,pub initial_h:f64,pub min_h:f64,pub max_h:f64,pub max_attempts:usize,pub abs_tolerance:f64,pub initial_radius:f64,pub quintic:bool,pub derivative_mode:DerivativeMode}
impl Default for IntegrationConfig {fn default()->Self{Self{t_start:0.0,t_end:0.2,initial_h:0.025,min_h:1e-12,max_h:0.1,max_attempts:4096,abs_tolerance:1e-6,initial_radius:0.0,quintic:true,derivative_mode:DerivativeMode::LocalAd}}}
#[derive(Clone,Debug)]
pub struct Endpoint {pub state:Vec<f64>,pub method:String}
#[derive(Clone,Debug)]
#[cfg_attr(feature="native",derive(serde::Serialize))]
pub struct Trial {pub t0:f64,pub t1:f64,pub accepted:bool,pub method:String,pub reason:String,pub defect:Option<f64>,pub radius_before:f64,pub radius_after:f64,pub accepted_state_fingerprint:u64,pub cumulative_work:Work}
#[derive(Clone,Debug)]
#[cfg_attr(feature="native",derive(serde::Serialize))]
pub struct Integration {
 pub success:bool,pub failure:Option<String>,pub times:Vec<f64>,pub states:Vec<Vec<f64>>,pub radii:Vec<f64>,pub final_t:f64,pub final_state:Vec<f64>,pub final_radius:f64,
 pub accepted:usize,pub rejected:usize,pub work:Work,pub trials:Vec<Trial>,pub segments:Vec<Segment>,pub absolute_budget:f64,
}
impl Integration {
 pub fn evaluate(&self,t:f64)->Result<(Vec<f64>,f64)>{
  let segment=self.segments.iter().find(|s|t>=s.curve().start()&&t<=s.curve().end()).ok_or("QUERY_OUTSIDE_COMPLETED_TRAJECTORY")?;
  let(v,r)=segment.evaluate(t,&mut Work::default())?;if r>self.absolute_budget{return Err("QUERY_ROUNDING_EXCEEDS_BUDGET".into())}Ok((v,r))
 }
}
fn fingerprint(y:&[f64])->u64{let mut h=0xcbf29ce484222325u64;for v in y{for b in v.to_bits().to_le_bytes(){h=(h^b as u64).wrapping_mul(0x100000001b3);}}h}
/// Causal continuous-output gate for endpoint generators. Full-model proof and
/// requested returned-point bounds must pass before committing any state/time.
/// Spatial discretization/model error is outside this semidiscrete ODE bound.
pub fn integrate<F>(model:&Model,y0:&[f64],times:&[f64],cfg:&IntegrationConfig,mut candidate:F)->Result<Integration>
where F:FnMut(f64,&[f64],f64,&mut Work)->Result<Endpoint>{
 model.initial_box(y0)?;
 if ![cfg.t_start,cfg.t_end,cfg.initial_h,cfg.min_h,cfg.max_h,cfg.abs_tolerance,cfg.initial_radius].iter().all(|x|x.is_finite())||cfg.t_end<=cfg.t_start||cfg.min_h<=0.0||cfg.initial_h<cfg.min_h||cfg.max_h<cfg.initial_h||cfg.abs_tolerance<=0.0||cfg.initial_radius<0.0||cfg.initial_radius>cfg.abs_tolerance*0.5||cfg.max_attempts==0||cfg.max_attempts>100000{return Err("INVALID_INTEGRATION_CONFIG".into())}
 for v in y0{if *v<cfg.initial_radius||Iv::point(*v)?.add(Iv::point(cfg.initial_radius)?)?.hi>1.0{return Err("INITIAL_UNCERTAINTY_LEAVES_INVARIANT_BOX".into())}}
 if times.len()<2||times[0]!=cfg.t_start||*times.last().unwrap()!=cfg.t_end||times.iter().any(|v|!v.is_finite())||times.windows(2).any(|x|x[1]<=x[0]){return Err("INVALID_OUTPUT_GRID".into())}
 let mut out=Integration{success:false,failure:None,times:vec![cfg.t_start],states:vec![y0.to_vec()],radii:vec![cfg.initial_radius],final_t:cfg.t_start,final_state:y0.to_vec(),final_radius:cfg.initial_radius,accepted:0,rejected:0,work:Work::default(),trials:Vec::new(),segments:Vec::new(),absolute_budget:cfg.abs_tolerance};
 let mut h=cfg.initial_h;let mut cursor=1usize;let span=cfg.t_end-cfg.t_start;let params=model.params();let target_defect=0.25*cfg.abs_tolerance*(params[1]-params[2]).max(1.0/span);
 for _ in 0..cfg.max_attempts{
  if out.final_t==cfg.t_end{out.success=true;break;}let t0=out.final_t;let t1=(t0+h).min(cfg.t_end);
  if t1<=t0||!(t1-t0).is_normal(){out.failure=Some("UNREPRESENTABLE_NEXT_TIME".into());break;}
  let actual_h=t1-t0;let r0=out.final_radius;out.work.candidate_calls+=1;let end=candidate(t0,&out.final_state,actual_h,&mut out.work);
  let mut reason=String::new();let mut method="candidate-refused".to_owned();let mut measured_defect=None;let mut proposed=None;
  match end {
   Err(e)=>{out.work.candidate_failures+=1;reason=e;}
   Ok(end)=>{
    method=end.method;let result=Curve::hermite(model,t0,t1,&out.final_state,&end.state,cfg.quintic,cfg.derivative_mode,&mut out.work).and_then(|curve|Segment::verify(model,curve,r0,&mut out.work));
    match result {
     Err(e)=>reason=e,
     Ok(segment)=>{
      measured_defect=Some(segment.defect());
      if segment.window_radius()>cfg.abs_tolerance*0.5{reason="CONTINUOUS_ERROR_BUDGET".into();}
      else{let mut observations=Vec::new();let mut next=cursor;
       while next<times.len()&&times[next]<=t1{match segment.evaluate(times[next],&mut out.work){Ok((v,r))if r<=cfg.abs_tolerance=>observations.push((times[next],v,r)),Ok(_)=>{reason="RETURNED_POINT_ERROR_BUDGET".into();break;}Err(e)=>{reason=e;break;}}next+=1;}
       if reason.is_empty(){proposed=Some((segment,observations,next));}
      }
     }
    }
   }
  }
  let accepted=proposed.is_some();
  if let Some((segment,observations,next))=proposed{out.final_t=t1;out.final_state=segment.curve().endpoint();out.final_radius=segment.endpoint_radius();for(t,v,r)in observations{out.times.push(t);out.states.push(v);out.radii.push(r);}cursor=next;out.segments.push(segment);out.accepted+=1;}else{out.rejected+=1;}
  out.trials.push(Trial{t0,t1,accepted,method,reason,defect:measured_defect,radius_before:r0,radius_after:out.final_radius,accepted_state_fingerprint:fingerprint(&out.final_state),cumulative_work:out.work.clone()});
  if accepted{let ratio=if let Some(d)=measured_defect{if d>0.0{0.9*(target_defect/d).powf(0.2)}else{2.0}}else{1.0};h=(actual_h*ratio.clamp(0.5,2.0)).clamp(cfg.min_h,cfg.max_h);}
  else{let smaller=actual_h*0.5;if smaller<cfg.min_h||smaller>=actual_h{out.failure=Some("MINIMUM_STEP_AFTER_DECLINE".into());break;}h=smaller;}
 }
 if out.final_t==cfg.t_end&&cursor==times.len(){out.success=true;}if !out.success&&out.failure.is_none(){out.failure=Some("ATTEMPT_LIMIT".into());}Ok(out)
}
