//! N4A experimental bridge. Encloses the EXACT represented-source first-Picard
//! carrier, including all two-cluster cubic polynomial coefficients and kernels.
//! Never calls an arbitrary OdeProblem and never admits a production step.
use crate::n3h_source_bounded_copy::C;
use crate::{I, Z, expneg_product, fact, i, j_list, pow};

#[derive(Clone, Debug)]
pub struct CertifiedComponent {
    pub exact_first_picard: Z,
    /// Interval bound on |computed binary64 re - exact real re| + |computed im - exact im|.
    pub float_gap_l1_upper: f64,
    /// Conditional physical analytic error metadata + floating carrier gap.
    /// This must be used ONLY at source-bound initial epoch zero.
    pub initial_epoch_physical_l1_upper: f64,
}

pub(crate) struct FrozenView<'a> {
    pub(crate) n: usize,
    pub(crate) weights: &'a [f64],
    pub(crate) phi: &'a [Vec<f64>; 2],
    pub(crate) xi: &'a [f64],
    pub(crate) kappa: &'a [f64],
    pub(crate) omega: f64,
    pub(crate) chi: &'a [f64; 2],
    pub(crate) y: &'a [C],
}
fn zp(re: f64, im: f64) -> Result<Z, String> {
    Ok(Z {
        re: i(re)?,
        im: i(im)?,
    })
}
fn zscale(z: Z, r: I) -> Result<Z, String> {
    Ok(Z {
        re: z.re.mul(r)?,
        im: z.im.mul(r)?,
    })
}
fn zconj(z: Z) -> Z {
    Z {
        re: z.re,
        im: z.im.neg(),
    }
}
fn polynomial_convolution(a: &[Z], b: &[Z]) -> Result<Vec<Z>, String> {
    let mut out = vec![Z::zero(); a.len() + b.len() - 1];
    for (j, x) in a.iter().enumerate() {
        for (k, y) in b.iter().enumerate() {
            out[j + k] = out[j + k].add((*x).mul(*y)?)?;
        }
    }
    Ok(out)
}
fn exp_phase(theta: f64, h: f64) -> Result<Z, String> {
    let angle = i(theta)?.mul(i(h)?)?;
    if angle.mag() > 0.25 {
        return Err("phase outside N4 source box".into());
    }
    let z = Z {
        re: I::zero(),
        im: angle,
    };
    let mut sum = Z::one();
    let mut term = Z::one();
    for k in 1..=18 {
        term = term.mul(z)?.div_positive(i(k as f64)?)?;
        sum = sum.add(term)?;
    }
    let tail = pow(I::new(0., angle.mag())?, 19)?
        .div_positive(fact(19)?)?
        .clamp_nonnegative();
    sum.pad(tail)
}
fn homogeneous(attenuation: f64, theta: f64, h: f64) -> Result<Z, String> {
    zscale(exp_phase(theta, h)?, expneg_product(attenuation, h)?)
}
/// Up to max_l=9, batch all moments using one common list of real J_r.
/// Exact represented real coefficients; source rates are not nominal decimals.
fn source_moments_all(a: f64, b: f64, theta: f64, h: f64, max_l: usize) -> Result<Vec<Z>, String> {
    if max_l > 9 {
        return Err("moment polynomial degree outside N4 limit".into());
    }
    let p = 18;
    let j = j_list(a, b, h, max_l + p + 1)?;
    if theta == 0. {
        return Ok(j
            .into_iter()
            .take(max_l + 1)
            .map(|x| Z {
                re: x,
                im: I::zero(),
            })
            .collect());
    }
    let phase = exp_phase(theta, h)?;
    let ith = Z {
        re: I::zero(),
        im: i(-theta)?,
    };
    let mut out = Vec::with_capacity(max_l + 1);
    let common = pow(I::new(0., theta.abs())?, p + 1)?
        .div_positive(fact(p + 1)?)?
        .clamp_nonnegative();
    for ell in 0..=max_l {
        let mut inner = Z::zero();
        let mut co = Z::one();
        for q in 0..=p {
            inner = inner.add(zscale(co, j[ell + q])?)?;
            co = co.mul(ith)?.div_positive(i((q + 1) as f64)?)?;
        }
        let tail = common.mul(j[ell + p + 1])?.clamp_nonnegative();
        out.push(phase.mul(inner.pad(tail)?)?);
    }
    Ok(out)
}

