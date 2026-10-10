//! Research-only current-operator certificate for a represented real 2 x 2 solve.
//!
//! The supplied binary64 entries are interpreted as exact real numbers.  Every
//! arithmetic operation below is widened by one adjacent binary64 value.  An
//! interval determinant excluding zero therefore permits an interval inverse.
//! For the recomputed residual r = b - W x, W^{-1} r encloses x_exact - x.
//! Dividing each component by its positive error scale and taking the infinity
//! norm also bounds sqrt(sum_i ((x_exact_i - x_i) / s_i)^2 / 2).
//!
//! This is not a certificate for how W was formed, an approximate JVP, a larger
//! operator containing this block, a nonlinear stage, or an integration endpoint.
//! There is no serialized/deserialized authority or operator cache.  The witness
//! owns W and its scales, and recomputes the residual for each candidate.

use serde_json::{Value, json};

#[derive(Clone, Copy, Debug)]
struct Interval {
    lo: f64,
    hi: f64,
}

#[derive(Clone, Copy, Debug)]
pub enum Rejection {
    NonfiniteMatrix,
    InvalidScale,
    NonfiniteRhs,
    NonfiniteCandidate,
    ArithmeticOverflow,
    DeterminantContainsZero,
}

impl Rejection {
    fn name(self) -> &'static str {
        match self {
            Self::NonfiniteMatrix => "nonfinite_matrix",
            Self::InvalidScale => "nonpositive_or_nonfinite_scale",
            Self::NonfiniteRhs => "nonfinite_rhs",
            Self::NonfiniteCandidate => "nonfinite_candidate",
            Self::ArithmeticOverflow => "nonfinite_intermediate_or_outward_endpoint",
            Self::DeterminantContainsZero => "determinant_interval_contains_zero",
        }
    }
}

fn next_up(x: f64) -> f64 {
    debug_assert!(x.is_finite());
    if x == 0.0 {
        f64::from_bits(1)
    } else if x > 0.0 {
        f64::from_bits(x.to_bits() + 1)
    } else {
        f64::from_bits(x.to_bits() - 1)
    }
}

fn next_down(x: f64) -> f64 {
    debug_assert!(x.is_finite());
    if x == 0.0 {
        -f64::from_bits(1)
    } else if x > 0.0 {
        f64::from_bits(x.to_bits() - 1)
    } else {
        f64::from_bits(x.to_bits() + 1)
    }
}

impl Interval {
    fn point(x: f64) -> Self {
        debug_assert!(x.is_finite());
        Self { lo: x, hi: x }
    }

    fn outward(lo: f64, hi: f64) -> Result<Self, Rejection> {
        if !lo.is_finite() || !hi.is_finite() {
            return Err(Rejection::ArithmeticOverflow);
        }
        let result = Self {
            lo: next_down(lo),
            hi: next_up(hi),
        };
        if !result.lo.is_finite() || !result.hi.is_finite() {
            return Err(Rejection::ArithmeticOverflow);
        }
        Ok(result)
    }

    fn add(self, rhs: Self) -> Result<Self, Rejection> {
        Self::outward(self.lo + rhs.lo, self.hi + rhs.hi)
    }

    fn sub(self, rhs: Self) -> Result<Self, Rejection> {
        Self::outward(self.lo - rhs.hi, self.hi - rhs.lo)
    }

    fn neg(self) -> Self {
        Self {
            lo: -self.hi,
            hi: -self.lo,
        }
    }

    fn hull_of_rounded(values: [f64; 4]) -> Result<Self, Rejection> {
        if values.iter().any(|x| !x.is_finite()) {
            return Err(Rejection::ArithmeticOverflow);
        }
        let lo = values.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        Self::outward(lo, hi)
    }

    fn mul(self, rhs: Self) -> Result<Self, Rejection> {
        Self::hull_of_rounded([
            self.lo * rhs.lo,
            self.lo * rhs.hi,
            self.hi * rhs.lo,
            self.hi * rhs.hi,
        ])
    }

    fn div(self, rhs: Self) -> Result<Self, Rejection> {
        if rhs.lo <= 0.0 && rhs.hi >= 0.0 {
            return Err(Rejection::DeterminantContainsZero);
        }
        Self::hull_of_rounded([
            self.lo / rhs.lo,
            self.lo / rhs.hi,
            self.hi / rhs.lo,
            self.hi / rhs.hi,
        ])
    }

