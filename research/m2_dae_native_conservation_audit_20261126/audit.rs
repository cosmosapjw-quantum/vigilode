use serde_json::{json,Value};
use vigilode::dae_evolution::*;
use vigilode::polynomial_events::{AffineFactor as F,DeclaredPolynomial as P,EventTerm as T,retro_dae_init::*};
use vigilode::{Atol,InteriorError,Options};
use rodas5p_core::directed::Interval as I;
fn p(terms:Vec<T>)->P{P::new(terms).unwrap()}
fn term(coefficient:f64,factors:Vec<F>)->T{T{coefficient,factors}}
fn opt(rt:f64)->Options{Options{rtol:rt,atol:Atol::Scalar(rt*0.001),dense_output:true,interior_error:InteriorError::Off,max_steps:4096,..Default::default()}}
fn policy()->EvolutionPolicy{EvolutionPolicy{init:InitPolicy{mode:InitMode::ProjectAlgebraic,state_tol:1e-11,residual_tol:1e-11,derivative_tol:1e-6,..Default::default()},tube_radius:1e-3,global_tol:1e-3,green:true}}
fn mp(method:MultiGuardRoute)->MultiGuardPolicy{MultiGuardPolicy{method,tolerance:1e-8,root_route:ThresholdRoute::DictionaryHint,derivative:GuardDerivativeRoute::Schur,value:GuardValueRoute::Transport,pending:PendingPolicy{method:PendingMethod::PiecewiseProbe,max_slabs:8},max_slabs:128}}
fn rec(r:&EvolutionRun)->Value{json!({"pointwise":r.pointwise_qualified,"branch":r.branch_qualified,"global":r.global_qualified,"bound":r.max_physical_error,"reason":r.reason,"driver_success":r.driver().is_success(),"driver_end":r.driver().t.last(),"attempts":r.driver().stats.attempts,"counters":r.driver().stats.counters,"work":r.work,"initial":r.initial,"slabs":r.slabs(),"queries":r.queries,"accepted":r.accepted,"warp":r.warp})}
fn grid(a:f64,b:f64)->Vec<f64>{(0..=32).map(|i|if i==32{b}else{a+(b-a)*i as f64/32.}).collect()}
const ROUTES:[EvolutionRoute;6]=[EvolutionRoute::Mass,EvolutionRoute::Reduced3,EvolutionRoute::Reduced5,EvolutionRoute::Prepared5,EvolutionRoute::Scalar5,EvolutionRoute::WarpFolded];
const CERTS:[ConservationRoute;3]=[ConservationRoute::Max,ConservationRoute::Integral4,ConservationRoute::Integral16];
fn main(){
 for route in ROUTES{for cert in CERTS{for rt in [1e-7,1e-9]{for radius in [0.,2f64.powi(-20)]{
  let mut row=json!({"solver":route,"cert":cert,"rtol":rt,"radius":radius});
  let initial=RobertsonInitial::simplex_ball([1.,0.],radius).unwrap();
  match evolve_robertson(3e7,initial,[0.,1.],&opt(rt),policy(),route,cert,&grid(0.,1.)){
   Ok(r)=>row["run"]=rec(&r),Err(e)=>row["error"]=json!(e.to_string())
  }
  println!("CELL {}",row);
 }}}}
 let tau=1./256.;let end=1./32.;
 for route in ROUTES[..5].iter().copied(){for cert in CERTS{for method in [MultiGuardRoute::Shared,MultiGuardRoute::Correlated]{
  let mut row=json!({"solver":route,"cert":cert,"order":method,"rtol":1e-9,"radius":2f64.powi(-20)});
  let initial=RobertsonInitial::simplex_ball([1.,0.],2f64.powi(-20)).unwrap();
  match evolve_robertson_clock_reset([3e7,6e7],initial,[0.,end],[2.*tau,tau],&opt(1e-9),policy(),route,cert,mp(method),&grid(tau,end)){
   Ok(r)=>{
    row["event"]=json!(r.event_time);row["winner"]=json!(r.pre().winner());row["reason"]=json!(r.reason);
    row["carry"]=json!(r.incoming_l1);row["center"]=json!(r.reset_center);row["attempts"]=json!(r.native_attempts);
    row["pre_guards"]=json!(r.pre().guards());
    row["pre_work"]=json!(r.pre().work);
    let qs=(0..=16).filter_map(|i|r.pre().pre_query(tau*i as f64/16.).ok()).collect::<Vec<_>>();
    row["pre_queries"]=json!(qs);
    if let Some(po)=r.post_diagnostic(){row["post"]=rec(po);}
    row["committed"]=json!(r.post().is_some());
   },Err(e)=>row["error"]=json!(e.to_string())
  }
  println!("CHAIN {}",row);
 }}}
 assert!(RobertsonInitial::exact([1.,1e-15]).is_err());
 assert!(RobertsonInitial::simplex_ball([-1e-15,0.],1.).is_err());
 assert!(RobertsonInitial::simplex_ball([1.,0.],-1.).is_err());
 assert!(RobertsonModel::new(1e7).is_err());
 let init=RobertsonInitial::exact([1.,0.]).unwrap();
 let tie=evolve_robertson_clock_reset([3e7,6e7],init,[0.,end],[tau,tau],&opt(1e-7),policy(),EvolutionRoute::Mass,ConservationRoute::Integral4,mp(MultiGuardRoute::Correlated),&[]).unwrap();
 assert!(tie.post().is_none()&&tie.pre().winner().is_none());
 let mut o=opt(1e-7);o.max_steps=1;
 let limited=evolve_robertson_clock_reset([3e7,6e7],init,[0.,end],[tau,2.*tau],&o,policy(),EvolutionRoute::Mass,ConservationRoute::Integral4,mp(MultiGuardRoute::Shared),&[]).unwrap();
 assert!(limited.post().is_none()&&limited.native_attempts<=1);
 let bad=p(vec![term(-1.,vec![F::clock(tau),F::clock(tau)])]);
 let h=p(vec![term(1.,vec![F::clock(tau)])]);
 let r=evolve_robertson_to_first_guard(3e7,init,[0.,end],&opt(1e-7),policy(),EvolutionRoute::Mass,ConservationRoute::Integral4,&[h,bad],mp(MultiGuardRoute::Correlated)).unwrap();
 assert!(r.event_authority().is_none());
 println!("NEGATIVE {}",json!({"tie":tie.reason,"budget":limited.reason,"nontransverse":r.reason()}));
 let model=RobertsonModel::new(3e7).unwrap();
 let point=|x:f64|I::point(x).unwrap();
 // Deliberately nonphysical dense comparator, not a physical initial set.
 // E_in encloses its offset from physical initial [1,0].
 for (n,v) in [(0,2f64.powi(-28)),(1,2f64.powi(-12))]{
  let y=vec![vec![point(1.)],vec![point(-v)],vec![point(v)]];
  for method in CERTS{
   let c=model.certify([0.,1e-5],&y,&[[-0.25,1.25]],1e-3,Some(v),1e12,method).unwrap();
   assert!(c.conservation.as_ref().unwrap().cells.iter().all(|c|c.v_minus>=v&&c.mu_l1_upper>=2.*3e7*v));
   println!("OVERSHOOT {}",json!({"case":n,"v":v,"method":method,"certificate":c}));
  }
 }
}
