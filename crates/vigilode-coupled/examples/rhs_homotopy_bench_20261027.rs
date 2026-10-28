//! Deterministic work/accuracy experiment, including unsuccessful/overhead arms.
use serde_json::{Value, json};
use std::cell::Cell;
use vigilode_coupled::rhs_homotopy::{
    RhsAcceleration, RhsHomotopyConfig, RhsOnlyMap, solve_rhs_homotopy,
};

struct Midpoint {
    y: Vec<f64>,
    decay: Vec<f64>,
    cubic: f64,
    h: f64,
    rhs_calls: Cell<u64>,
}
impl RhsOnlyMap for Midpoint {
    fn dimension(&self) -> usize {
        self.y.len()
    }
    fn evaluate(&self, k: &[f64], out: &mut [f64]) -> Result<(), String> {
        self.rhs_calls.set(self.rhs_calls.get() + 1);
        for i in 0..k.len() {
            let ymid = self.y[i] + 0.5 * k[i];
            let cubic = if self.cubic == 0.0 {
                0.0
            } else {
                self.cubic * ymid.powi(3)
            };
            out[i] = -self.h * (self.decay[i] * ymid + cubic);
        }
        Ok(())
    }
}
fn bisect_midpoint(y: f64, k: f64, g: f64, h: f64) -> f64 {
    // F'(K)=1+h(k+3g(y+K/2)^2)/2 >=1 for the tested k,g,h>=0.
    let (mut lo, mut hi) = (-2.0 * y, 0.0);
    for _ in 0..90 {
        let mid = lo + 0.5 * (hi - lo);
        let ym = y + 0.5 * mid;
        let f = mid + h * (k * ym + g * ym.powi(3));
        if f > 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    lo + 0.5 * (hi - lo)
}
fn base(mode: RhsAcceleration, path: Vec<f64>) -> RhsHomotopyConfig {
    RhsHomotopyConfig {
        acceleration: mode,
        lambdas: path,
        residual_l2_tolerance: 1e-12,
        max_iterations_per_lambda: 4096,
        max_map_evaluations: 20000,
        ..RhsHomotopyConfig::default()
    }
}
fn midpoint_cell(n: usize, k: f64, g: f64, h: f64, mode: RhsAcceleration, path: Vec<f64>) -> Value {
    let map = Midpoint {
        y: (0..n).map(|i| 1.0 + 0.01 * ((i % 7) as f64)).collect(),
        decay: vec![k; n],
        cubic: g,
        h,
        rhs_calls: Cell::new(0),
    };
    let result = solve_rhs_homotopy(&map, &vec![0.0; n], &base(mode, path.clone()));
    let error = result.root.as_ref().map(|root| {
        root.iter()
            .enumerate()
            .map(|(i, &v)| (v - bisect_midpoint(map.y[i], k, g, h)).abs())
            .fold(0.0_f64, f64::max)
    });
    let physical_error = result.root.as_ref().map(|root| {
        root.iter()
            .enumerate()
            .map(|(i, &v)| {
                let y0 = map.y[i];
                let exact = y0 * (-k * h).exp()
                    / (1.0 + (g / k) * y0 * y0 * (1.0 - (-2.0 * k * h).exp())).sqrt();
                (y0 + v - exact).abs()
            })
            .fold(0.0_f64, f64::max)
    });
    json!({"family":"implicit-midpoint-polynomial-decay","n":n,"decay":k,"cubic":g,"h":h,"mode":mode,"lambdas":path,"success":result.root.is_some(),"max_stage_error_vs_bisection":error,"stage_error_cap":1e-10,"one_step_ode_error":physical_error,"rhs_callbacks":map.rhs_calls.get(),"jacobian_callbacks":0,"jvp_callbacks":0,"ad_callbacks":0,"matrix_factorizations":0,"newton_updates":0,"attempt":result})
}

struct ClassicalRk4 {
    h: f64,
    rhs_calls: Cell<u64>,
}
fn f(y: f64) -> f64 {
    -y - 0.5 * y * y * y
}
impl RhsOnlyMap for ClassicalRk4 {
    fn dimension(&self) -> usize {
        4
    }
    fn evaluate(&self, k: &[f64], out: &mut [f64]) -> Result<(), String> {
        self.rhs_calls.set(self.rhs_calls.get() + 4);
        out[0] = self.h * f(1.0);
        out[1] = self.h * f(1.0 + 0.5 * k[0]);
        out[2] = self.h * f(1.0 + 0.5 * k[1]);
        out[3] = self.h * f(1.0 + k[2]);
        Ok(())
    }
}
fn rk4_cell(mode: RhsAcceleration) -> Value {
    let map = ClassicalRk4 {
        h: 0.25,
        rhs_calls: Cell::new(0),
    };
    let k0 = map.h * f(1.0);
    let k1 = map.h * f(1.0 + 0.5 * k0);
    let k2 = map.h * f(1.0 + 0.5 * k1);
    let k3 = map.h * f(1.0 + k2);
    let oracle = [k0, k1, k2, k3];
    let result = solve_rhs_homotopy(&map, &[0.0; 4], &base(mode, vec![1.0]));
    let error = result.root.as_ref().map(|r| {
        r.iter()
            .zip(oracle)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0_f64, f64::max)
    });
    json!({"family":"classical-rk4-causal-target","mode":mode,"success":result.root.is_some(),"max_stage_error":error,"rhs_callbacks":map.rhs_calls.get(),"sequential_reference_rhs":4,"attempt":result})
}

