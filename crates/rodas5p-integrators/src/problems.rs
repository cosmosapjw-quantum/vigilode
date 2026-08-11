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
    let jvp = Arc::new(move |_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
        out[0] = lambda * v[0];
        Ok(())
    });
    let exact = Arc::new(move |t: f64| vec![y0 * (lambda * t).exp()]);
    (
        OdeProblem::new(
            format!("scalar-linear-{lambda}"),
            1,
            rhs,
            Some(batch),
            Some(jac),
            Some(jvp),
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
    let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        let d = y[0] - t.sin();
        out[0] = (lambda + 2.0 * mu * d) * v[0];
        Ok(())
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
            Some(jvp),
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
    let jvp = {
        let exact = exact_arc.clone();
        let a = amat.clone();
        Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
            let ph = exact(t);
            a.matvec_into(v, out)?;
            for i in 0..n {
                let d = y[i] - ph[i];
                out[i] += 3.0 * nonlinearity * d * d * v[i];
            }
            Ok(())
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
            Some(jvp),
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
    let jvp_j = j.clone();
    let jvp =
        Arc::new(move |_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| jvp_j.matvec_into(v, out));
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
        Some(jvp),
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
    let jvp_linear = linear.clone();
    let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        let phi = exact_value(t);
        jvp_linear.matvec_into(v, out)?;
        for component in 0..2 {
            let defect = y[component] - phi[component];
            out[component] += 3.0 * nonlinearity * defect * defect * v[component];
        }
        Ok(())
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
        Some(jvp),
        Some(partial_t),
        false,
        Some(mass.clone()),
        Some(exact),
    )?;
    Ok((problem, y0, mass, linear))
}
pub fn complex_dahlquist_problem(
    blocks: usize,
    damping: f64,
    frequency: f64,
    t0: f64,
) -> CoreResult<(OdeProblem, Vec<f64>)> {
    if blocks == 0
        || !damping.is_finite()
        || damping < 0.0
        || !frequency.is_finite()
        || !t0.is_finite()
    {
        return Err(rodas5p_core::CoreError::InvalidInput(
            "invalid complex Dahlquist parameters".into(),
        ));
    }
    let n = 2 * blocks;
    let mut matrix = DenseMatrix::zeros(n, n);
    for block in 0..blocks {
        let i = 2 * block;
        matrix[(i, i)] = -damping;
        matrix[(i, i + 1)] = -frequency;
        matrix[(i + 1, i)] = frequency;
        matrix[(i + 1, i + 1)] = -damping;
    }
    let matrix = Arc::new(matrix);
    let exact: Arc<dyn Fn(f64) -> Vec<f64> + Send + Sync> = Arc::new(move |t: f64| {
        let decay = (-damping * t).exp();
        let mut out = vec![0.0; n];
        for block in 0..blocks {
            let phase = 0.17 * block as f64 + frequency * t;
            out[2 * block] = decay * phase.cos();
            out[2 * block + 1] = decay * phase.sin();
        }
        out
    });
    let rhs_matrix = matrix.clone();
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| rhs_matrix.matvec_into(y, out));
    let batch_matrix = matrix.clone();
    let batch = Arc::new(move |_times: &[f64], states: &[Vec<f64>]| {
        states
            .iter()
            .map(|state| batch_matrix.matvec(state))
            .collect()
    });
    let jacobian_matrix = matrix.clone();
    let jacobian = Arc::new(move |_t: f64, _y: &[f64]| Ok((*jacobian_matrix).clone()));
    let jvp_matrix = matrix.clone();
    let jvp = Arc::new(move |_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
        jvp_matrix.matvec_into(v, out)
    });
    let y0 = exact(t0);
    Ok((
        OdeProblem::new(
            format!("complex-dahlquist-b{blocks}-s{damping}-w{frequency}"),
            n,
            rhs,
            Some(batch),
            Some(jacobian),
            Some(jvp),
            None,
            true,
            None,
            Some(exact),
        )?,
        y0,
    ))
}

