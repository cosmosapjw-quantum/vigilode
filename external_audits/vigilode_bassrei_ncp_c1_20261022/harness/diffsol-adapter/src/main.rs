//! diffsol 0.17.1 comparator of the BASS/rei NCP C1 campaign, on the VigilODE
//! adapter kernels themselves (`lrs_photon_fv_support`, included by
//! `#[path]` from the checkout at `CAMP/src`). Usage:
//!
//! `vigilode_bassrei_diffsol --request PATH`
//!
//! Requests (one JSON object, written by `harness/runners.py`/`parity.py`):
//!
//! ```text
//! {"model": <scenarios.model_description>, "algorithm": "bdf" | "tsit45",
//!  "rtol": r, "atol": [..]}                -> trajectory at the outputs
//! {"mode": "probe", "model": .., "t": t, "states": [[..]]}  -> rhs per state
//! ```
//!
//! BDF uses diffsol's dense `NalgebraLU`; diffsol assembles its Jacobian from
//! the Jacobian-vector closure. For the linear P02 operator `J v = f(t, v)`
//! exactly; for the bordered and chemistry models `J v` is the adapter's
//! analytic dense Jacobian at `(t, x)` (cached per point) times `v`. A
//! kernel error (the Case-A domain refusal) returns a NaN residual, which
//! diffsol treats as a failed step. Wall time is diagnostic only.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;
use std::{env, fs};

use diffsol::{
    DenseMatrix as _, MatrixCommon as _, NalgebraLU, NalgebraMat, OdeBuilder, OdeSolverMethod as _,
    VectorView as _,
};
use rodas5p_core::{CoreError, CoreResult, DenseMatrix};
use serde_json::{Value, json};

#[path = "../../src/crates/rodas5p-integrators/examples/lrs_photon_fv_support/mod.rs"]
#[allow(dead_code, unused_imports, clippy::all)]
mod support;
use support::border::{BorderParams, Bordered};
use support::chem::{CaseATable, ChemGrid, ChemModel, ChemParams};
use support::{Absorber, Boundary, Coordinate, Geometry, Layout, Model};

enum Kind {
    P02(Arc<Model>),
    Bordered(Arc<Bordered>),
    Chem(Arc<ChemModel>),
}

impl Kind {
    fn n(&self) -> usize {
        match self {
            Kind::P02(m) => m.layout.n(),
            Kind::Bordered(b) => b.n(),
            Kind::Chem(c) => c.n(),
        }
    }
    fn rhs(&self, t: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
        match self {
            Kind::P02(m) => m.rhs(t, y, out),
            Kind::Bordered(b) => b.rhs(t, y, out),
            Kind::Chem(c) => c.rhs(t, y, out),
        }
    }
    fn dense(&self, t: f64, y: &[f64], m: &mut DenseMatrix) -> CoreResult<()> {
        match self {
            Kind::P02(p) => p.dense_fill(t, m),
            Kind::Bordered(b) => b.dense_fill(t, y, m),
            Kind::Chem(c) => c.dense_fill(t, y, m),
        }
    }
    fn linear(&self) -> bool {
        matches!(self, Kind::P02(_))
    }
}

fn bad(msg: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(msg.into())
}

fn f64_of(v: &Value, key: &str) -> CoreResult<f64> {
    v.get(key).and_then(Value::as_f64).ok_or_else(|| bad(format!("missing number {key}")))
}

fn usize_of(v: &Value, key: &str) -> CoreResult<usize> {
    v.get(key)
        .and_then(Value::as_u64)
        .map(|x| x as usize)
        .ok_or_else(|| bad(format!("missing integer {key}")))
}

fn vec_of(v: &Value, key: &str) -> CoreResult<Vec<f64>> {
    v.get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| bad(format!("missing array {key}")))?
        .iter()
        .map(|x| x.as_f64().ok_or_else(|| bad(format!("non-number in {key}"))))
        .collect()
}

fn photon_model(d: &Value) -> CoreResult<Model> {
    let l = d.get("layout").ok_or_else(|| bad("missing layout"))?;
    let coordinate = match l.get("coordinate").and_then(Value::as_str) {
        Some("physical") => Coordinate::Physical,
        Some("comoving") => Coordinate::Comoving,
        _ => return Err(bad("coordinate")),
    };
    let boundary = match l.get("boundary").and_then(Value::as_str) {
        Some("outflow") => Boundary::Outflow,
        Some("closed") => Boundary::Closed,
        _ => return Err(bad("boundary")),
    };
    let layout = Layout::new(vec_of(l, "q_edges")?, vec_of(l, "mu_edges")?, coordinate, boundary)?;
    let g = d.get("geometry").ok_or_else(|| bad("missing geometry"))?;
    let geometry = match g.get("kind").and_then(Value::as_str) {
        Some("constant") => Geometry::Constant { h: f64_of(g, "h")?, s: f64_of(g, "s")? },
        Some("power_law") => Geometry::PowerLaw {
            hh: f64_of(g, "hh")?,
            s0: f64_of(g, "s0")?,
            p: f64_of(g, "p")?,
        },
        _ => return Err(bad("geometry kind")),
    };
    let absorber = match d.get("absorber") {
        None | Some(Value::Null) => None,
        Some(a) => Some(Absorber { k0: f64_of(a, "k0")?, q_chi: f64_of(a, "q_chi")? }),
    };
    Model::new(layout, geometry, f64_of(d, "gamma")?, absorber, false)
}