fn trajectory(steps: usize, mode: RhsAcceleration, warm: bool) -> Value {
    let h = 1.0 / steps as f64;
    let mut y = 1.0_f64;
    let mut rhs = 0;
    let mut max_error = 0.0_f64;
    let mut max_stage_error = 0.0_f64;
    let mut max_residual = 0.0_f64;
    let mut accepted = 0;
    let mut previous_stage = 0.0;
    let mut samples = vec![json!({"t":0.0,"y":1.0,"exact":1.0})];
    for i in 0..steps {
        let map = Midpoint {
            y: vec![y],
            decay: vec![1.0],
            cubic: 0.5,
            h,
            rhs_calls: Cell::new(0),
        };
        let seed = if warm { previous_stage } else { 0.0 };
        let result = solve_rhs_homotopy(&map, &[seed], &base(mode, vec![1.0]));
        rhs += map.rhs_calls.get();
        let Some(root) = result.root else {
            return json!({"steps":steps,"mode":mode,"success":false,"failure":result.failure,"rhs_callbacks":rhs});
        };
        let oracle = bisect_midpoint(y, 1.0, 0.5, h);
        max_stage_error = max_stage_error.max((root[0] - oracle).abs());
        max_residual = max_residual.max(result.points.last().unwrap().residual_l2);
        y += root[0];
        previous_stage = root[0];
        accepted += 1;
        let t = (i + 1) as f64 * h;
        let exact = (-t).exp() / (1.0 + 0.5 * (1.0 - (-2.0 * t).exp())).sqrt();
        max_error = max_error.max((y - exact).abs());
        samples.push(json!({"t":t,"y":y,"exact":exact}));
    }
    let exact = (-1.0_f64).exp() / (1.0 + 0.5 * (1.0 - (-2.0_f64).exp())).sqrt();
    json!({"family":"implicit-midpoint-trajectory","steps":steps,"h":h,"mode":mode,"warm_previous_stage":warm,"success":true,"accepted_steps":accepted,"rhs_callbacks":rhs,"max_grid_error":max_error,"endpoint_error":(y-exact).abs(),"max_stage_error_vs_bisection":max_stage_error,"max_stage_residual":max_residual,"state":y,"samples":samples})
}

fn main() {
    let modes = [
        RhsAcceleration::Picard,
        RhsAcceleration::Screened,
        RhsAcceleration::SlowOnly,
    ];
    let mut stage_cells = Vec::new();
    // Frozen before execution: fast, slow, nonlinear, vector, noncontractive.
    for (n, k, g, h) in [
        (1, 1.0, 0.0, 0.2),
        (1, 1.0, 0.0, 1.98),
        (64, 1.0, 0.0, 1.98),
        (1, 1.0, 0.5, 0.5),
        (64, 10.0, 0.5, 0.18),
        (1, 1.0, 0.0, 2.2),
    ] {
        for mode in modes {
            stage_cells.push(midpoint_cell(n, k, g, h, mode, vec![1.0]));
        }
    }
    for mode in modes {
        stage_cells.push(midpoint_cell(1, 1.0, 0.5, 0.5, mode, vec![0.25, 0.5, 1.0]));
    }
    let causal: Vec<_> = modes.into_iter().map(rk4_cell).collect();
    let mut trajectories = Vec::new();
    for steps in [8, 16, 32, 64] {
        for mode in modes {
            for warm in [false, true] {
                trajectories.push(trajectory(steps, mode, warm));
            }
        }
    }
    println!("{}",serde_json::to_string_pretty(&json!({"schema":1,"target":"rhs-map-homotopy-native-20261027","stage_cells":stage_cells,"causal_cells":causal,"trajectories":trajectories,"interpretation":{"cost":"actual vector RHS callback entries including screening and continuation","bisection":"separate monotone root oracle, excluded from candidate cost","ode_error":"separate physical grid-error observation; matching stage roots does not increase RK order","timing":"not measured by this deterministic-count experiment"}})).unwrap());
}