    fn abs_upper(self) -> f64 {
        // Negation and max are exact for finite binary64 endpoints.
        self.lo.abs().max(self.hi.abs())
    }

    fn as_pair(self) -> [f64; 2] {
        [self.lo, self.hi]
    }
}

/// Immutable witness bound to an owned current represented operator and scales.
/// No Deserialize implementation: construction is the only authority path.
#[derive(Debug)]
pub struct GainWitness2 {
    w: [[f64; 2]; 2],
    scales: [f64; 2],
    inverse: [[Interval; 2]; 2],
    determinant: Interval,
    scaled_residual_gain_inf_upper: f64,
}

#[derive(Debug)]
pub struct Certificate2 {
    residual: [Interval; 2],
    correction: [Interval; 2],
    weighted_correction: [Interval; 2],
    weighted_inf_upper: f64,
    scaled_residual_gain_inf_upper: f64,
}

impl GainWitness2 {
    pub fn new(w: [[f64; 2]; 2], scales: [f64; 2]) -> Result<Self, Rejection> {
        if w.iter().flatten().any(|x| !x.is_finite()) {
            return Err(Rejection::NonfiniteMatrix);
        }
        if scales.iter().any(|x| !x.is_finite() || *x <= 0.0) {
            return Err(Rejection::InvalidScale);
        }
        let [[a, b], [c, d]] = w.map(|row| row.map(Interval::point));
        let determinant = a.mul(d)?.sub(b.mul(c)?)?;
        if determinant.lo <= 0.0 && determinant.hi >= 0.0 {
            return Err(Rejection::DeterminantContainsZero);
        }
        let inverse = [
            [d.div(determinant)?, b.neg().div(determinant)?],
            [c.neg().div(determinant)?, a.div(determinant)?],
        ];

        // Bound || S^{-1} W^{-1} S ||_infinity, where S = diag(scales).
        // Consequently || S^{-1} delta ||_infinity <= gain *
        // || S^{-1} residual ||_infinity.  certify() uses the tighter signed
        // interval correction directly, not this optional scalar product.
        let mut scaled_residual_gain_inf_upper: f64 = 0.0;
        for i in 0..2 {
            let mut sum = Interval::point(0.0);
            for j in 0..2 {
                let term = Interval::point(inverse[i][j].abs_upper())
                    .mul(Interval::point(scales[j]))?
                    .div(Interval::point(scales[i]))?;
                sum = sum.add(term)?;
            }
            scaled_residual_gain_inf_upper = scaled_residual_gain_inf_upper.max(sum.hi);
        }
        Ok(Self {
            w,
            scales,
            inverse,
            determinant,
            scaled_residual_gain_inf_upper,
        })
    }

    pub fn certify(&self, b: [f64; 2], x: [f64; 2]) -> Result<Certificate2, Rejection> {
        if b.iter().any(|x| !x.is_finite()) {
            return Err(Rejection::NonfiniteRhs);
        }
        if x.iter().any(|x| !x.is_finite()) {
            return Err(Rejection::NonfiniteCandidate);
        }
        let mut residual = [Interval::point(0.0); 2];
        for i in 0..2 {
            let dot = Interval::point(self.w[i][0])
                .mul(Interval::point(x[0]))?
                .add(Interval::point(self.w[i][1]).mul(Interval::point(x[1]))?)?;
            residual[i] = Interval::point(b[i]).sub(dot)?;
        }
        let mut correction = [Interval::point(0.0); 2];
        let mut weighted_correction = [Interval::point(0.0); 2];
        let mut weighted_inf_upper: f64 = 0.0;
        for i in 0..2 {
            correction[i] = self.inverse[i][0]
                .mul(residual[0])?
                .add(self.inverse[i][1].mul(residual[1])?)?;
            weighted_correction[i] = correction[i].div(Interval::point(self.scales[i]))?;
            weighted_inf_upper = weighted_inf_upper.max(weighted_correction[i].abs_upper());
        }
        Ok(Certificate2 {
            residual,
            correction,
            weighted_correction,
            weighted_inf_upper,
            scaled_residual_gain_inf_upper: self.scaled_residual_gain_inf_upper,
        })
    }
}

