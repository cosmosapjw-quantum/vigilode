// Exact legacy function from f2f20c1, with only a minimal matrix adapter.
struct DenseMatrix { n: usize, data: Vec<f64> }
impl DenseMatrix {
 fn nrows(&self)->usize {self.n} fn ncols(&self)->usize {self.n}
 fn as_slice(&self)->&[f64] {&self.data}
 fn row(&self,i:usize)->&[f64] {&self.data[i*self.n..(i+1)*self.n]}
}
pub fn rodas5p_staleness_bound(
    h: f64,
    h_p: f64,
    gamma: f64,
    j: &DenseMatrix,
    j_p: &DenseMatrix,
    s: &[f64],
) -> f64 {
    let n = s.len();
    let shapes = [j.nrows(), j.ncols(), j_p.nrows(), j_p.ncols()];
    if !(h > 0.0 && h_p > 0.0 && gamma > 0.0 && h.is_finite() && h_p.is_finite())
        || shapes.iter().any(|d| *d != n)
        || !s.iter().all(|v| *v > 0.0 && v.is_finite())
        || !j
            .as_slice()
            .iter()
            .chain(j_p.as_slice())
            .all(|v| v.is_finite())
    {
        return f64::INFINITY;
    }
    let a = h_p * gamma;
    let mut mu = f64::NEG_INFINITY;
    let mut drift = 0.0_f64;
    for i in 0..n {
        let (row, row_p) = (j.row(i), j_p.row(i));
        let mut row_mu = row_p[i];
        let mut row_drift = 0.0;
        for k in 0..n {
            let ratio = s[k] / s[i];
            row_drift += (row[k] - row_p[k]).abs() * ratio;
            if k != i {
                row_mu += row_p[k].abs() * ratio;
            }
        }
        mu = mu.max(row_mu);
        drift = drift.max(row_drift);
    }
    let denominator = 1.0 - a * mu;
    if denominator.is_nan() || denominator <= 0.0 {
        return f64::INFINITY;
    }
    let q = ((1.0 - h_p / h).abs() + a * drift) / denominator;
    if q.is_finite() { q } else { f64::INFINITY }
}

fn main() {
 let jp = DenseMatrix { n:3, data:vec![-1.,0.,0.,0.,-1.,0.,0.,0.,-1.] };
 let j = DenseMatrix { n:3, data:vec![-1.,0.,1.,0.,-1.,0.,0.,0.,-1.] };
 let q=rodas5p_staleness_bound(1.,1.,1.,&j,&jp,&[f64::MIN_POSITIVE,f64::MAX,f64::MIN_POSITIVE]);
 println!("legacy_q={q}; exact_T_weighted_norm=0.5; unsound_underestimate={}",q<0.5);
 assert_eq!(q,0.0);
}
