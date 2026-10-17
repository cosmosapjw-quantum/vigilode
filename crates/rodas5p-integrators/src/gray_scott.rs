use std::sync::Arc;

use rodas5p_core::{CoreError, CoreResult};

use crate::problem::{OdeProblem, SparseLinearPart};

/// Build Gray-Scott 2D problem on periodic unit square with dimension $n = 2 N^2$.
///
/// Equations:
/// $u_t = D_u \Delta u - u v^2 + F (1 - u)$
/// $v_t = D_v \Delta v + u v^2 - (F + k) v$
///
/// Parameters:
/// $D_u = 2\times 10^{-3}$, $D_v = 10^{-3}$, $F = 0.04$, $k = 0.06$, on $t \in [0, 20]$.
pub fn gray_scott_2d(n_grid: usize) -> CoreResult<(OdeProblem, Vec<f64>)> {
    if n_grid < 2 {
        return Err(CoreError::Dimension(
            "gray-scott-2d requires grid size >= 2".into(),
        ));
    }
    let n2 = n_grid * n_grid;
    let n = 2 * n2;
    let dx = 1.0 / (n_grid as f64);
    let inv_dx2 = 1.0 / (dx * dx);

    const D_U: f64 = 2.0e-3;
    const D_V: f64 = 1.0e-3;
    const F: f64 = 0.04;
    const K: f64 = 0.06;

    // Build initial condition
    let mut y0 = vec![0.0f64; n];
    for j in 0..n_grid {
        let y_coord = (j as f64 + 0.5) * dx;
        let dy = y_coord - 0.5;
        for i in 0..n_grid {
            let x_coord = (i as f64 + 0.5) * dx;
            let dx_c = x_coord - 0.5;
            let dist_sq = dx_c * dx_c + dy * dy;
            let bump = 0.25 * (-100.0 * dist_sq).exp();
            let idx = i + j * n_grid;
            y0[idx] = 1.0 - bump; // u
            y0[n2 + idx] = bump; // v
        }
    }

    // Helper to evaluate 5-point Laplacian on periodic grid
    let laplacian = move |vals: &[f64], out: &mut [f64]| {
        for j in 0..n_grid {
            let j_up = if j + 1 < n_grid { j + 1 } else { 0 };
            let j_down = if j > 0 { j - 1 } else { n_grid - 1 };
            for i in 0..n_grid {
                let i_right = if i + 1 < n_grid { i + 1 } else { 0 };
                let i_left = if i > 0 { i - 1 } else { n_grid - 1 };
                let idx = i + j * n_grid;
                let c = vals[idx];
                let l = vals[i_left + j * n_grid];
                let r = vals[i_right + j * n_grid];
                let u = vals[i + j_up * n_grid];
                let d = vals[i + j_down * n_grid];
                out[idx] = (l + r + u + d - 4.0 * c) * inv_dx2;
            }
        }
    };

    // Full RHS callback
    let rhs_lap = laplacian;
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        let (u, v) = y.split_at(n2);
        let (out_u, out_v) = out.split_at_mut(n2);

        let mut lap_u = vec![0.0f64; n2];
        let mut lap_v = vec![0.0f64; n2];
        rhs_lap(u, &mut lap_u);
        rhs_lap(v, &mut lap_v);

        for i in 0..n2 {
            let ui = u[i];
            let vi = v[i];
            let uv2 = ui * vi * vi;
            out_u[i] = D_U * lap_u[i] - uv2 + F * (1.0 - ui);
            out_v[i] = D_V * lap_v[i] + uv2 - (F + K) * vi;
        }
        Ok(())
    });

    // JVP callback
    let jvp_lap = laplacian;
    let jvp = Arc::new(move |_t: f64, y: &[f64], delta: &[f64], out: &mut [f64]| {
        let (u, v) = y.split_at(n2);
        let (du, dv) = delta.split_at(n2);
        let (out_u, out_v) = out.split_at_mut(n2);

        let mut lap_du = vec![0.0f64; n2];
        let mut lap_dv = vec![0.0f64; n2];
        jvp_lap(du, &mut lap_du);
        jvp_lap(dv, &mut lap_dv);

        for i in 0..n2 {
            let ui = u[i];
            let vi = v[i];
            let dui = du[i];
            let dvi = dv[i];
            // d/d(u,v) [-u v^2 + F(1-u)] * (du, dv) = -v^2 du - 2 u v dv - F du
            out_u[i] = D_U * lap_du[i] - vi * vi * dui - 2.0 * ui * vi * dvi - F * dui;
            // d/d(u,v) [u v^2 - (F+k)v] * (du, dv) = v^2 du + 2 u v dv - (F+k) dv
            out_v[i] = D_V * lap_dv[i] + vi * vi * dui + 2.0 * ui * vi * dvi - (F + K) * dvi;
        }
        Ok(())
    });

    // Build sparse linear part A = blockdiag(D_u Lap - F I, D_v Lap - (F + k) I)
    let mut col_ptr = Vec::with_capacity(n + 1);
    let mut row_idx = Vec::with_capacity(10 * n);
    let mut values = Vec::with_capacity(10 * n);

    col_ptr.push(0);

    // Block 1: u block (columns 0..n2)
    for j in 0..n_grid {
        let j_up = if j + 1 < n_grid { j + 1 } else { 0 };
        let j_down = if j > 0 { j - 1 } else { n_grid - 1 };
        for i in 0..n_grid {
            let i_right = if i + 1 < n_grid { i + 1 } else { 0 };
            let i_left = if i > 0 { i - 1 } else { n_grid - 1 };
            let col = i + j * n_grid;

            // Collect nonzeros in column col for u block
            // Entries in row idx:
            // diagonal: col
            // left: i_left + j * n_grid
            // right: i_right + j * n_grid
            // down: i + j_down * n_grid
            // up: i + j_up * n_grid
            let mut entries: Vec<(usize, f64)> = Vec::new();
            let diag_val = D_U * (-4.0 * inv_dx2) - F;
            let off_val = D_U * inv_dx2;

            entries.push((col, diag_val));
            // Coincident periodic neighbors (N=2) contribute twice.
            entries.push((i_left + j * n_grid, off_val));
            entries.push((i_right + j * n_grid, off_val));
            entries.push((i + j_down * n_grid, off_val));
            entries.push((i + j_up * n_grid, off_val));

            entries.sort_by_key(|&(r, _)| r);
            let column_start = row_idx.len();
            for (r, v) in entries {
                if row_idx.len() > column_start && row_idx.last() == Some(&r) {
                    *values.last_mut().expect("entry in current column") += v;
                } else {
                    row_idx.push(r);
                    values.push(v);
                }
            }
            col_ptr.push(row_idx.len());
        }
    }

    // Block 2: v block (columns n2..n)
    for j in 0..n_grid {
        let j_up = if j + 1 < n_grid { j + 1 } else { 0 };
        let j_down = if j > 0 { j - 1 } else { n_grid - 1 };
        for i in 0..n_grid {
            let i_right = if i + 1 < n_grid { i + 1 } else { 0 };
            let i_left = if i > 0 { i - 1 } else { n_grid - 1 };
            let col = i + j * n_grid;

            let mut entries: Vec<(usize, f64)> = Vec::new();
            let diag_val = D_V * (-4.0 * inv_dx2) - (F + K);
            let off_val = D_V * inv_dx2;

            entries.push((n2 + col, diag_val));
            // Coincident periodic neighbors (N=2) contribute twice.
            entries.push((n2 + i_left + j * n_grid, off_val));
            entries.push((n2 + i_right + j * n_grid, off_val));
            entries.push((n2 + i + j_down * n_grid, off_val));
            entries.push((n2 + i + j_up * n_grid, off_val));

            entries.sort_by_key(|&(r, _)| r);
            let column_start = row_idx.len();
            for (r, v) in entries {
                if row_idx.len() > column_start && row_idx.last() == Some(&r) {
                    *values.last_mut().expect("entry in current column") += v;
                } else {
                    row_idx.push(r);
                    values.push(v);
                }
            }
            col_ptr.push(row_idx.len());
        }
    }

    let sparse_a = SparseLinearPart::new(n, n, col_ptr, row_idx, values)?;

    // Nonlinear callback N(t, y) = [-u v^2 + F, u v^2]^T
    let nonlinear_n = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        let (u, v) = y.split_at(n2);
        let (out_u, out_v) = out.split_at_mut(n2);
        for i in 0..n2 {
            let uv2 = u[i] * v[i] * v[i];
            out_u[i] = -uv2 + F;
            out_v[i] = uv2;
        }
        Ok(())
    });

    let problem = OdeProblem::new(
        format!("gray-scott-2d-n{n_grid}"),
        n,
        rhs,
        None,
        None,
        Some(jvp),
        None,
        true,
        None,
        None,
    )?
    .with_linear_part(sparse_a, nonlinear_n)?;

    Ok((problem, y0))
}
