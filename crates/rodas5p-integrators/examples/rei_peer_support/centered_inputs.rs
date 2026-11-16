//! Same real Peer step expressed around the last physical state.
//! Exact tableau identities B*1=1 and Gamma*1=-gamma*1 cancel anchors
//! algebraically before floating arithmetic; imported coefficients stay unchanged.
use super::peer;
use rodas5p_core::CoreResult;
fn sum(it: impl Iterator<Item = f64>) -> f64 {
    let (mut s, mut c) = (0.0, 0.0);
    for x in it {
        let y = x - c;
        let t = s + y;
        c = (t - s) - y;
        s = t;
    }
    s
}
pub fn build(state: &peer::State, h: f64) -> CoreResult<[peer::StageInput; 6]> {
    let mut out = peer::stage_inputs(state, h)?; // inherited guards and source clocks
    let tab = peer::tableau(h / state.h_prev)?;
    for (i, row) in out.iter_mut().enumerate() {
        for k in 0..state.y[5].len() {
            let anchor = state.y[5][k];
            row.explicit[k] = sum((0..6).map(|j| {
                tab.b_matrix[i][j] * (state.y[j][k] - anchor)
                    + h * tab.a_matrix[i][j] * state.f[j][k]
            }));
            row.transfer[k] =
                sum((0..6).map(|j| tab.gamma_matrix[i][j] * (state.y[j][k] - anchor)));
        }
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exactly_constant_history_has_zero_increment_inputs() {
        let s = peer::State {
            t: 0.25,
            h_prev: 0.125,
            y: std::array::from_fn(|_| vec![9.75, 0.003141592653589793]),
            f: std::array::from_fn(|_| vec![0.0, 0.0]),
        };
        for row in build(&s, 0.125).unwrap() {
            assert_eq!(
                row.explicit,
                vec![0.0, 0.0],
                "constant-state forcing invented"
            );
            assert_eq!(
                row.transfer,
                vec![0.0, 0.0],
                "constant-state Jacobian forcing invented"
            );
        }
    }
    #[test]
    fn source_tableau_constant_identities_close_at_roundoff() {
        for sigma in [0.5, 1.0, 1.1] {
            let t = peer::tableau(sigma).unwrap();
            for i in 0..6 {
                let b: f64 = t.b_matrix[i].iter().sum();
                let g: f64 = t.gamma_matrix[i].iter().sum();
                assert!((b - 1.0).abs() < 1e-9);
                assert!((g + peer::gamma()).abs() < 1e-9);
            }
        }
    }
}
