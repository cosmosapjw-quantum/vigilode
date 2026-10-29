use vigilode_prefix02::BoundModel;
fn bits(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}
fn packed(v: &[f64]) -> String {
    v.iter().map(|x| bits(*x)).collect::<Vec<_>>().join(";")
}
fn main() {
    println!("id,sigma,mu,rho,k,e,jdiag,rhs,x,weights");
    for id in 0..40 {
        let n = 4;
        let mut j = vec![0.0; n * n];
        let mut jd = vec![0.0; n];
        for i in 0..n {
            jd[i] = -2.0 - (i as f64) / 8.0;
            j[i * n + i] = jd[i];
        }
        let weights = (0..n).map(|i| 1.0 + (i as f64) / 10.0).collect::<Vec<_>>();
        let m = BoundModel {
            n,
            jacobian: j,
            weights: weights.clone(),
            sigma: 0.25,
            model_id: "bits".into(),
            source_id: "frozen".into(),
            clock_id: "x".into(),
            norm_id: "weighted".into(),
            epoch: 1,
        };
        let rhs = (0..n)
            .map(|i| 1.0 + ((i + id) % 5) as f64 / 8.0)
            .collect::<Vec<_>>();
        let x = (0..n)
            .map(|i| rhs[i] / (1.0 - 0.25 * jd[i]) + (id as f64 + 1.0) * 1e-10)
            .collect::<Vec<_>>();
        let c = m.certify(&rhs, &x, 1.0, 1.0).unwrap();
        println!(
            "{id},{},{},{},{},{},{},{},{},{}",
            bits(m.sigma),
            bits(m.mu_upper().unwrap()),
            bits(c.residual_upper),
            bits(c.inverse_upper),
            bits(c.forward_upper),
            packed(&jd),
            packed(&rhs),
            packed(&x),
            packed(&weights)
        );
    }
}
