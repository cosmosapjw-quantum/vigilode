use crate::OdeProblem;
use rodas5p_core::{CoreResult, DenseMatrix};
use std::sync::Arc;

pub fn scalar_linear_problem(lambda: f64, y0: f64) -> (OdeProblem, Vec<f64>) {
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = lambda * y[0];
        Ok(())
    });
    let batch = Arc::new(move |_t: &[f64], states: &[Vec<f64>]| {
        Ok(states.iter().map(|y| vec![lambda * y[0]]).collect())
    });
    let jac = Arc::new(move |_t: f64, _y: &[f64]| DenseMatrix::new(1, 1, vec![lambda]));
    let exact = Arc::new(move |t: f64| vec![y0 * (lambda * t).exp()]);
    (
        OdeProblem::new(
            format!("scalar-linear-{lambda}"),
            1,
            rhs,
            Some(batch),
            Some(jac),
            None,
            None,
            true,
            None,
            Some(exact),
        )
        .unwrap(),
        vec![y0],
    )
}

pub fn prothero_robinson_problem(lambda: f64, mu: f64, t0: f64) -> (OdeProblem, Vec<f64>) {
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let d = y[0] - t.sin();
        out[0] = lambda * d + t.cos() + mu * d * d;
        Ok(())
    });
    let batch = Arc::new(move |times: &[f64], states: &[Vec<f64>]| {
        Ok(times
            .iter()
            .zip(states)
            .map(|(&t, y)| {
                let d = y[0] - t.sin();
                vec![lambda * d + t.cos() + mu * d * d]
            })
            .collect())
    });
    let jac = Arc::new(move |t: f64, y: &[f64]| {
        let d = y[0] - t.sin();
        DenseMatrix::new(1, 1, vec![lambda + 2.0 * mu * d])
    });
    let ft = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let d = y[0] - t.sin();
        out[0] = -lambda * t.cos() - t.sin() - 2.0 * mu * d * t.cos();
        Ok(())
    });
    let exact = Arc::new(move |t: f64| vec![t.sin()]);
    (
        OdeProblem::new(
            format!("PR-l{lambda}-m{mu}"),
            1,
            rhs,
            Some(batch),
            Some(jac),
            None,
            Some(ft),
            false,
            None,
            Some(exact),
        )
        .unwrap(),
        vec![t0.sin()],
    )
}