impl Certificate2 {
    pub fn wrms_upper(&self) -> f64 {
        self.weighted_inf_upper
    }

    fn as_json(&self) -> Value {
        json!({
            "residual_intervals": self.residual.map(Interval::as_pair),
            "correction_intervals": self.correction.map(Interval::as_pair),
            "weighted_correction_intervals": self.weighted_correction.map(Interval::as_pair),
            "weighted_inf_upper": self.weighted_inf_upper,
            "wrms_upper": self.wrms_upper(),
            "scaled_residual_gain_inf_upper": self.scaled_residual_gain_inf_upper
        })
    }
}

fn float_json(x: f64) -> Value {
    if x.is_finite() {
        json!(x)
    } else if x.is_nan() {
        json!("NaN")
    } else if x.is_sign_positive() {
        json!("+Infinity")
    } else {
        json!("-Infinity")
    }
}

fn case(
    id: &str,
    partition: &str,
    w: [[f64; 2]; 2],
    b: [f64; 2],
    x: [f64; 2],
    scales: [f64; 2],
) -> Value {
    let mut output = json!({
        "id": id,
        "partition": partition,
        "W": w.map(|row| row.map(float_json)),
        "b": b.map(float_json),
        "x": x.map(float_json),
        "scales": scales.map(float_json)
    });
    match GainWitness2::new(w, scales) {
        Err(error) => {
            output["status"] = json!("rejected");
            output["rejection_stage"] = json!("construction");
            output["error"] = json!(error.name());
        }
        Ok(witness) => {
            output["determinant_interval"] = json!(witness.determinant.as_pair());
            output["inverse_intervals"] =
                json!(witness.inverse.map(|row| row.map(Interval::as_pair)));
            match witness.certify(b, x) {
                Err(error) => {
                    output["status"] = json!("rejected");
                    output["rejection_stage"] = json!("certification");
                    output["error"] = json!(error.name());
                }
                Ok(certificate) => {
                    output["status"] = json!("admitted");
                    output["certificate"] = certificate.as_json();
                }
            }
        }
    }
    output
}

