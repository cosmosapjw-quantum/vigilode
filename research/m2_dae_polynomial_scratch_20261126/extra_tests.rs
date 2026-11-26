
#[test]
#[ignore="fixed native slab residual and guard candidate portfolio"]
fn polynomial_portfolio(){
 use serde_json::json;
 for solver in [EvolutionRoute::Mass,EvolutionRoute::Prepared5,EvolutionRoute::Scalar5]{
  for original in [ConservationRoute::Max,ConservationRoute::Integral4]{
   let m=graph("alternating",solver,original);
   let r=evolve_robertson_phase_atlas(&m,0,initial(2f64.powi(-26)),[0.,0.002],&opts(),policy(),pm(MultiGuardRoute::Correlated),atlas_budget(),PhaseReuseRoute::Reuse).unwrap();
   assert_eq!(r.stop(),PhaseAtlasStop::Horizon);
   for phase in r.phases().iter().take(2){
    let model=RobertsonModel::new(m[phase.mode].rate).unwrap();
    let slabs=phase.original_slab_diagnostics();
    for (tag,k) in [("first",0),("middle",slabs.len()/2),("last",slabs.len()-1)]{
     let s=&slabs[k];let c=s.certificate.as_ref().unwrap();assert!(c.global_qualified);
     let y=power(s);
     let detail=json!({"solver":solver,"original":original,"mode":phase.mode,"position":tag,"slab":k,"times":s.times,"rate":m[phase.mode].rate});
     for route in [ConservationRoute::Max,ConservationRoute::Integral4,ConservationRoute::Integral16]{
      let (v,a)=measure(||model.certify(s.times,&y,&[[-0.25,1.25]],policy().tube_radius,Some(c.incoming),policy().init.condition_cap,route));
      let v=v.unwrap();assert!(v.global_qualified);
      if route==original{assert_eq!(serde_json::to_string(c).unwrap(),serde_json::to_string(&v).unwrap());}
      println!("RESIDUAL {}",json!({"detail":detail,"route":route,"allocation":a,"certificate":v}));
     }
     for route in [GuardValueRoute::Transport,GuardValueRoute::Box]{
      let (g,a)=measure(||de::GuardProof::new(model.source(),&m[phase.mode].guards[0],&y,c,GuardDerivativeRoute::ConservedSchur,route));
      let g=g.unwrap();
      for t in [s.times[0],(s.times[0]+s.times[1])*0.5,s.times[1]]{
       let (v,b)=measure(||g.value_at(t,c.whole_x_error.unwrap()));let v=v.unwrap();assert_eq!(b.requests(),0);
       println!("GUARD {}",json!({"detail":detail,"route":route,"time":t,"radius":c.whole_x_error,"value":v.as_pair(),"build":a,"query":b,"guard":g.evidence}));
      }
     }
     // A degree-16 guard from four quartic affine factors exercises
     // bounded composition in the actual guard proof path.
     let a=F{constant:0.25,time:0.125,state:vec![(0,0.5),(1,-0.25),(2,0.125),(1,0.25)]};
     let g=p(vec![tr(1.,vec![a.clone(),a.clone(),a.clone(),a])]);
     let proof=de::GuardProof::new(model.source(),&g,&y,c,GuardDerivativeRoute::ConservedSchur,GuardValueRoute::Transport).unwrap();
     assert_eq!(proof.hint_coefficients().len(),17);
     let (v,a)=measure(||proof.value_at((s.times[0]+s.times[1])*0.5,c.whole_x_error.unwrap()));v.unwrap();assert_eq!(a.requests(),0);
     println!("DEGREE {}",json!({"detail":detail,"degree":16,"allocation":a}));
    }
   }
  }
 }
}
