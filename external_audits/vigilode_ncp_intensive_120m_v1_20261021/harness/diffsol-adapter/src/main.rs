use diffsol::{DenseMatrix, MatrixCommon, NalgebraLU, NalgebraMat, OdeBuilder, OdeSolverMethod, VectorView};
use serde_json::{json, Value};
use std::{env, fs, time::Instant};

fn run(c: &Value, alg: &str, reps: usize) -> Result<Value, Box<dyn std::error::Error>> {
    let model = c["model"].as_str().unwrap();
    let y0: Vec<f64> = serde_json::from_value(c["initial_state"].clone())?;
    let n=y0.len();
    let times: Vec<f64> = serde_json::from_value(c["output_times"].clone())?;
    let t0=c["t_span"][0].as_f64().unwrap();
    let rtol=c["rtol"].as_f64().unwrap();
    let atol: Vec<f64>=serde_json::from_value(c["atol"].clone())?;
    let p: Vec<f64> = match model {
        "diagonal_stiff" => serde_json::from_value(c["parameters"]["rates"].clone())?,
        "rotation" => vec![c["parameters"]["omega"].as_f64().unwrap()],
        "damping" => vec![c["parameters"]["omega"].as_f64().unwrap(),c["parameters"]["zeta"].as_f64().unwrap()],
        "nonnormal" => vec![c["parameters"]["a"].as_f64().unwrap(),c["parameters"]["b"].as_f64().unwrap(),c["parameters"]["d"].as_f64().unwrap()],
        "periodic_heat" => vec![c["parameters"]["diffusion"].as_f64().unwrap()],
        "logistic" => vec![c["parameters"]["rate"].as_f64().unwrap(),c["parameters"]["capacity"].as_f64().unwrap()],
        "robertson" => vec![c["parameters"]["k1"].as_f64().unwrap(),c["parameters"]["k2"].as_f64().unwrap(),c["parameters"]["k3"].as_f64().unwrap()],
        "vanderpol" => vec![c["parameters"]["mu"].as_f64().unwrap()],
        _ => return Err(format!("unsupported model {model}").into()),
    };
    let mut warmup=Vec::new(); let mut measurements=Vec::new();
    for rep in 0..(2+reps) {
        let setup=Instant::now();
        let init_y=y0.clone();
        let rhs_model=model.to_string(); let jac_model=model.to_string();
        let b=OdeBuilder::<NalgebraMat<f64>>::new().t0(t0).rtol(rtol).atol(atol.clone()).p(p.clone())
            .rhs_implicit(
                move |x, p, _t, out| match rhs_model.as_str() {
                    "diagonal_stiff" => for i in 0..out.len(){out[i]=p[i]*x[i]},
                    "rotation" => {out[0]=-p[0]*x[1];out[1]=p[0]*x[0]},
                    "damping" => {out[0]=x[1];out[1]=-p[0]*p[0]*x[0]-2.0*p[1]*p[0]*x[1]},
                    "nonnormal" => {out[0]=p[0]*x[0]+p[1]*x[1];out[1]=p[2]*x[1]},
                    "periodic_heat" => {let d=p[0]*(out.len() as f64).powi(2);let m=out.len();for i in 0..m{out[i]=d*(x[(i+m-1)%m]-2.0*x[i]+x[(i+1)%m]);}},
                    "logistic" => out[0]=p[0]*x[0]*(1.0-x[0]/p[1]),
                    "robertson" => {let a=p[0]*x[0];let b=p[1]*x[1]*x[2];let d=p[2]*x[1]*x[1];out[0]=-a+b;out[1]=a-b-d;out[2]=d},
                    "vanderpol" => {out[0]=x[1];out[1]=p[0]*(1.0-x[0]*x[0])*x[1]-x[0]},
                    _ => unreachable!()
                },
                move |x, p, _t, v, out| match jac_model.as_str() {
                    "diagonal_stiff" => for i in 0..out.len(){out[i]=p[i]*v[i]},
                    "rotation" => {out[0]=-p[0]*v[1];out[1]=p[0]*v[0]},
                    "damping" => {out[0]=v[1];out[1]=-p[0]*p[0]*v[0]-2.0*p[1]*p[0]*v[1]},
                    "nonnormal" => {out[0]=p[0]*v[0]+p[1]*v[1];out[1]=p[2]*v[1]},
                    "periodic_heat" => {let d=p[0]*(out.len() as f64).powi(2);let m=out.len();for i in 0..m{out[i]=d*(v[(i+m-1)%m]-2.0*v[i]+v[(i+1)%m]);}},
                    "logistic" => out[0]=p[0]*(1.0-2.0*x[0]/p[1])*v[0],
                    "robertson" => {let j12=p[1]*x[2];let j13=p[1]*x[1];let j22=-p[1]*x[2]-2.0*p[2]*x[1];let j23=-p[1]*x[1];let j32=2.0*p[2]*x[1];out[0]=-p[0]*v[0]+j12*v[1]+j13*v[2];out[1]=p[0]*v[0]+j22*v[1]+j23*v[2];out[2]=j32*v[1]},
                    "vanderpol" => {out[0]=v[1];out[1]=(-2.0*p[0]*x[0]*x[1]-1.0)*v[0]+p[0]*(1.0-x[0]*x[0])*v[1]},
                    _ => unreachable!()
                },
            ).init(move |_p,_t,y| y.copy_from_slice(&init_y),n).build()?;
        let setup_seconds=setup.elapsed().as_secs_f64();
        let solve_start=Instant::now();
        let (mat, reason)=if alg=="Tsit45" { let mut s=b.tsit45()?; s.solve_dense(&times)? } else { let mut s=b.bdf::<NalgebraLU<f64>>()?; s.solve_dense(&times)? };
        let solve_seconds=solve_start.elapsed().as_secs_f64();
        let states: Vec<Vec<f64>>=(0..mat.ncols()).map(|j| (0..mat.nrows()).map(|i| mat.column(j).get_index(i)).collect()).collect();
        let row=json!({"repetition":if rep<2{rep+1}else{rep-1},"setup_seconds":setup_seconds,"elapsed_seconds":solve_seconds,"states":states,"stop_reason":format!("{reason:?}")});
        if rep<2 {warmup.push(row);} else {measurements.push(row);}
    }
    Ok(json!({"status":"OK","diffsol_version":"0.17.1","algorithm":alg,"linear_solver":"NalgebraLU<f64> dense","jacobian":"analytic Jv closure","warmup":warmup,"measurements":measurements,"solver_counters":null}))
}
fn main(){
 let a:Vec<String>=env::args().collect();
 let request:Value=serde_json::from_slice(&fs::read(&a[1]).unwrap()).unwrap();
 let result=run(&request["case"],request["algorithm"].as_str().unwrap(),7);
 match result {Ok(v)=>println!("{}",v),Err(e)=>{eprintln!("{}",e);std::process::exit(2)}}
}