pub fn oscillatory_prothero_robinson_problem(
    lambda: f64,
    mu: f64,
    omega: f64,
    t0: f64,
) -> CoreResult<(OdeProblem, Vec<f64>)> {
    if ![lambda, mu, omega, t0]
        .iter()
        .all(|value| value.is_finite())
        || omega <= 0.0
    {
        return Err(rodas5p_core::CoreError::InvalidInput(
            "invalid oscillatory Prothero-Robinson parameters".into(),
        ));
    }
    let g = move |t: f64| (omega * t).sin();
    let gp = move |t: f64| omega * (omega * t).cos();
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let defect = y[0] - g(t);
        out[0] = lambda * defect + gp(t) + mu * defect * defect;
        Ok(())
    });
    let batch = Arc::new(move |times: &[f64], states: &[Vec<f64>]| {
        Ok(times
            .iter()
            .zip(states)
            .map(|(&t, y)| {
                let defect = y[0] - g(t);
                vec![lambda * defect + gp(t) + mu * defect * defect]
            })
            .collect())
    });
    let jacobian = Arc::new(move |t: f64, y: &[f64]| {
        let defect = y[0] - g(t);
        DenseMatrix::new(1, 1, vec![lambda + 2.0 * mu * defect])
    });
    let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        let defect = y[0] - g(t);
        out[0] = (lambda + 2.0 * mu * defect) * v[0];
        Ok(())
    });
    let partial_t = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let gt = g(t);
        let gpt = gp(t);
        let defect = y[0] - gt;
        out[0] = -lambda * gpt - omega * omega * gt - 2.0 * mu * defect * gpt;
        Ok(())
    });
    let exact: Arc<dyn Fn(f64) -> Vec<f64> + Send + Sync> = Arc::new(move |t: f64| vec![g(t)]);
    let y0 = exact(t0);
    Ok((
        OdeProblem::new(
            format!("oscillatory-pr-l{lambda}-m{mu}-w{omega}"),
            1,
            rhs,
            Some(batch),
            Some(jacobian),
            Some(jvp),
            Some(partial_t),
            false,
            None,
            Some(exact),
        )?,
        y0,
    ))
}

pub fn stiff_van_der_pol_problem(mu: f64) -> CoreResult<(OdeProblem, Vec<f64>)> {
    if !(mu.is_finite() && mu > 0.0) {
        return Err(rodas5p_core::CoreError::InvalidInput(
            "van der Pol stiffness must be finite and positive".into(),
        ));
    }
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = y[1];
        out[1] = mu * (1.0 - y[0] * y[0]) * y[1] - y[0];
        Ok(())
    });
    let batch = Arc::new(move |_times: &[f64], states: &[Vec<f64>]| {
        Ok(states
            .iter()
            .map(|y| vec![y[1], mu * (1.0 - y[0] * y[0]) * y[1] - y[0]])
            .collect())
    });
    let jacobian = Arc::new(move |_t: f64, y: &[f64]| {
        DenseMatrix::new(
            2,
            2,
            vec![
                0.0,
                1.0,
                -2.0 * mu * y[0] * y[1] - 1.0,
                mu * (1.0 - y[0] * y[0]),
            ],
        )
    });
    let jvp = Arc::new(move |_t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        out[0] = v[1];
        out[1] = (-2.0 * mu * y[0] * y[1] - 1.0) * v[0] + mu * (1.0 - y[0] * y[0]) * v[1];
        Ok(())
    });
    Ok((
        OdeProblem::new(
            format!("stiff-van-der-pol-mu{mu}"),
            2,
            rhs,
            Some(batch),
            Some(jacobian),
            Some(jvp),
            None,
            true,
            None,
            None,
        )?,
        vec![2.0, 0.0],
    ))
}

pub fn robertson_problem() -> CoreResult<(OdeProblem, Vec<f64>)> {
    let rhs = Arc::new(|_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
        out[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
        out[2] = 3.0e7 * y[1] * y[1];
        Ok(())
    });
    let batch = Arc::new(|_times: &[f64], states: &[Vec<f64>]| {
        Ok(states
            .iter()
            .map(|y| {
                vec![
                    -0.04 * y[0] + 1.0e4 * y[1] * y[2],
                    0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1],
                    3.0e7 * y[1] * y[1],
                ]
            })
            .collect())
    });
    let jacobian = Arc::new(|_t: f64, y: &[f64]| {
        DenseMatrix::new(
            3,
            3,
            vec![
                -0.04,
                1.0e4 * y[2],
                1.0e4 * y[1],
                0.04,
                -1.0e4 * y[2] - 6.0e7 * y[1],
                -1.0e4 * y[1],
                0.0,
                6.0e7 * y[1],
                0.0,
            ],
        )
    });
    let jvp = Arc::new(|_t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        out[0] = -0.04 * v[0] + 1.0e4 * y[2] * v[1] + 1.0e4 * y[1] * v[2];
        out[1] = 0.04 * v[0] + (-1.0e4 * y[2] - 6.0e7 * y[1]) * v[1] - 1.0e4 * y[1] * v[2];
        out[2] = 6.0e7 * y[1] * v[1];
        Ok(())
    });
    Ok((
        OdeProblem::new(
            "robertson",
            3,
            rhs,
            Some(batch),
            Some(jacobian),
            Some(jvp),
            None,
            true,
            None,
            None,
        )?,
        vec![1.0, 0.0, 0.0],
    ))
}

