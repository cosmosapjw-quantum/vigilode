use diffsol::{FaerLU, FaerMat, OdeBuilder, OdeSolverMethod, VectorHost};
use serde_json::{Value, json};
use std::{
    cell::Cell,
    io::{self, Read},
    time::Instant,
};
const MODEL_SOURCE: &str = "45037fc24e8f8101605f309e1fdb3a9a4e3df56d";

#[derive(Clone, Copy, Debug)]
enum Case {
    Decay,
    Driven,
    Logistic,
    Harmonic,
    Rapid,
    Damped,
    Mixed,
    Nonnormal,
    Large,
    Moderate,
    Stiff,
}
impl Case {
    fn parse(s: &str) -> Result<Self, String> {
        Ok(match s {
            "decay" => Self::Decay,
            "driven" => Self::Driven,
            "logistic" => Self::Logistic,
            "harmonic" => Self::Harmonic,
            "rapid" => Self::Rapid,
            "damped" => Self::Damped,
            "mixed-scales" => Self::Mixed,
            "nonnormal" => Self::Nonnormal,
            "large-vector" => Self::Large,
            "moderate-stiff" => Self::Moderate,
            "stiff-control" => Self::Stiff,
            _ => return Err("unknown case".into()),
        })
    }
    fn end(self) -> f64 {
        match self {
            Self::Logistic => 10.,
            Self::Harmonic => 12.,
            Self::Rapid | Self::Moderate => 2.,
            Self::Stiff => 1.,
            _ => 4.,
        }
    }
    fn paired(self) -> bool {
        matches!(
            self,
            Self::Harmonic | Self::Rapid | Self::Damped | Self::Nonnormal
        )
    }
    fn stiff(self) -> bool {
        matches!(self, Self::Moderate | Self::Stiff)
    }
    fn initial(self, i: usize) -> f64 {
        match self {
            Self::Logistic => 0.05 + 0.01 * (i % 8) as f64,
            Self::Mixed => 10_f64.powi((i % 12) as i32 - 8),
            Self::Nonnormal => (i % 2) as f64,
            Self::Harmonic | Self::Rapid | Self::Damped => 1. - (i % 2) as f64,
            _ => 1. / (1 + i % 8) as f64,
        }
    }
    fn rate(self, i: usize) -> f64 {
        match self {
            Self::Logistic => 0.25 + 0.1 * (i % 8) as f64,
            Self::Large => 0.5 + 0.05 * (i % 8) as f64,
            Self::Moderate => [1., 30., 300.][i % 3],
            Self::Stiff => [1., 1000., 100000.][i % 3],
            _ => [0.25, 1., 3., 0.5][i % 4],
        }
    }
    fn frequency(self, i: usize) -> f64 {
        let (a, b) = match self {
            Self::Rapid => (100., 5.),
            Self::Damped => (10., 1.),
            _ => (1., 0.25),
        };
        a + b * ((i / 2) % 4) as f64
    }
    fn rhs(self, t: f64, y: &[f64], out: &mut [f64]) {
        for i in 0..y.len() {
            out[i] = match self {
                Self::Harmonic | Self::Rapid | Self::Damped => {
                    let d = if matches!(self, Self::Damped) { 3. } else { 0. };
                    -d * y[i]
                        + if i % 2 == 0 {
                            -self.frequency(i) * y[i + 1]
                        } else {
                            self.frequency(i) * y[i - 1]
                        }
                }
                Self::Nonnormal => {
                    if i % 2 == 0 {
                        -y[i] + 30. * y[i + 1]
                    } else {
                        -2. * y[i]
                    }
                }
                Self::Logistic => self.rate(i) * y[i] * (1. - y[i]),
                Self::Driven => -self.rate(i) * y[i] + t.cos() + self.rate(i) * t.sin(),
                _ => -self.rate(i) * y[i],
            };
        }
    }
    fn exact(self, t: f64, n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| match self {
                Self::Logistic => {
                    1. / (1. + (1. / self.initial(i) - 1.) * (-self.rate(i) * t).exp())
                }
                Self::Driven => t.sin() + self.initial(i) * (-self.rate(i) * t).exp(),
                Self::Harmonic | Self::Rapid | Self::Damped => {
                    let env = if matches!(self, Self::Damped) {
                        (-3. * t).exp()
                    } else {
                        1.
                    };
                    env * if i % 2 == 0 {
                        (self.frequency(i) * t).cos()
                    } else {
                        (self.frequency(i) * t).sin()
                    }
                }
                Self::Nonnormal => {
                    if i % 2 == 0 {
                        30. * ((-t).exp() - (-2. * t).exp())
                    } else {
                        (-2. * t).exp()
                    }
                }
                _ => self.initial(i) * (-self.rate(i) * t).exp(),
            })
            .collect()
    }
}
fn run(req: Value) -> Result<Value, String> {
    let name = req["case"].as_str().ok_or("missing case")?;
    let case = Case::parse(name)?;
    let n = req["dimension"].as_u64().ok_or("invalid dimension")? as usize;
    let expected = match case {
        Case::Mixed | Case::Moderate | Case::Stiff => n == 12,
        Case::Large => n == 4096 || n == 16384,
        _ => n == 8,
    };
    if n == 0 || case.paired() && n % 2 != 0 || !expected {
        return Err("unregistered dimension or odd paired system".into());
    }
    let rtol = req["rtol"].as_f64().ok_or("missing rtol")?;
    let atol = req["atol"].as_f64().ok_or("missing atol")?;
    if !rtol.is_finite() || rtol <= 0. || !atol.is_finite() || atol <= 0. {
        return Err("invalid tolerance".into());
    }
    if req["source_commit"].as_str() != Some(MODEL_SOURCE) {
        return Err("immutable model source mismatch".into());
    }
    let max_steps = match req.get("max_steps") {
        Some(v) => v.as_u64().ok_or("invalid max_steps")?,
        None => 100000,
    };
    if req
        .get("mode")
        .is_some_and(|v| v.as_str() != Some("diffsol"))
    {
        return Err("unsupported adapter mode".into());
    }
    let adapter_source =
        option_env!("VIGILODE_ADAPTER_SOURCE_COMMIT").unwrap_or("UNCOMMITTED_DEVELOPMENT");
    if req
        .get("adapter_source_commit")
        .is_some_and(|v| v.as_str() != Some(adapter_source))
    {
        return Err("adapter source identity mismatch".into());
    }
    let start = Instant::now();
    let initial: Vec<f64> = (0..n).map(|i| case.initial(i)).collect();
    let requested: Vec<f64> = (0..5).map(|i| case.end() * i as f64 / 4.).collect();
    let mut times = vec![0.];
    let mut states = vec![initial.clone()];
    let mut last_time = 0.;
    let mut last_state = initial.clone();
    let mut step_calls = 0u64;
    let rhs_attempted = Cell::new(0u64);
    let rhs_completed = Cell::new(0u64);
    let jvp_calls = Cell::new(0u64);
    let mut stats = None;
    macro_rules! solve {($builder:expr,$method:ident $(,$linear:ty)?)=>{{
        let problem=$builder.init(|_p,_t,y|{y.as_mut_slice().copy_from_slice(&initial);},n).build().map_err(|e|e.to_string());
        match problem {Err(e)=>Err(e),Ok(problem)=>{
            let solver=problem.$method$(::<$linear>)?();
            match solver {Err(e)=>Err(e.to_string()),Ok(mut s)=>{
                let outcome=(||->Result<(),String>{
                    s.set_stop_time(case.end()).map_err(|e|e.to_string())?;
                    for target in requested.iter().skip(1){
                        while s.state().t<*target {
                            if step_calls>=max_steps{return Err("max_steps public step-call cap reached".into())}
                            step_calls+=1;
                            let result=s.step();
                            last_time=s.state().t;last_state.copy_from_slice(s.state().y.as_slice());
                            result.map_err(|e|e.to_string())?;
                            if !last_time.is_finite()||!last_state.iter().all(|x|x.is_finite()){return Err("nonfinite accepted state".into())}
                        }
                        let y=s.interpolate(*target).map_err(|e|e.to_string())?;
                        if !y.as_slice().iter().all(|x|x.is_finite()){return Err("nonfinite interpolated state".into())}
                        times.push(*target);states.push(y.as_slice().to_vec());
                    }Ok(())})();
                stats=Some(s.get_statistics().clone());outcome
            }}
        }}
    }};}
    let builder = OdeBuilder::<FaerMat<f64>>::new()
        .p([0.])
        .rtol(rtol)
        .atol([atol])
        .h0(1e-3)
        .t0(0.);
    let rhs = |y: &diffsol::FaerVec<f64>,
               _p: &diffsol::FaerVec<f64>,
               t: f64,
               out: &mut diffsol::FaerVec<f64>| {
        rhs_attempted.set(rhs_attempted.get() + 1);
        case.rhs(t, y.as_slice(), out.as_mut_slice());
        rhs_completed.set(rhs_completed.get() + 1);
    };
    let outcome = if case.stiff() {
        solve!(
            builder.rhs_implicit(rhs, |_y, _p, _t, v, out| {
                jvp_calls.set(jvp_calls.get() + 1);
                for i in 0..n {
                    out[i] = -case.rate(i) * v[i];
                }
            }),
            bdf,
            FaerLU<f64>
        )
    } else {
        solve!(builder.rhs(rhs), tsit45)
    };
    let elapsed = start.elapsed().as_secs_f64();
    let stats = serde_json::to_value(stats).map_err(|e| e.to_string())?;
    let reference: Vec<Vec<f64>> = times.iter().map(|t| case.exact(*t, n)).collect();
    let (mut abs, mut norm, mut wrms) = (0_f64, 0_f64, 0_f64);
    for (ys, rs) in states.iter().zip(&reference) {
        let mut sum = 0.;
        for i in 0..n {
            let e = (ys[i] - rs[i]).abs();
            abs = abs.max(e);
            norm = norm.max(e / 1_f64.max(initial[i].abs()).max(rs[i].abs()));
            sum += (e / (atol + rtol * rs[i].abs())).powi(2);
        }
        wrms = wrms.max((sum / n as f64).sqrt());
    }
    let mut record = req;
    let obj = record.as_object_mut().ok_or("request must be object")?;
    let fields = json!({"schema":"vigilode.nonstiff-benchmark-run.v1","adapter_source_commit":adapter_source,"mode":"diffsol","end":case.end(),"success":outcome.is_ok()&&times.len()==5,"message":outcome.err().unwrap_or_else(||"success".into()),"elapsed_seconds":elapsed,"requested_times":requested,"times":times,"states":states,"reference":reference,"max_abs_error":abs,"max_normalized_error":norm,"max_reference_scaled_wrms":wrms,"last_accepted_time":last_time,"last_accepted_state":last_state,
    "work":{"rhs_attempted":rhs_attempted.get(),"rhs_completed":rhs_completed.get(),"jvp_calls":jvp_calls.get(),"public_step_calls":step_calls,"solver_statistics":stats,"linear_solves":null,"hot_loop_allocations":null,"unavailable_reason":"Diffsol exposes linear setups and Newton iterations, not exact linear solve or allocation counts"},
    "adapter":{"solver":"diffsol","version":"0.16.1","method":if case.stiff(){"BDF"}else{"Tsit45"},"matrix_backend":"faer","linear_solver":if case.stiff(){Some("FaerLU")}else{None},"jacobian":if case.stiff(){"analytic JVP; dense assembly for dimension 12 controls"}else{"none"},"initial_step":0.001,"automatic_method_switching":false,"max_steps_policy":"global public step() call cap; internal rejections included in solver_statistics and RHS counters","output_policy":"one adaptive history; built-in dense interpolation at five requested times; end stop-time","callback_language":"Rust","adapter_source_commit":option_env!("VIGILODE_ADAPTER_SOURCE_COMMIT").unwrap_or("UNCOMMITTED_DEVELOPMENT")},"timing_authority":"diagnostic_only","timing_scope":"fresh initial data, problem/workspace construction, solve callbacks and output collection; reference/error/JSON excluded"});
    obj.extend(fields.as_object().unwrap().clone());
    Ok(record)
}
fn main() {
    let mut text = String::new();
    let result = io::stdin()
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())
        .and_then(|_| serde_json::from_str(&text).map_err(|e| e.to_string()))
        .and_then(run);
    match result {
        Ok(v) => println!("{v}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rhs_matches_reference_trajectory_derivative_all_families() {
        for name in [
            "decay",
            "driven",
            "logistic",
            "harmonic",
            "rapid",
            "damped",
            "mixed-scales",
            "nonnormal",
            "large-vector",
            "moderate-stiff",
            "stiff-control",
        ] {
            let c = Case::parse(name).unwrap();
            let n = if matches!(c, Case::Large) { 16384 } else { 12 };
            let t = 0.3;
            let h = 1e-6;
            let y = c.exact(t, n);
            let yp = c.exact(t + h, n);
            let ym = c.exact(t - h, n);
            let mut dy = vec![0.; n];
            c.rhs(t, &y, &mut dy);
            for i in 0..n {
                let numerical = (yp[i] - ym[i]) / (2. * h);
                assert!(
                    (dy[i] - numerical).abs() < 1e-7 * dy[i].abs().max(1.),
                    "{name} {i}: {} vs {numerical}",
                    dy[i]
                );
            }
            for (i, reference) in c.exact(0., n).iter().enumerate() {
                assert!((reference - c.initial(i)).abs() <= 5e-14 * c.initial(i).abs().max(1.));
            }
        }
    }
}
