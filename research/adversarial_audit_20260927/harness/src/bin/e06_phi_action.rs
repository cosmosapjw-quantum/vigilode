//! E-06: dense_phi_action / matrix_exp_pade13 accuracy probe. Reads cases JSON, prints results JSON.
use rodas5p_core::{DenseMatrix, dense_phi_action, matrix_exp_pade13};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Case {
    name: String,
    n: usize,
    scale: f64,
    a: Vec<Vec<f64>>,
    v: Vec<f64>,
}
#[derive(Deserialize)]
struct Input {
    cases: Vec<Case>,
}
#[derive(Serialize)]
struct Out {
    name: String,
    n: usize,
    scale: f64,
    phi: Vec<Option<Vec<f64>>>,
    phi_errors: Vec<Option<String>>,
    expm_last_col: Option<Vec<f64>>,
    expm_v: Option<Vec<f64>>,
    expm_error: Option<String>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let input: Input = serde_json::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    let mut outs = Vec::new();
    for c in &input.cases {
        let a = DenseMatrix::from_vec_rows(c.a.clone()).unwrap();
        let mut phi = Vec::new();
        let mut errs = Vec::new();
        for k in 0..5 {
            match dense_phi_action(&a, c.scale, k, &c.v) {
                Ok(x) => {
                    phi.push(Some(x));
                    errs.push(None);
                }
                Err(e) => {
                    phi.push(None);
                    errs.push(Some(format!("{e}")));
                }
            }
        }
        let (expm_last_col, expm_v, expm_error) = match matrix_exp_pade13(&a.scale(c.scale)) {
            Ok(e) => {
                let last: Vec<f64> = (0..c.n).map(|i| e[(i, c.n - 1)]).collect();
                (Some(last), Some(e.matvec(&c.v).unwrap()), None)
            }
            Err(e) => (None, None, Some(format!("{e}"))),
        };
        outs.push(Out { name: c.name.clone(), n: c.n, scale: c.scale, phi, phi_errors: errs, expm_last_col, expm_v, expm_error });
    }
    println!("{}", serde_json::to_string(&outs).unwrap());
}
