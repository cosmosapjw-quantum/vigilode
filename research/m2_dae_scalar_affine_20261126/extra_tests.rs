
const ROOT_ROUTES:[RootRoute;5]=[RootRoute::Sturm,RootRoute::NativeBernstein,RootRoute::DictionaryBernstein,RootRoute::Companion,RootRoute::DictionaryCompanion];
#[test]
fn affine_singleton_and_branch_boundary(){
 use rodas5p_integrators::retro_event_poly::RootKind;
 let tol=2f64.powi(-48);
 for (a,b,expected) in [(0.,1.,None),(-1.,1.,Some(1.)),(-1.,2.,Some(0.5)),(1.,-2.,Some(0.5)),(-2.,1.,None),(1.,1.,None)]{
  let p=RepresentedPolynomial::from_power(&[a,b]).unwrap();
  let r=p.isolate_affine_exact(tol).unwrap();
  assert_eq!(r.is_some(),expected.is_some());
  if let (Some(v),Some(x))=(r,expected){assert_eq!(v.theta,[x,x]);assert_eq!(v.multiplicity,1);assert_eq!(v.kind,if b>0.{RootKind::Up}else{RootKind::Down});}
 }
 let p=RepresentedPolynomial::from_power(&[-1.,3.]).unwrap();
 let v=p.isolate_affine_exact(tol).unwrap().unwrap();assert_eq!(v.exact_theta,["1/3","1/3"]);assert!(v.theta[0]<v.theta[1]);
 assert!(RepresentedPolynomial::from_power(&[1.,0.,1.]).unwrap().isolate_affine_exact(tol).is_err());
 assert!(RepresentedPolynomial::from_power(&[0.]).unwrap().isolate_affine_exact(tol).is_err());
 assert!(p.isolate_affine_exact(0.).is_err());
 let model=RobertsonModel::new(3e7).unwrap();
 let src=model.source();
 for branch in [[0.,1.],[-1.,0.]]{
  // Exact z=0 at a branch endpoint must refuse, even though the
  // normalized half-open root API admits the right endpoint.
  assert!(src.initialize(0.,&[1.,0.],&[0.],&[branch],policy().init,InitRoute::ScalarRoot).is_err());
 }
}
#[test]
#[ignore="actual native frozen scalar root and projection portfolio"]
fn scalar_portfolio(){
 use serde_json::json;
 let g=p(vec![tr(1.,vec![F::component(0,0.)]),tr(1.,vec![F::component(1,0.)]),tr(1.,vec![F::component(2,0.)]),tr(-1.,vec![])]);
 let tol=2f64.powi(-48);
 for solver in [EvolutionRoute::Mass,EvolutionRoute::Prepared5,EvolutionRoute::Scalar5]{
  for cert in [ConservationRoute::Max,ConservationRoute::Integral4]{
   let m=graph("alternating",solver,cert);
   let r=evolve_robertson_phase_atlas(&m,0,initial(2f64.powi(-26)),[0.,0.002],&opts(),policy(),pm(MultiGuardRoute::Correlated),atlas_budget(),PhaseReuseRoute::Reuse).unwrap();
   assert_eq!(r.stop(),PhaseAtlasStop::Horizon);
   for phase in r.phases().iter().take(2){
    let model=RobertsonModel::new(m[phase.mode].rate).unwrap();let src=model.source();
    assert_eq!(serde_json::to_value(&g).unwrap(),serde_json::to_value(src).unwrap()["g"][0]);
    let slabs=phase.original_slab_diagnostics();
    for (tag,k) in [("first",0),("middle",slabs.len()/2),("last",slabs.len()-1)]{
     let s=&slabs[k];assert!(s.certificate.as_ref().unwrap().global_qualified);
     let t=s.times[0];let x=&s.native.y0[..2];
     let detail=json!({"solver":solver,"cert":cert,"mode":phase.mode,"position":tag,"slab":k,"time":t,"x":x,"rate":m[phase.mode].rate});
     let (p0,a)=measure(||RepresentedPolynomial::frozen_scalar(&g,t,x,[-0.25,1.25]));let p0=p0.unwrap();
     assert_eq!(p0.degree(),Some(1));
     println!("FROZEN {}",json!({"detail":detail,"coefficients":p0.exact_coefficients(),"clock":p0.exact_clock(),"allocation":a}));
     for route in ROOT_ROUTES{
      let (v,a)=measure(||p0.isolate(route,tol));let v=v.unwrap();assert_eq!(v.roots.len(),1);assert_eq!(v.roots[0].multiplicity,1);assert_eq!(v.work.companion_attempts,0);
      println!("ROOT {}",json!({"detail":detail,"method":route,"report":v,"allocation":a}));
     }
     let (v,a)=measure(||p0.isolate_affine_exact(tol));let v=v.unwrap().unwrap();
     println!("AFFINE {}",json!({"detail":detail,"root":v,"allocation":a}));
     for route in [InitRoute::Reuse,InitRoute::PreparedBatch,InitRoute::ScalarRoot]{
      let (v,a)=measure(||de::project(src,t,x,&[0.125],&[[-0.25,1.25]],policy().init,route,None));let v=v.unwrap();assert!(v.qualified);
      println!("INIT {}",json!({"detail":detail,"route":route,"report":v,"allocation":a}));
     }
    }
   }
  }
 }
}