pub fn manufactured_vector_problem(
    n: usize,
    stiffness: f64,
    nonlinearity: f64,
    nonnormality: f64,
    t0: f64,
) -> CoreResult<(OdeProblem, Vec<f64>)> {
    if n < 2 {
        return Err(rodas5p_core::CoreError::InvalidInput(
            "n>=2 required".into(),
        ));
    }
    let mut a_matrix = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a_matrix[(i, i)] = -0.5 * stiffness;
        if i > 0 {
            a_matrix[(i, i - 1)] = 0.25 * stiffness;
        }
        if i + 1 < n {
            a_matrix[(i, i + 1)] = 0.25 * stiffness + nonnormality * stiffness;
        }
    }
    let x: Vec<f64> = (1..=n).map(|i| i as f64 / (n + 1) as f64).collect();
    let av: Vec<f64> = x.iter().map(|z| (std::f64::consts::PI * z).sin()).collect();
    let bv: Vec<f64> = x
        .iter()
        .map(|z| 0.35 * (2.0 * std::f64::consts::PI * z).sin())
        .collect();
    let exact_vec = {
        let av = av.clone();
        let bv = bv.clone();
        move |t: f64| {
            av.iter()
                .zip(&bv)
                .map(|(a, b)| a * t.sin() + b * (0.5 * t).cos())
                .collect::<Vec<_>>()
        }
    };
    let d1_vec = {
        let av = av.clone();
        let bv = bv.clone();
        move |t: f64| {
            av.iter()
                .zip(&bv)
                .map(|(a, b)| a * t.cos() - 0.5 * b * (0.5 * t).sin())
                .collect::<Vec<_>>()
        }
    };
    let d2_vec = {
        let av = av.clone();
        let bv = bv.clone();
        move |t: f64| {
            av.iter()
                .zip(&bv)
                .map(|(a, b)| -a * t.sin() - 0.25 * b * (0.5 * t).cos())
                .collect::<Vec<_>>()
        }
    };
    let exact_arc: Arc<dyn Fn(f64) -> Vec<f64> + Send + Sync> = Arc::new(exact_vec);
    let d1_arc: Arc<dyn Fn(f64) -> Vec<f64> + Send + Sync> = Arc::new(d1_vec);
    let d2_arc: Arc<dyn Fn(f64) -> Vec<f64> + Send + Sync> = Arc::new(d2_vec);
    let amat = Arc::new(a_matrix);
    let rhs = {
        let exact = exact_arc.clone();
        let d1 = d1_arc.clone();
        let a = amat.clone();
        Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
            let ph = exact(t);
            let v = d1(t);
            let d: Vec<f64> = y.iter().zip(&ph).map(|(x, p)| x - p).collect();
            a.matvec_into(&d, out)?;
            for i in 0..out.len() {
                out[i] += v[i] + nonlinearity * d[i].powi(3);
            }
            Ok(())
        })
    };
    let batch = {
        let exact = exact_arc.clone();
        let d1 = d1_arc.clone();
        let a = amat.clone();
        Arc::new(move |times: &[f64], states: &[Vec<f64>]| {
            times
                .iter()
                .zip(states)
                .map(|(&t, y)| {
                    let ph = exact(t);
                    let v = d1(t);
                    let d: Vec<f64> = y.iter().zip(&ph).map(|(x, p)| x - p).collect();
                    let mut out = a.matvec(&d)?;
                    for i in 0..n {
                        out[i] += v[i] + nonlinearity * d[i].powi(3);
                    }
                    Ok(out)
                })
                .collect()
        })
    };
    let jac = {
        let exact = exact_arc.clone();
        let a = amat.clone();
        Arc::new(move |t: f64, y: &[f64]| {
            let ph = exact(t);
            let mut j = (*a).clone();
            for i in 0..n {
                let d = y[i] - ph[i];
                j[(i, i)] += 3.0 * nonlinearity * d * d;
            }
            Ok(j)
        })
    };
    let ft = {
        let exact = exact_arc.clone();
        let d1 = d1_arc.clone();
        let d2 = d2_arc.clone();
        let a = amat.clone();
        Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
            let ph = exact(t);
            let v = d1(t);
            let acc = d2(t);
            a.matvec_into(&v, out)?;
            for i in 0..n {
                let d = y[i] - ph[i];
                out[i] = -out[i] + acc[i] - 3.0 * nonlinearity * d * d * v[i];
            }
            Ok(())
        })
    };
    let y0 = exact_arc(t0);
    Ok((
        OdeProblem::new(
            format!("mv-n{n}-s{stiffness}-m{nonlinearity}-eta{nonnormality}"),
            n,
            rhs,
            Some(batch),
            Some(jac),
            None,
            Some(ft),
            false,
            None,
            Some(exact_arc),
        )?,
        y0,
    ))
}

pub fn constant_affine_mass_problem() -> (OdeProblem, Vec<f64>, DenseMatrix, DenseMatrix) {
    let m = DenseMatrix::from_rows(&[&[2.0, 1.0], &[0.0, 3.0]]).unwrap();
    let j = DenseMatrix::from_rows(&[&[-4.0, 1.0], &[2.0, -5.0]]).unwrap();
    let r0 = [0.5, -2.0 / 3.0];
    let r1 = [3.0 / 7.0, 5.0 / 11.0];
    let r2 = [-1.0 / 13.0, 2.0 / 17.0];
    let forcing = move |t: f64| {
        [
            r0[0] + t * r1[0] + t * t * r2[0],
            r0[1] + t * r1[1] + t * t * r2[1],
        ]
    };
    let rhs_j = j.clone();
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        rhs_j.matvec_into(y, out)?;
        let r = forcing(t);
        out[0] += r[0];
        out[1] += r[1];
        Ok(())
    });
    let batch_j = j.clone();
    let batch = Arc::new(move |times: &[f64], states: &[Vec<f64>]| {
        times
            .iter()
            .zip(states)
            .map(|(&t, y)| {
                let mut out = batch_j.matvec(y)?;
                let r = forcing(t);
                out[0] += r[0];
                out[1] += r[1];
                Ok(out)
            })
            .collect()
    });
    let jac_j = j.clone();
    let jac = Arc::new(move |_t: f64, _y: &[f64]| Ok(jac_j.clone()));
    let ft = Arc::new(move |t: f64, _y: &[f64], out: &mut [f64]| {
        out[0] = r1[0] + 2.0 * t * r2[0];
        out[1] = r1[1] + 2.0 * t * r2[1];
        Ok(())
    });
    let p = OdeProblem::new(
        "affine-noncommuting-mass",
        2,
        rhs,
        Some(batch),
        Some(jac),
        None,
        Some(ft),
        false,
        Some(m.clone()),
        None,
    )
    .unwrap();
    (p, vec![2.0 / 5.0, -1.0 / 3.0], m, j)
}

