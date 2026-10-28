use rodas5p_core::{rodas5p_coefficients, directed::Interval};
use rodas5p_integrators::{StageTarget,QuadraticStageProblem,InverseWitness,certify_stage_target,doubling_certificate};
use serde_json::json;
fn main(){
 let coeffs=rodas5p_coefficients().unwrap();
 let target=StageTarget::sequential(&coeffs).unwrap();
 let problem=QuadraticStageProblem{jacobian:vec![vec![-1.0]],y:vec![1.0],h:0.0625,q:vec![0.0]};
 let stages=vec![vec![0.0];8];
 let witness=InverseWitness::diagonal(&problem,target.gamma).unwrap();
 for mode in ["constructor","zero-upper","empty-row"]{
  let mut w=witness.clone();
  if mode=="zero-upper"{w.upper[0][0]=0.0;}
  if mode=="empty-row"{w.upper[0].clear();}
  match certify_stage_target(&target,&problem,&stages,&[1.0],&[0.0],&w,1.0,0.0){
   Ok(c)=>println!("{}",json!({"case":"witness-integrity","mode":mode,"gamma":target.gamma,"status":"ok","certificate":c})),
   Err(e)=>println!("{}",json!({"case":"witness-integrity","mode":mode,"status":"rejected","error":e.to_string()}))
  }
 }
 for s in [1usize,2,4,8,9,16]{
  let mut t=StageTarget{id:"r4-exact-chain",snapshot_sha256:"standalone-exact-dyadic-fixture",gamma:0.0,c:vec![0.0;s],gamma_rows:vec![0.0;s],alpha_rows:(0..s).map(|i|vec![0.0;i]).collect(),coupling_rows:(0..s).map(|i|(0..i).map(|j|Interval::point(if j+1==i{1.0}else{0.0}).unwrap()).collect()).collect(),b:vec![0.0;s],btilde:vec![0.0;s]};
  t.b[s-1]=1.0;
  let p=QuadraticStageProblem{jacobian:vec![vec![1.0]],y:vec![1.0],h:1.0,q:vec![0.0]};
  let w=InverseWitness::diagonal(&p,t.gamma).unwrap();
  let k=vec![vec![0.0];s];
  let serial=certify_stage_target(&t,&p,&k,&[1.0],&[0.0],&w,1.0,0.0).unwrap();
  let doubling=doubling_certificate(&t,&p,&k,&[1.0],&[0.0],&w,1.0,0.0,0.0,1,1).unwrap();
  println!("{}",json!({"case":"stage-depth","s":s,"serial":serial,"doubling":doubling}));
 }
}