fn kind_of(d: &Value) -> CoreResult<Kind> {
    match d.get("family").and_then(Value::as_str) {
        Some("p02") => Ok(Kind::P02(Arc::new(photon_model(d)?))),
        Some("bordered") => {
            let b = d.get("border").ok_or_else(|| bad("missing border"))?;
            let p = BorderParams {
                k: f64_of(b, "k")?,
                alpha: f64_of(b, "alpha")?,
                beta: f64_of(b, "beta")?,
                kappa: f64_of(b, "kappa")?,
                tau: f64_of(b, "tau")?,
                gamma0: f64_of(b, "gamma0")?,
            };
            Ok(Kind::Bordered(Arc::new(Bordered::new(Arc::new(photon_model(d)?), p)?)))
        }
        Some("chem") => {
            let path = d.get("table_path").and_then(Value::as_str).ok_or_else(|| bad("table_path"))?;
            let table = CaseATable::from_csv_path(path)?;
            let g = d.get("grid").ok_or_else(|| bad("missing grid"))?;
            let grid = ChemGrid {
                n1: usize_of(g, "n1")?,
                n2: usize_of(g, "n2")?,
                n3: usize_of(g, "n3")?,
                nm: usize_of(g, "nm")?,
            };
            let p = d.get("params").ok_or_else(|| bad("missing params"))?;
            let params = ChemParams { h0: f64_of(p, "H0")?, amp: f64_of(p, "amp")?, tau_m: f64_of(p, "tau_m")? };
            Ok(Kind::Chem(Arc::new(ChemModel::new(table, grid, params)?)))
        }
        _ => Err(bad("unknown family")),
    }
}

struct Cache {
    t: f64,
    x: Vec<f64>,
    valid: bool,
    m: DenseMatrix,
}

