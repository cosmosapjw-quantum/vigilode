//! Local, single-thread kernel probe; no whole-ODE performance claim.
use rodas5p_core::{DenseMatrix, LuFactorization};
use rodas5p_integrators::conditioned_polynomial::*;
use rodas5p_integrators::polynomial_resolvent::{
    DeclaredPolynomialResolvent as Expanded, PolynomialPhysicalNorm as Norm,
    PolynomialRefinementPolicy as Policy, PolynomialResolventError as Error,
    PolynomialResolventWorkspace as ExpandedWs,
};
use serde_json::json;
use std::{
    hint::black_box,
    time::{Duration, Instant},
};
struct RankOneNilpotent {
    d: ChartDeclaration,
}
impl RankOneNilpotent {
    fn moment(&self, x: &[f64]) -> f64 {
        x.iter()
            .enumerate()
            .map(|(i, x)| if i % 2 == 0 { *x } else { -x })
            .sum::<f64>()
            / x.len() as f64
    }
    fn direct(&self, b: &[f64], out: &mut [f64]) {
        let cb = self.moment(b);
        let a = 1.0 - self.d.center();
        for (x, b) in out.iter_mut().zip(b) {
            *x = b / a + self.d.scale() / (a * a) * cb;
        }
    }
}
impl DeclaredChartOperator for RankOneNilpotent {
    fn declaration(&self) -> &ChartDeclaration {
        &self.d
    }
    fn apply_chart(&self, x: &[f64], out: &mut [f64]) -> Result<(), Error> {
        out.fill(self.moment(x));
        Ok(())
    }
    fn apply_original(&self, x: &[f64], out: &mut [f64]) -> Result<(), Error> {
        let c = self.moment(x);
        for (o, x) in out.iter_mut().zip(x) {
            *o = self.d.scale().mul_add(c, self.d.center() * x);
        }
        Ok(())
    }
}
fn residual(p: &RankOneNilpotent, b: &[f64], x: &[f64], tmp: &mut [f64]) -> f64 {
    p.apply_original(x, tmp).unwrap();
    b.iter()
        .zip(x)
        .zip(tmp)
        .map(|((b, x), bx)| (b - x + *bx).abs())
        .fold(0.0, f64::max)
}
fn main() {
    let center = -100.0;
    let scale = 100.0;
    let tol = 1e-11;
    for n in [32, 128, 512] {
        let declaration = ChartDeclaration::new(
            ChartOperatorIdentity {
                operator_key: 17,
                dimension: n,
                model_epoch: 1,
                provider_epoch: 1,
                operator_scale_bits: 1.0_f64.to_bits(),
                physical_norm: Norm::Infinity,
            },
            center,
            scale,
            &[0.0, 0.0, 1.0],
            0.0,
            1.0,
            91,
        )
        .unwrap();
        let p = RankOneNilpotent { d: declaration };
        let mut setup = ChartWork::default();
        let plan = ConditionedPolynomialPlan::prepare(&p.d, 1e-12, &mut setup).unwrap();
        let mut ws = ChartWorkspace::new(n);
        let q = [center * center, -2.0 * center, 1.0];
        let expanded = Expanded::new(&q, 0.0, center.abs() + scale, 1e-12).unwrap();
        let mut expanded_ws = ExpandedWs::new(n);
        let m=DenseMatrix::new(n,n,(0..n*n).map(|k| if k/n==k%n {1.0-center}else{0.0} - scale*(if k%n%2==0 {1.0}else{-1.0})/n as f64).collect()).unwrap();
        let lu = LuFactorization::new(&m).unwrap();
        let exact: Vec<f64> = (0..n).map(|i| (i % 17) as f64 / 17.0 - 0.5).collect();
        let mut b = vec![0.0; n];
        p.apply_original(&exact, &mut b).unwrap();
        for i in 0..n {
            b[i] = exact[i] - b[i];
        }
        let mut out = vec![0.0; n];
        let mut tmp = vec![0.0; n];
        let mut times = vec![Vec::new(); 6];
        let mut errors = [0.0_f64; 6];
        let mut residuals = [0.0_f64; 6];
        let mut accepted = [true; 6];
        for round in 0..11 {
            for offset in 0..6 {
                let mode = (round + offset) % 6;
                let start = Instant::now();
                let mut calls = 0usize;
                while calls == 0 || start.elapsed() < Duration::from_millis(2) {
                    let ok = match mode {
                        0 => {
                            let mut w = ChartWork::default();
                            plan.apply(
                                &p,
                                &b,
                                &mut out,
                                &mut ws,
                                Policy::default(),
                                tol,
                                1e-13,
                                &mut w,
                            )
                            .unwrap()
                            .accepted
                        }
                        1 => {
                            let mut w = ChartWork::default();
                            let fresh_provider = RankOneNilpotent {
                                d: ChartDeclaration::new(
                                    p.d.identity(),
                                    p.d.center(),
                                    p.d.scale(),
                                    p.d.coefficients(),
                                    p.d.relation_norm(),
                                    p.d.chart_norm(),
                                    p.d.evidence_reference(),
                                )
                                .unwrap(),
                            };
                            let new_plan = ConditionedPolynomialPlan::prepare(
                                &fresh_provider.d,
                                1e-12,
                                &mut w,
                            )
                            .unwrap();
                            new_plan
                                .apply(
                                    &fresh_provider,
                                    &b,
                                    &mut out,
                                    &mut ChartWorkspace::new(n),
                                    Policy::default(),
                                    tol,
                                    1e-13,
                                    &mut w,
                                )
                                .unwrap()
                                .accepted
                        }
                        2 => {
                            expanded
                                .apply_checked(
                                    &b,
                                    &mut out,
                                    &mut expanded_ws,
                                    tol,
                                    1e-13,
                                    |x, y| p.apply_original(x, y),
                                )
                                .unwrap()
                                .residual_accepted
                        }
                        3 => {
                            out = lu.solve(&b).unwrap();
                            true
                        }
                        4 => {
                            out = LuFactorization::new(&m).unwrap().solve(&b).unwrap();
                            true
                        }
                        _ => {
                            p.direct(&b, &mut out);
                            true
                        }
                    };
                    // The same independent original residual and reference check
                    // is timed for all methods, including candidates already checked internally.
                    let r = residual(&p, &b, &out, &mut tmp);
                    let e = out
                        .iter()
                        .zip(&exact)
                        .map(|(x, y)| (x - y).abs())
                        .fold(0.0, f64::max);
                    accepted[mode] &= ok && r + 1e-13 <= tol;
                    errors[mode] = errors[mode].max(e);
                    residuals[mode] = residuals[mode].max(r);
                    black_box(&out);
                    calls += 1;
                }
                times[mode].push(start.elapsed().as_secs_f64() / calls as f64);
            }
        }
        let names = [
            "conditioned_reused",
            "conditioned_fresh_setup",
            "expanded_reused",
            "dense_lu_reused",
            "dense_lu_fresh",
            "nilpotent_direct",
        ];
        for mode in 0..6 {
            println!(
                "{}",
                json!({"schema_version":1,"kind":"conditioned_chart_kernel","dimension":n,"method":names[mode],"seconds":times[mode],"accepted":accepted[mode],"max_reference_error":errors[mode],"max_original_residual":residuals[mode],"residual_tolerance":tol,"setup_work":setup,"prepared_total_defect_upper":plan.total_defect_upper(),"timing_scope":"11 cyclic >=2ms groups; one process; matrix and RHS prebuilt for all; final residual and reference diagnostics timed for every method; fresh includes declaration validation/allocation, plan/workspace or factorization, reused excludes them","structure":"C=u*v^T, u=ones, v=alternating +/-1/n, v^T*u=0 exactly, B=-100I+100C; native C action O(n); n is even binary power"})
            );
        }
    }
}