pub fn manufactured_mass_nonlinear_problem(
    stiffness: f64,
    nonlinearity: f64,
    nonnormality: f64,
    t0: f64,
) -> CoreResult<(OdeProblem, Vec<f64>, DenseMatrix, DenseMatrix)> {
    if !stiffness.is_finite()
        || !nonlinearity.is_finite()
        || !nonnormality.is_finite()
        || stiffness <= 0.0
        || nonlinearity < 0.0
    {
        return Err(rodas5p_core::CoreError::InvalidInput(
            "invalid manufactured mass-problem parameters".into(),
        ));
    }

    let mass = DenseMatrix::from_rows(&[&[2.0, 1.0], &[0.3, 1.5]])?;
    let linear = DenseMatrix::from_rows(&[
        &[-0.6 * stiffness, (0.2 + nonnormality) * stiffness],
        &[0.15 * stiffness, -0.4 * stiffness],
    ])?;

    let exact_value = |t: f64| {
        vec![
            t.sin() + 0.2 * (0.5 * t).cos(),
            0.5 * t.cos() - 0.1 * (0.5 * t).sin(),
        ]
    };
    let exact_first = |t: f64| {
        vec![
            t.cos() - 0.1 * (0.5 * t).sin(),
            -0.5 * t.sin() - 0.05 * (0.5 * t).cos(),
        ]
    };
    let exact_second = |t: f64| {
        vec![
            -t.sin() - 0.05 * (0.5 * t).cos(),
            -0.5 * t.cos() + 0.025 * (0.5 * t).sin(),
        ]
    };

    let rhs_mass = mass.clone();
    let rhs_linear = linear.clone();
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let phi = exact_value(t);
        let velocity = exact_first(t);
        let defect: Vec<f64> = y.iter().zip(&phi).map(|(a, b)| a - b).collect();
        rhs_linear.matvec_into(&defect, out)?;
        let mass_velocity = rhs_mass.matvec(&velocity)?;
        for component in 0..2 {
            out[component] += mass_velocity[component] + nonlinearity * defect[component].powi(3);
        }
        Ok(())
    });

    let batch_mass = mass.clone();
    let batch_linear = linear.clone();
    let batch = Arc::new(move |times: &[f64], states: &[Vec<f64>]| {
        times
            .iter()
            .zip(states)
            .map(|(&t, y)| {
                let phi = exact_value(t);
                let velocity = exact_first(t);
                let defect: Vec<f64> = y.iter().zip(&phi).map(|(a, b)| a - b).collect();
                let mut out = batch_linear.matvec(&defect)?;
                let mass_velocity = batch_mass.matvec(&velocity)?;
                for component in 0..2 {
                    out[component] +=
                        mass_velocity[component] + nonlinearity * defect[component].powi(3);
                }
                Ok(out)
            })
            .collect()
    });

    let jac_linear = linear.clone();
    let jacobian = Arc::new(move |t: f64, y: &[f64]| {
        let phi = exact_value(t);
        let mut out = jac_linear.clone();
        for component in 0..2 {
            let defect = y[component] - phi[component];
            out[(component, component)] += 3.0 * nonlinearity * defect * defect;
        }
        Ok(out)
    });

    let ft_mass = mass.clone();
    let ft_linear = linear.clone();
    let partial_t = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let phi = exact_value(t);
        let velocity = exact_first(t);
        let acceleration = exact_second(t);
        let linear_velocity = ft_linear.matvec(&velocity)?;
        let mass_acceleration = ft_mass.matvec(&acceleration)?;
        for component in 0..2 {
            let defect = y[component] - phi[component];
            out[component] = -linear_velocity[component] + mass_acceleration[component]
                - 3.0 * nonlinearity * defect * defect * velocity[component];
        }
        Ok(())
    });

    let exact = Arc::new(exact_value);
    let y0 = exact(t0);
    let problem = OdeProblem::new(
        format!("manufactured-mass-s{stiffness}-m{nonlinearity}-eta{nonnormality}"),
        2,
        rhs,
        Some(batch),
        Some(jacobian),
        None,
        Some(partial_t),
        false,
        Some(mass.clone()),
        Some(exact),
    )?;
    Ok((problem, y0, mass, linear))
}