fn solve(kind: Rc<Kind>, d: &Value, alg: &str, rtol: f64, atol: Vec<f64>) -> Result<Value, String> {
    let n = kind.n();
    let y0 = vec_of(d, "y0").map_err(|e| e.to_string())?;
    let outs = vec_of(d, "outputs").map_err(|e| e.to_string())?;
    if y0.len() != n || atol.len() != n {
        return Err(format!("shape: y0 {} atol {} n {n}", y0.len(), atol.len()));
    }
    let times: Vec<f64> = outs[1..].to_vec();
    let calls = Rc::new([Cell::new(0u64), Cell::new(0u64), Cell::new(0u64)]);
    let buf = Rc::new(RefCell::new((vec![0.0; n], vec![0.0; n], vec![0.0; n])));
    let cache = Rc::new(RefCell::new(Cache { t: f64::NAN, x: vec![0.0; n], valid: false, m: DenseMatrix::zeros(n, n) }));
    let (k1, b1, c1) = (kind.clone(), buf.clone(), calls.clone());
    let (k2, b2, c2, cache2) = (kind.clone(), buf.clone(), calls.clone(), cache.clone());
    let y0c = y0.clone();
    let setup = Instant::now();
    let problem = OdeBuilder::<NalgebraMat<f64>>::new()
        .t0(0.0)
        .rtol(rtol)
        .atol(atol)
        .p(vec![0.0])
        .rhs_implicit(
            move |x, _p, t, out| {
                c1[0].set(c1[0].get() + 1);
                let mut bb = b1.borrow_mut();
                let (yb, ob, _) = &mut *bb;
                for i in 0..yb.len() {
                    yb[i] = x[i];
                }
                if k1.rhs(t, yb, ob).is_err() {
                    ob.fill(f64::NAN);
                }
                for i in 0..ob.len() {
                    out[i] = ob[i];
                }
            },
            move |x, _p, t, v, out| {
                c2[1].set(c2[1].get() + 1);
                let mut bb = b2.borrow_mut();
                let (yb, ob, vb) = &mut *bb;
                for i in 0..vb.len() {
                    vb[i] = v[i];
                }
                if k2.linear() {
                    if k2.rhs(t, vb, ob).is_err() {
                        ob.fill(f64::NAN);
                    }
                } else {
                    for i in 0..yb.len() {
                        yb[i] = x[i];
                    }
                    let mut c = cache2.borrow_mut();
                    let same = c.valid
                        && c.t.to_bits() == t.to_bits()
                        && c.x.iter().zip(yb.iter()).all(|(a, b)| a.to_bits() == b.to_bits());
                    if !same {
                        c2[2].set(c2[2].get() + 1);
                        let Cache { m, x: cx, .. } = &mut *c;
                        let ok = k2.dense(t, yb, m).is_ok();
                        cx.copy_from_slice(yb);
                        c.t = t;
                        c.valid = ok;
                    }
                    if c.valid {
                        for (r, o) in ob.iter_mut().enumerate() {
                            let row = c.m.row(r);
                            let mut s = 0.0;
                            for (a, b) in row.iter().zip(vb.iter()) {
                                s += a * b;
                            }
                            *o = s;
                        }
                    } else {
                        ob.fill(f64::NAN);
                    }
                }
                for i in 0..ob.len() {
                    out[i] = ob[i];
                }
            },
        )
        .init(
            move |_p, _t, y| {
                for i in 0..y0c.len() {
                    y[i] = y0c[i];
                }
            },
            n,
        )
        .build()
        .map_err(|e| e.to_string())?;
    let setup_seconds = setup.elapsed().as_secs_f64();
    let start = Instant::now();
    let (mat, reason) = if alg == "tsit45" {
        let mut s = problem.tsit45().map_err(|e| e.to_string())?;
        s.solve_dense(&times).map_err(|e| e.to_string())?
    } else {
        let mut s = problem.bdf::<NalgebraLU<f64>>().map_err(|e| e.to_string())?;
        s.solve_dense(&times).map_err(|e| e.to_string())?
    };
    let solve_seconds = start.elapsed().as_secs_f64();
    let mut states: Vec<Vec<f64>> = vec![y0];
    for j in 0..mat.ncols() {
        states.push((0..mat.nrows()).map(|i| mat.column(j).get_index(i)).collect());
    }
    Ok(json!({"status": "OK", "times": outs, "states": states, "solve_seconds": solve_seconds,
        "setup_seconds": setup_seconds, "stop_reason": format!("{reason:?}"),
        "counters": {"rhs_calls": calls[0].get(), "jac_mul_calls": calls[1].get(),
                     "dense_jacobian_fills": calls[2].get()},
        "diffsol_version": option_env!("NCPC1_DIFFSOL_VERSION").unwrap_or("unrecorded"), "algorithm": alg,
        "linear_solver": if alg == "tsit45" { "none" } else { "NalgebraLU<f64> dense" },
        "timing_authority": "diagnostic_only"}))
}

fn probe(kind: &Kind, req: &Value) -> Result<Value, String> {
    let t = f64_of(req, "t").map_err(|e| e.to_string())?;
    let n = kind.n();
    let mut rhs = Vec::new();
    for s in req.get("states").and_then(Value::as_array).ok_or("missing states")? {
        let y: Vec<f64> = s
            .as_array()
            .ok_or("state")?
            .iter()
            .map(|x| x.as_f64().ok_or("number"))
            .collect::<Result<_, _>>()?;
        let mut out = vec![0.0; n];
        kind.rhs(t, &y, &mut out).map_err(|e| e.to_string())?;
        rhs.push(out);
    }
    Ok(json!({"status": "OK", "n": n, "rhs": rhs}))
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let out = (|| -> Result<Value, String> {
        if args.len() != 3 || args[1] != "--request" {
            return Err("usage: vigilode_bassrei_diffsol --request PATH".into());
        }
        let text = fs::read_to_string(&args[2]).map_err(|e| e.to_string())?;
        let req: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let d = req.get("model").ok_or("missing model")?;
        let kind = Rc::new(kind_of(d).map_err(|e| e.to_string())?);
        if req.get("mode").and_then(Value::as_str) == Some("probe") {
            return probe(&kind, &req);
        }
        let alg = req.get("algorithm").and_then(Value::as_str).unwrap_or("bdf").to_string();
        let rtol = f64_of(&req, "rtol").map_err(|e| e.to_string())?;
        let atol = vec_of(&req, "atol").map_err(|e| e.to_string())?;
        match solve(kind, d, &alg, rtol, atol) {
            Ok(v) => Ok(v),
            Err(e) => Ok(json!({"status": "NUMERICAL_FAILURE", "reason": e})),
        }
    })();
    match out {
        Ok(v) => println!("{v}"),
        Err(e) => {
            println!("{}", json!({"status": "ADAPTER_FAILURE", "reason": e}));
            std::process::exit(2);
        }
    }
}
