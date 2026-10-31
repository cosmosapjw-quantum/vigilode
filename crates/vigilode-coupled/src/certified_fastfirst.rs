//! Opt-in source-owned diagonal-affine RHS-only fast-first research.
//!
//! Exact mathematical source (represented input numbers interpreted as reals):
//! G_i(x) = a_i*x_i + b_i, q=max_i |a_i| < 1. The coefficients live in
//! private vectors, with no supplied caller callback or contraction constant.
//! Directed residual bounds certify an exact-real affine fixed point; the
//! binary64 map and Anderson gamma are only untrusted candidate generators.
//! This is NOT a generic ODE/global-IVP/RODAS or production default certificate.

use rodas5p_core::directed::{
    Interval, add_down, add_up, div_up, mul_down, mul_up, sqrt_down, sqrt_up, sub_down,
};
use serde::Serialize;

#[derive(Clone, Debug)]
pub struct FrozenDiagonalAffine {
    diagonal: Vec<f64>,
    shift: Vec<f64>,
    q: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum CertifiedFailureKind {
    Input,
    Budget,
    Arithmetic,
    CertificateInconclusive,
    IterationLimit,
}

#[derive(Clone, Debug, Serialize)]
pub struct CertifiedFailure {
    pub kind: CertifiedFailureKind,
    pub detail: String,
}

#[derive(Clone, Debug)]
pub struct CertifiedConfig {
    pub lambdas: Vec<f64>,
    /// Requested L2 distance to exact real fixed point, not operational residual.
    pub root_error_target: f64,
    pub max_iterations_per_station: usize,
    pub max_map_evaluations: u64,
    /// false: certified baseline-first Anderson; true: certified speculative fast-first.
    pub fast_first: bool,
}

impl Default for CertifiedConfig {
    fn default() -> Self {
        Self {
            lambdas: vec![1.0],
            root_error_target: 1.0e-10,
            max_iterations_per_station: 4096,
            max_map_evaluations: 16384,
            fast_first: true,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CertifiedWork {
    pub map_attempted: u64,
    pub map_completed: u64,
    pub exact_residual_interval_evaluations: u64,
    pub picard_updates: u64,
    pub rounded_picard_admissions: u64,
    pub speculative_trials: u64,
    pub trials_admitted: u64,
    pub trials_refused: u64,
    pub coefficient_skips: u64,
    pub budget_skips: u64,
    pub continuation_stations: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CertifiedPoint {
    pub lambda: f64,
    pub state: Vec<f64>,
    pub exact_residual_l2_lower: f64,
    pub exact_residual_l2_upper: f64,
    pub exact_root_error_upper: f64,
    pub map_evaluations: u64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CertifiedAttempt {
    pub root: Option<Vec<f64>>,
    pub points: Vec<CertifiedPoint>,
    pub work: CertifiedWork,
    pub failure: Option<CertifiedFailure>,
}

struct Value {
    state: Vec<f64>,
    mapped: Vec<f64>,
    float_residual: Vec<f64>,
    residual_lower: f64,
    residual_upper: f64,
    root_upper: f64,
    float_image_error_upper: f64,
}

fn fail(kind: CertifiedFailureKind, detail: impl Into<String>) -> CertifiedFailure {
    CertifiedFailure {
        kind,
        detail: detail.into(),
    }
}

fn arithmetic(error: impl std::fmt::Display) -> CertifiedFailure {
    fail(CertifiedFailureKind::Arithmetic, error.to_string())
}

impl FrozenDiagonalAffine {
    pub fn new(diagonal: Vec<f64>, shift: Vec<f64>) -> Result<Self, CertifiedFailure> {
        if diagonal.is_empty()
            || diagonal.len() != shift.len()
            || diagonal.iter().chain(shift.iter()).any(|v| !v.is_finite())
        {
            return Err(fail(
                CertifiedFailureKind::Input,
                "invalid source dimension or coefficients",
            ));
        }
        // Every abs() is exact for a finite binary64 input.
        let q = diagonal.iter().fold(0.0_f64, |m, &v| m.max(v.abs()));
        if q >= 1.0 {
            return Err(fail(
                CertifiedFailureKind::Input,
                "represented source is not L2-contractive",
            ));
        }
        Ok(Self { diagonal, shift, q })
    }

    pub fn dimension(&self) -> usize {
        self.diagonal.len()
    }

    /// Exact-real L2 contraction majorant from privately owned source coefficients.
    pub fn represented_contraction(&self) -> f64 {
        self.q
    }

    /// Observational readback only: callers cannot replace the owned source.
    pub fn represented_source_bits(&self) -> Vec<(u64, u64)> {
        self.diagonal
            .iter()
            .zip(&self.shift)
            .map(|(a, b)| (a.to_bits(), b.to_bits()))
            .collect()
    }

    fn evaluate(
        &self,
        anchor: &[f64],
        state: Vec<f64>,
        lambda: f64,
        denom_lower: f64,
        cfg: &CertifiedConfig,
        work: &mut CertifiedWork,
    ) -> Result<Value, CertifiedFailure> {
        if state.len() != self.dimension() || state.iter().any(|v| !v.is_finite()) {
            return Err(fail(
                CertifiedFailureKind::Input,
                "candidate is not a finite represented vector",
            ));
        }
        if work.map_attempted >= cfg.max_map_evaluations {
            return Err(fail(
                CertifiedFailureKind::Budget,
                "source evaluation cap reached before dispatch",
            ));
        }
        work.map_attempted += 1;
        let mut mapped = Vec::with_capacity(self.dimension());
        let mut residual = Vec::with_capacity(self.dimension());
        let mut lower_sq = 0.0_f64;
        let mut upper_sq = 0.0_f64;
        let mut image_error_sq = 0.0_f64;
        let one_minus_lambda = Interval::point(1.0)
            .and_then(|one| one.sub(Interval::point(lambda)?))
            .map_err(arithmetic)?;
        let lambda_interval = Interval::point(lambda).map_err(arithmetic)?;
        for i in 0..self.dimension() {
            let (a, b, x, c) = (self.diagonal[i], self.shift[i], state[i], anchor[i]);
            // Operational f64 map for generating subsequent candidates.
            let g_float = a * x + b;
            let mapped_float = (1.0 - lambda) * c + lambda * g_float;
            let residual_float = mapped_float - x;
            if !mapped_float.is_finite() || !residual_float.is_finite() {
                return Err(fail(
                    CertifiedFailureKind::Arithmetic,
                    "floating callback overflow/nonfinite",
                ));
            }
            // Independent enclosure of the ORIGINAL exact-real map, not of g_float.
            let a_real = Interval::point(a).map_err(arithmetic)?;
            let x_real = Interval::point(x).map_err(arithmetic)?;
            let b_real = Interval::point(b).map_err(arithmetic)?;
            let c_real = Interval::point(c).map_err(arithmetic)?;
            let exact_mapped = one_minus_lambda
                .mul(c_real)
                .and_then(|z| {
                    a_real
                        .mul(x_real)?
                        .add(b_real)?
                        .mul(lambda_interval)?
                        .add(z)
                })
                .map_err(arithmetic)?;
            let exact_residual = exact_mapped.sub(x_real).map_err(arithmetic)?;
            let float_image_error = exact_mapped
                .sub(Interval::point(mapped_float).map_err(arithmetic)?)
                .map_err(arithmetic)?;
            image_error_sq = add_up(
                image_error_sq,
                mul_up(float_image_error.mag(), float_image_error.mag()).map_err(arithmetic)?,
            )
            .map_err(arithmetic)?;
            // All squared exact residuals are nonnegative. A directed
            // underflowed lower product may be one negative subnormal ulp;
            // clamping its bound to zero is exact and avoids a spurious sqrt error.
            let lower_term =
                mul_down(exact_residual.mig(), exact_residual.mig()).map_err(arithmetic)?;
            lower_sq = add_down(lower_sq, lower_term.max(0.0))
                .map_err(arithmetic)?
                .max(0.0);
            upper_sq = add_up(
                upper_sq,
                mul_up(exact_residual.mag(), exact_residual.mag()).map_err(arithmetic)?,
            )
            .map_err(arithmetic)?;
            mapped.push(mapped_float);
            residual.push(residual_float);
        }
        let residual_lower = sqrt_down(lower_sq).map_err(arithmetic)?;
        let residual_upper = sqrt_up(upper_sq).map_err(arithmetic)?;
        let root_upper = div_up(residual_upper, denom_lower).map_err(arithmetic)?;
        let float_image_error_upper = sqrt_up(image_error_sq).map_err(arithmetic)?;
        work.map_completed += 1;
        work.exact_residual_interval_evaluations += 1;
        Ok(Value {
            state,
            mapped,
            float_residual: residual,
            residual_lower,
            residual_upper,
            root_upper,
            float_image_error_upper,
        })
    }

    pub fn solve(&self, anchor: &[f64], cfg: &CertifiedConfig) -> CertifiedAttempt {
        let mut out = CertifiedAttempt::default();
        if anchor.len() != self.dimension()
            || anchor.iter().any(|v| !v.is_finite())
            || !cfg.root_error_target.is_finite()
            || cfg.root_error_target <= 0.0
            || cfg.max_iterations_per_station == 0
            || cfg.max_map_evaluations == 0
            || cfg.lambdas.last() != Some(&1.0)
            || cfg
                .lambdas
                .iter()
                .any(|&l| !l.is_finite() || l <= 0.0 || l > 1.0)
            || cfg.lambdas.windows(2).any(|w| w[0] >= w[1])
        {
            out.failure = Some(fail(
                CertifiedFailureKind::Input,
                "invalid source-owned solver contract",
            ));
            return out;
        }
        let mut seed = anchor.to_vec();
        for &lambda in &cfg.lambdas {
            out.work.continuation_stations += 1;
            let before = out.work.map_attempted;
            let q_upper = match mul_up(lambda, self.q) {
                Ok(v) if v < 1.0 => v,
                _ => {
                    out.failure = Some(fail(
                        CertifiedFailureKind::Arithmetic,
                        "source contraction upper reached unity",
                    ));
                    return out;
                }
            };
            let denom_lower = match sub_down(1.0, q_upper) {
                Ok(v) if v > 0.0 => v,
                _ => {
                    out.failure = Some(fail(
                        CertifiedFailureKind::Arithmetic,
                        "nonpositive inverse contraction denominator",
                    ));
                    return out;
                }
            };
            let mut current =
                match self.evaluate(anchor, seed, lambda, denom_lower, cfg, &mut out.work) {
                    Ok(v) => v,
                    Err(e) => {
                        out.failure = Some(e);
                        return out;
                    }
                };
            let mut previous: Option<Value> = None;
            let mut iterations = 0;
            while current.root_upper > cfg.root_error_target {
                if iterations >= cfg.max_iterations_per_station {
                    out.failure = Some(fail(
                        CertifiedFailureKind::IterationLimit,
                        "station iteration cap",
                    ));
                    return out;
                }
                let mut chosen: Option<Value> = None;
                let mut baseline: Option<Value> = None;
                // Both arms retain the SAME directed source/target. The
                // conventional arm evaluates Picard first, then the Anderson
                // candidate; fast-first evaluates the candidate first and
                // skips Picard only on the proved contraction-envelope gate.
                if !cfg.fast_first {
                    out.work.picard_updates += 1;
                    baseline = Some(
                        match self.evaluate(
                            anchor,
                            current.mapped.clone(),
                            lambda,
                            denom_lower,
                            cfg,
                            &mut out.work,
                        ) {
                            Ok(v) => v,
                            Err(e) => {
                                out.failure = Some(e);
                                return out;
                            }
                        },
                    );
                }
                let baseline_needs_more = baseline
                    .as_ref()
                    .is_none_or(|v| v.root_upper > cfg.root_error_target);
                if baseline_needs_more && let Some(old) = &previous {
                    let reserved = if baseline.is_some() { 1 } else { 2 };
                    if cfg
                        .max_map_evaluations
                        .saturating_sub(out.work.map_attempted)
                        < reserved
                    {
                        out.work.budget_skips += 1;
                    } else {
                        let scale = current
                            .float_residual
                            .iter()
                            .chain(old.float_residual.iter())
                            .fold(0.0_f64, |s, &v| s.max(v.abs()));
                        let (mut numerator, mut denominator) = (0.0_f64, 0.0_f64);
                        if scale > 0.0 {
                            for (&r, &r_old) in
                                current.float_residual.iter().zip(&old.float_residual)
                            {
                                let d = (r / scale) - (r_old / scale);
                                numerator += (r / scale) * d;
                                denominator += d * d;
                            }
                        }
                        let gamma = numerator / denominator;
                        if gamma.is_finite() {
                            let trial: Vec<_> = current
                                .mapped
                                .iter()
                                .zip(&old.mapped)
                                .map(|(&g, &g_old)| g - gamma * (g - g_old))
                                .collect();
                            if trial.iter().all(|v| v.is_finite()) {
                                out.work.speculative_trials += 1;
                                let candidate = match self.evaluate(
                                    anchor,
                                    trial,
                                    lambda,
                                    denom_lower,
                                    cfg,
                                    &mut out.work,
                                ) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        out.failure = Some(e);
                                        return out;
                                    }
                                };
                                let certified = if cfg.fast_first {
                                    // The ORIGINAL exact-real accelerated
                                    // residual is bounded without a Picard
                                    // callback. A downward threshold prevents
                                    // admission from arithmetic rounding.
                                    match mul_down(q_upper, current.residual_lower) {
                                        Ok(bound) => candidate.residual_upper <= bound,
                                        Err(e) => {
                                            out.failure = Some(arithmetic(e));
                                            return out;
                                        }
                                    }
                                } else {
                                    // Conventional baseline-first Anderson:
                                    // compare both ORIGINAL exact-real
                                    // residuals, including all outward widths.
                                    candidate.residual_upper
                                        <= baseline
                                            .as_ref()
                                            .expect("baseline-first Picard evaluated")
                                            .residual_lower
                                };
                                if certified {
                                    out.work.trials_admitted += 1;
                                    chosen = Some(candidate);
                                } else {
                                    out.work.trials_refused += 1;
                                }
                            } else {
                                out.work.coefficient_skips += 1;
                            }
                        } else {
                            out.work.coefficient_skips += 1;
                        }
                    }
                }
                if chosen.is_none() {
                    let baseline = match baseline {
                        Some(v) => v,
                        None => {
                            out.work.picard_updates += 1;
                            match self.evaluate(
                                anchor,
                                current.mapped.clone(),
                                lambda,
                                denom_lower,
                                cfg,
                                &mut out.work,
                            ) {
                                Ok(v) => v,
                                Err(e) => {
                                    out.failure = Some(e);
                                    return out;
                                }
                            }
                        }
                    };
                    // A represented Picard image fl(H(x)) has an ADDITIVE
                    // error envelope, never presumed exact contraction.
                    // ||r(y)|| <= q||r(x)||+(1+q)||fl(H(x))-H(x)||.
                    let envelope = mul_up(q_upper, current.residual_upper).and_then(|base| {
                        add_up(1.0, q_upper)
                            .and_then(|factor| mul_up(factor, current.float_image_error_upper))
                            .and_then(|roundoff| add_up(base, roundoff))
                    });
                    match envelope {
                        Ok(v) if baseline.residual_lower <= v => {
                            out.work.rounded_picard_admissions += 1;
                            chosen = Some(baseline);
                        }
                        Ok(_) => {
                            out.failure = Some(fail(
                                CertifiedFailureKind::CertificateInconclusive,
                                "rounded Picard contradicts inexact contraction envelope",
                            ));
                            return out;
                        }
                        Err(e) => {
                            out.failure = Some(arithmetic(e));
                            return out;
                        }
                    }
                }
                previous = Some(current);
                // chosen is always populated by a successful trial or Picard fallback.
                current = chosen.expect("certified fast-first selected a candidate");
                iterations += 1;
            }
            seed = current.state;
            out.points.push(CertifiedPoint {
                lambda,
                state: seed.clone(),
                exact_residual_l2_lower: current.residual_lower,
                exact_residual_l2_upper: current.residual_upper,
                exact_root_error_upper: current.root_upper,
                map_evaluations: out.work.map_attempted - before,
            });
        }
        out.root = Some(seed);
        out
    }
}