/// Exact semigroup–cubic single-window first Picard, no root truncation beyond
/// the SOURCE'S OWN p=2 oscillation polynomial. Input y is exact represented dyadic.
/// Fail closed outside the frozen N3H research configuration.
#[allow(clippy::needless_range_loop)] // explicit fixed alpha/c/d/e source-index contract
pub(crate) fn ideal_first_picard(
    view: &FrozenView<'_>,
    h: f64,
    p: usize,
) -> Result<Vec<Z>, String> {
    if view.n <= 8
        || view.n != view.y.len()
        || p != 2
        || h != 0.25
        || view.weights.len() != view.n
        || view.xi.len() != view.n
        || view.kappa.len() != view.n
        || view.phi.iter().any(|v| v.len() != view.n)
        || !view.omega.is_finite()
        || view.omega != 0.5
        || view.chi != &[0.05, 0.02]
    {
        return Err("N4A source/degree/window mismatch".into());
    }
    let mut poly = vec![vec![vec![Z::zero(); p + 1]; 2]; 2];
    for alpha in 0..2 {
        for j in 0..view.n {
            let cluster = if view.kappa[j] == 0.25 {
                0
            } else if view.kappa[j] == 1000. {
                1
            } else {
                return Err("unknown two-rate cluster".into());
            };
            if view.phi[0][j] != 1. || view.phi[1][j].abs() != 1. {
                return Err("basis mismatch".into());
            }
            if view.weights[j] <= 0. || !view.weights[j].is_finite() {
                return Err("bad represented weight".into());
            }
            let native_theta = view.omega * view.xi[j];
            if view.omega.mul_add(view.xi[j], -native_theta) != 0. {
                return Err("inexact source omega*xi".into());
            }
            let base = zscale(
                zp(view.y[j].re, view.y[j].im)?,
                i(view.weights[j])?.mul(i(view.phi[alpha][j])?)?,
            )?;
            let freq = Z {
                re: I::zero(),
                im: i(native_theta)?,
            };
            let mut pow = Z::one();
            for ell in 0..=p {
                poly[alpha][cluster][ell] = poly[alpha][cluster][ell].add(base.mul(pow)?)?;
                pow = pow.mul(freq)?.div_positive(i((ell + 1) as f64)?)?;
            }
        }
    }
    let mut triples = Vec::with_capacity(16);
    for alpha in 0..2 {
        for c in 0..2 {
            for d in 0..2 {
                for e in 0..2 {
                    let temp = polynomial_convolution(
                        &poly[alpha][c],
                        &poly[alpha][d]
                            .iter()
                            .copied()
                            .map(zconj)
                            .collect::<Vec<_>>(),
                    )?;
                    let coef = polynomial_convolution(&temp, &poly[alpha][e])?;
                    let decay = (if c == 0 { 0.25 } else { 1000. })
                        + (if d == 0 { 0.25 } else { 1000. })
                        + (if e == 0 { 0.25 } else { 1000. });
                    triples.push((alpha, decay, coef));
                }
            }
        }
    }
    let mut result = Vec::with_capacity(view.n);
    for j in 0..view.n {
        let theta = view.omega * view.xi[j];
        let mut ans = homogeneous(view.kappa[j], theta, h)?.mul(zp(view.y[j].re, view.y[j].im)?)?;
        // Expensive but preserves the exact N3H cubic c,d,e sums.
        // Both alphas use the same damping cluster kernels. Cache across alpha.
        let mut moments = Vec::with_capacity(8);
        for c in 0..2 {
            for d in 0..2 {
                for e in 0..2 {
                    let decay = (if c == 0 { 0.25 } else { 1000. })
                        + (if d == 0 { 0.25 } else { 1000. })
                        + (if e == 0 { 0.25 } else { 1000. });
                    moments.push(source_moments_all(view.kappa[j], decay, theta, h, 3 * p)?);
                }
            }
        }
        for (item_index, (alpha, _decay, coef)) in triples.iter().enumerate() {
            // alpha is the outer loop and c/d/e are ordered binary digits.
            // This preserves individual terms even when distinct cluster triples
            // have the same total damping.
            let idx = item_index % 8;
            let mut sum = Z::zero();
            for (q, v) in coef.iter().enumerate() {
                sum = sum.add(v.mul(moments[idx][q])?)?;
            }
            let prefactor = Z {
                re: I::zero(),
                im: i(view.phi[*alpha][j])?.mul(i(view.chi[*alpha])?)?,
            };
            ans = ans.add(prefactor.mul(sum)?)?;
        }
        result.push(ans);
    }
    Ok(result)
}

pub(crate) fn compare_float(
    ideal: &[Z],
    native: &[C],
    ideal_errors: &[f64],
) -> Result<Vec<CertifiedComponent>, String> {
    if ideal.len() != native.len() || ideal_errors.len() != ideal.len() {
        return Err("component count mismatch".into());
    }
    let mut all = Vec::with_capacity(ideal.len());
    for j in 0..ideal.len() {
        let gap_re = i(native[j].re)?.sub(ideal[j].re)?.mag();
        let gap_im = i(native[j].im)?.sub(ideal[j].im)?.mag();
        let gap = i(gap_re)?.add(i(gap_im)?)?.hi;
        if !ideal_errors[j].is_finite() || ideal_errors[j] < 0. {
            return Err("uncertified ideal input".into());
        }
        let total = i(ideal_errors[j])?.add(i(gap)?)?.hi;
        all.push(CertifiedComponent {
            exact_first_picard: ideal[j],
            float_gap_l1_upper: gap,
            initial_epoch_physical_l1_upper: total,
        });
    }
    Ok(all)
}
