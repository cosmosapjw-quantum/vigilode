//! Cell-local 4x4 plus three-global-moment Jacobian application.
pub fn apply(blocks: &[[[f64; 4]; 4]], u: &[[f64; 3]], v: &[[f64; 3]], x: &[f64]) -> Vec<f64> {
    assert_eq!(x.len(), 4 * blocks.len());
    assert_eq!(u.len(), x.len());
    assert_eq!(v.len(), x.len());
    let mut moments = [0.0; 3];
    let mut corr = [0.0; 3];
    for j in 0..x.len() {
        for k in 0..3 {
            let z = v[j][k] * x[j] - corr[k];
            let t = moments[k] + z;
            corr[k] = (t - moments[k]) - z;
            moments[k] = t;
        }
    }
    let mut out = vec![0.0; x.len()];
    for (i, b) in blocks.iter().enumerate() {
        for r in 0..4 {
            out[4 * i + r] = (0..4).map(|c| b[r][c] * x[4 * i + c]).sum::<f64>()
                + (0..3).map(|k| u[4 * i + r][k] * moments[k]).sum::<f64>();
        }
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_plus_global_jacobian_has_real_offcell_response() {
        let mut d = vec![[[0.0; 4]; 4]; 2];
        for b in &mut d {
            for j in 0..4 {
                b[j][j] = -2.0;
            }
        }
        let mut u = vec![[0.0; 3]; 8];
        let mut v = vec![[0.0; 3]; 8];
        u[0][0] = 3.0;
        u[4][0] = 2.0;
        v[1][0] = 0.25;
        v[5][0] = 0.75;
        let x = vec![1.0, 2.0, 0.0, 0.0, 3.0, 4.0, 0.0, 0.0];
        let y = apply(&d, &u, &v, &x);
        assert_eq!(y, vec![8.5, -4.0, 0.0, 0.0, 1.0, -8.0, 0.0, 0.0]);
    }
}