pub fn semilinear_advection_diffusion_problem(
    n: usize,
    diffusion: f64,
    advection: f64,
    reaction: f64,
    nonlinearity: f64,
    t0: f64,
) -> CoreResult<(OdeProblem, Vec<f64>)> {
    if n < 2
        || ![diffusion, advection, reaction, nonlinearity, t0]
            .iter()
            .all(|value| value.is_finite())
        || diffusion < 0.0
    {
        return Err(rodas5p_core::CoreError::InvalidInput(
            "invalid semilinear advection-diffusion parameters".into(),
        ));
    }
    let dx = 1.0 / (n + 1) as f64;
    let mut operator = DenseMatrix::zeros(n, n);
    for i in 0..n {
        operator[(i, i)] = -2.0 * diffusion / (dx * dx) + reaction - advection / dx;
        if i > 0 {
            operator[(i, i - 1)] = diffusion / (dx * dx) + advection / dx;
        }
        if i + 1 < n {
            operator[(i, i + 1)] = diffusion / (dx * dx);
        }
    }
    let shape = (1..=n)
        .map(|index| (std::f64::consts::PI * index as f64 * dx).sin())
        .collect::<Vec<_>>();
    let exact: Arc<dyn Fn(f64) -> Vec<f64> + Send + Sync> = {
        let shape = shape.clone();
        Arc::new(move |t: f64| shape.iter().map(|value| (-t).exp() * value).collect())
    };
    let derivative: Arc<dyn Fn(f64) -> Vec<f64> + Send + Sync> = {
        let exact = exact.clone();
        Arc::new(move |t: f64| exact(t).into_iter().map(|value| -value).collect())
    };
    let operator = Arc::new(operator);
    let rhs_operator = operator.clone();
    let rhs_exact = exact.clone();
    let rhs_derivative = derivative.clone();
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let phi = rhs_exact(t);
        let dphi = rhs_derivative(t);
        let defect = y.iter().zip(&phi).map(|(a, b)| a - b).collect::<Vec<_>>();
        rhs_operator.matvec_into(&defect, out)?;
        for i in 0..n {
            out[i] += dphi[i] + nonlinearity * defect[i].powi(3);
        }
        Ok(())
    });
    let batch_operator = operator.clone();
    let batch_exact = exact.clone();
    let batch_derivative = derivative.clone();
    let batch = Arc::new(move |times: &[f64], states: &[Vec<f64>]| {
        times
            .iter()
            .zip(states)
            .map(|(&t, y)| {
                let phi = batch_exact(t);
                let dphi = batch_derivative(t);
                let defect = y.iter().zip(&phi).map(|(a, b)| a - b).collect::<Vec<_>>();
                let mut out = batch_operator.matvec(&defect)?;
                for i in 0..n {
                    out[i] += dphi[i] + nonlinearity * defect[i].powi(3);
                }
                Ok(out)
            })
            .collect()
    });
    let jac_operator = operator.clone();
    let jac_exact = exact.clone();
    let jacobian = Arc::new(move |t: f64, y: &[f64]| {
        let phi = jac_exact(t);
        let mut jac = (*jac_operator).clone();
        for i in 0..n {
            let defect = y[i] - phi[i];
            jac[(i, i)] += 3.0 * nonlinearity * defect * defect;
        }
        Ok(jac)
    });
    let jvp_operator = operator.clone();
    let jvp_exact = exact.clone();
    let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        let phi = jvp_exact(t);
        jvp_operator.matvec_into(v, out)?;
        for i in 0..n {
            let defect = y[i] - phi[i];
            out[i] += 3.0 * nonlinearity * defect * defect * v[i];
        }
        Ok(())
    });
    let ft_operator = operator.clone();
    let ft_exact = exact.clone();
    let partial_t = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let phi = ft_exact(t);
        let dphi = phi.iter().map(|value| -value).collect::<Vec<_>>();
        let ddphi = phi.clone();
        ft_operator.matvec_into(&dphi, out)?;
        for i in 0..n {
            let defect = y[i] - phi[i];
            out[i] = -out[i] + ddphi[i] - 3.0 * nonlinearity * defect * defect * dphi[i];
        }
        Ok(())
    });
    let y0 = exact(t0);
    Ok((
        OdeProblem::new(
            format!("advection-diffusion-n{n}-d{diffusion}-a{advection}-r{reaction}"),
            n,
            rhs,
            Some(batch),
            Some(jacobian),
            Some(jvp),
            Some(partial_t),
            false,
            None,
            Some(exact),
        )?,
        y0,
    ))
}
