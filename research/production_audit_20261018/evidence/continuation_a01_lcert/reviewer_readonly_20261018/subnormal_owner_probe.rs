use rodas5p_integrators::{CollectiveModel,PacketWorkCounters,compile_packet,generate_candidate,CausalWeights};
use rodas5p_integrators::packet_window::{RepresentedPacketCurve,PacketFrozenOutput};
fn main(){
 for case in 0..2 {
  let tiny=f64::from_bits(1);
  let (z,width,chi,h,eta) = if case==0 {((tiny,0.0),0.0,0.0,0.125,0.0)} else {((0.3,-0.2),0.8,0.3,0.1,tiny)};
  let model=CollectiveModel::new(vec![0.5],vec![1.0],vec![z],width,chi,1.25,0.0).unwrap();
  let mut work=PacketWorkCounters::default(); let mut packet=compile_packet(&model,h,4,&mut work).unwrap();
  let table=CausalWeights::compile(4,12); let mut cand=generate_candidate(&packet,4,2,&table,&mut work).unwrap();
  let curve=RepresentedPacketCurve::freeze(&cand,&packet,&mut work).unwrap();
  let mut initial_radii=[eta]; let owner=PacketFrozenOutput::new(curve,&initial_radii,&mut work).unwrap();
  initial_radii[0]=f64::MAX; std::hint::black_box(initial_radii);
  cand.mean[0]=(f64::NAN,f64::NAN); packet.free[0]=(f64::MAX,f64::MAX);
  let mut ws=owner.workspace(&mut work).unwrap();
  let times = if case==0 {[0.0,0.0625,0.125]} else {[0.0,tiny,0.05]};
  for time in times {
   let saved={let v=owner.enclose_at(time,&mut ws,&mut work).unwrap();
    let values=[z.0,z.1,width,chi,time,eta,v.modes()[0].0,v.modes()[0].1,v.mode_radii()[0],v.mean().0,v.mean().1,v.mean_radius()];
    print!("{}",case); for x in values {print!(" {}",x.to_bits());} println!();
    [v.modes()[0].0.to_bits(),v.modes()[0].1.to_bits(),v.mode_radii()[0].to_bits(),v.mean().0.to_bits(),v.mean().1.to_bits(),v.mean_radius().to_bits()]};
   let before=work; assert!(owner.enclose_at(f64::NAN,&mut ws,&mut work).is_err());
   assert_eq!(work.curve_frozen_query_attempts,before.curve_frozen_query_attempts+1); assert_eq!(work.curve_frozen_query_refusals,before.curve_frozen_query_refusals+1);
   let v=owner.enclose_at(time,&mut ws,&mut work).unwrap(); let after=[v.modes()[0].0.to_bits(),v.modes()[0].1.to_bits(),v.mode_radii()[0].to_bits(),v.mean().0.to_bits(),v.mean().1.to_bits(),v.mean_radius().to_bits()]; assert_eq!(saved,after);
  }
 }
}