/// Fixed preregistered study.  No timing, tuning or adaptive case selection.
pub fn study() -> Value {
    let mut cells = Vec::new();
    let perturbation = 2.0_f64.powi(-40);
    for k in [0.0, 1.0, 1024.0, 1048576.0] {
        for (scale_index, scales) in [
            [1.0, 1.0],
            [2.0_f64.powi(-20), 2.0_f64.powi(20)],
            [2.0_f64.powi(20), 2.0_f64.powi(-20)],
        ]
        .into_iter()
        .enumerate()
        {
            for coordinate in 0..2 {
                let baseline = [1.0 - k, 1.0];
                let mut x = baseline;
                x[coordinate] += perturbation;
                let mut result = case(
                    &format!("grid_k{k}_s{scale_index}_p{coordinate}"),
                    "discovery",
                    [[1.0, k], [0.0, 1.0]],
                    [1.0, 1.0],
                    x,
                    scales,
                );
                result["perturbation_coordinate_zero_based"] = json!(coordinate);
                result["requested_perturbation"] = json!(perturbation);
                result["perturbation_survived"] = json!(x[coordinate] != baseline[coordinate]);
                cells.push(result);
            }
        }
    }
    cells.push(case(
        "control_symmetric",
        "control",
        [[3.0, 1.0], [1.0, 2.0]],
        [1.0, 1.0],
        [0.2 + perturbation, 0.4],
        [1.0, 1.0],
    ));
    cells.push(case(
        "control_near_singular",
        "control",
        [[1.0, 1.0], [1.0, 1.0 + perturbation]],
        [1.0, 1.0],
        [1.0, perturbation],
        [1.0, 1.0],
    ));
    for k in [7.0, 65536.0] {
        for coordinate in 0..2 {
            let baseline = [3.0 + 2.0 * k, -2.0];
            let mut x = baseline;
            let holdout_perturbation = 2.0_f64.powi(-30);
            x[coordinate] += holdout_perturbation;
            let mut result = case(
                &format!("holdout_k{k}_p{coordinate}"),
                "fixed_holdout",
                [[1.0, k], [0.0, 1.0]],
                [3.0, -2.0],
                x,
                [2.0_f64.powi(-7), 2.0_f64.powi(9)],
            );
            result["perturbation_coordinate_zero_based"] = json!(coordinate);
            result["requested_perturbation"] = json!(holdout_perturbation);
            result["perturbation_survived"] = json!(x[coordinate] != baseline[coordinate]);
            cells.push(result);
        }
    }

    let identity = [[1.0, 0.0], [0.0, 1.0]];
    let unit = [1.0, 1.0];
    let invalid_controls = vec![
        case(
            "invalid_singular",
            "invalid",
            [[1.0, 1.0], [1.0, 1.0]],
            unit,
            unit,
            unit,
        ),
        case(
            "invalid_nonfinite_w",
            "invalid",
            [[f64::NAN, 0.0], [0.0, 1.0]],
            unit,
            unit,
            unit,
        ),
        case(
            "invalid_nonfinite_b",
            "invalid",
            identity,
            [f64::INFINITY, 1.0],
            unit,
            unit,
        ),
        case(
            "invalid_nonfinite_x",
            "invalid",
            identity,
            unit,
            [1.0, f64::NAN],
            unit,
        ),
        case(
            "invalid_zero_scale",
            "invalid",
            identity,
            unit,
            unit,
            [0.0, 1.0],
        ),
        case(
            "invalid_negative_scale",
            "invalid",
            identity,
            unit,
            unit,
            [-1.0, 1.0],
        ),
        case(
            "invalid_infinite_scale",
            "invalid",
            identity,
            unit,
            unit,
            [1.0, f64::INFINITY],
        ),
        case(
            "invalid_nan_scale",
            "invalid",
            identity,
            unit,
            unit,
            [f64::NAN, 1.0],
        ),
        case(
            "invalid_determinant_overlap",
            "invalid",
            [[1.0, 1.0], [1.0, 1.0 + f64::EPSILON]],
            unit,
            unit,
            unit,
        ),
        case(
            "invalid_determinant_overflow",
            "invalid",
            [[f64::MAX, 0.0], [0.0, 2.0]],
            unit,
            unit,
            unit,
        ),
        case(
            "invalid_residual_overflow",
            "invalid",
            identity,
            [f64::MAX, 1.0],
            [-f64::MAX, 1.0],
            unit,
        ),
        case(
            "invalid_scale_gain_overflow",
            "invalid",
            // With identity the scale cancels in S^-1 W^-1 S: the original
            // negative control was actually valid.  A nonzero off-diagonal
            // makes this inverse gain exceed binary64 for the stated scales.
            [[1.0, 1.0], [0.0, 1.0]],
            unit,
            unit,
            [f64::from_bits(1), 1.0],
        ),
    ];
    let admitted_count = cells
        .iter()
        .filter(|cell| cell["status"] == "admitted")
        .count();
    let invalid_rejected_count = invalid_controls
        .iter()
        .filter(|cell| cell["status"] == "rejected")
        .count();
    json!({
        "schema_version": 1,
        "contract": {
            "matrix": "exact real matrix of exported binary64 entries; real 2x2",
            "norm": "weighted infinity upper bound, hence weighted RMS upper bound with dimension 2",
            "scales": "positive finite binary64, fixed per witness",
            "arithmetic": "binary64 round-to-nearest basic operations widened to adjacent endpoints; reject nonfinite intermediates/endpoints",
            "gain": "upper bound on induced infinity norm of S^-1 W^-1 S; S=diag(scales)",
            "excluded": ["W formation uncertainty", "JVP uncertainty", "off-block couplings", "nonlinear stage transfer", "time accumulation", "global endpoint error", "performance claims"],
            "nonfinite_json_encoding": "invalid inputs only: strings NaN, +Infinity, -Infinity"
        },
        "cells": cells,
        "invalid_controls": invalid_controls,
        "summary": {
            "expected_discovery": 24,
            "expected_controls": 2,
            "expected_fixed_holdout": 4,
            "admitted": admitted_count,
            "invalid_controls": invalid_controls.len(),
            "invalid_rejected": invalid_rejected_count,
            "independent_exact_oracle": "not run by this module; external checker required"
        }
    })
}
