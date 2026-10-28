//! E-07 (4): print the f64 ROW-form RODAS5P coefficients exactly as the Rust crate computes them.
//! Every value is printed with 17 significant digits and as raw IEEE-754 bits (hex).
use rodas5p_core::{DenseMatrix, load_rodas5p_coefficients};

fn v(x: f64) -> serde_json::Value {
    serde_json::json!({"dec": format!("{:.16e}", x), "bits": format!("{:016x}", x.to_bits())})
}
fn vec(xs: &[f64]) -> serde_json::Value {
    serde_json::Value::Array(xs.iter().map(|x| v(*x)).collect())
}
fn mat(m: &DenseMatrix) -> serde_json::Value {
    serde_json::Value::Array((0..m.nrows()).map(|i| vec(m.row(i))).collect())
}

fn main() {
    let c = load_rodas5p_coefficients().expect("coefficients");
    let out = serde_json::json!({
        "schema": c.snapshot_schema_version,
        "stages": c.stages(),
        "gamma": v(c.gamma),
        "c": vec(&c.c),
        "b_code": vec(&c.b_code),
        "a_input": mat(&c.a),
        "c_matrix_input": mat(&c.c_matrix),
        "gamma_matrix": mat(&c.gamma_matrix),
        "alpha": mat(&c.alpha),
        "beta": mat(&c.beta),
        "l": mat(&c.l),
        "b": vec(&c.b),
        "btilde": vec(&c.btilde),
        "gamma_rows": vec(&c.gamma_rows),
        "dense_h": mat(&c.dense_h),
        "dense_d": mat(&c.dense_d),
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
