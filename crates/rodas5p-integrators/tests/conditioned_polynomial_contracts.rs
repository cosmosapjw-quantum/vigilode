use std::alloc::{GlobalAlloc, Layout, System};
struct CountingAlloc;
thread_local! {
    static COUNT_ENABLED: Cell<bool> = const { Cell::new(false) };
    static ALLOCATION_COUNT: Cell<usize> = const { Cell::new(0) };
}
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNT_ENABLED.with(|enabled| {
            if enabled.get() {
                ALLOCATION_COUNT.with(|n| n.set(n.get() + 1));
            }
        });
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        COUNT_ENABLED.with(|enabled| {
            if enabled.get() {
                ALLOCATION_COUNT.with(|n| n.set(n.get() + 1));
            }
        });
        unsafe { System.realloc(ptr, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAlloc = CountingAlloc;
use rodas5p_integrators::conditioned_polynomial::*;
use rodas5p_integrators::polynomial_resolvent::{
    DeclaredPolynomialResolvent, PolynomialPhysicalNorm as Norm,
    PolynomialRefinementPolicy as Policy, PolynomialRefinementStop as Stop,
    PolynomialResolventError as Error,
};
use std::cell::Cell;

struct MatrixProvider {
    d: ChartDeclaration,
    c: Vec<f64>,
    calls: Cell<usize>,
    partial: bool,
    fail_original: bool,
}
impl DeclaredChartOperator for MatrixProvider {
    fn declaration(&self) -> &ChartDeclaration {
        &self.d
    }
    fn apply_chart(&self, x: &[f64], out: &mut [f64]) -> Result<(), Error> {
        self.calls.set(self.calls.get() + 1);
        if self.partial {
            out[0] = 0.0;
            return Ok(());
        }
        for (i, o) in out.iter_mut().enumerate() {
            *o = self.c[i * x.len()..(i + 1) * x.len()]
                .iter()
                .zip(x)
                .map(|(a, b)| a * b)
                .sum();
        }
        Ok(())
    }
    fn apply_original(&self, x: &[f64], out: &mut [f64]) -> Result<(), Error> {
        self.calls.set(self.calls.get() + 1);
        if self.fail_original {
            return Err(Error::OperatorFailure);
        }
        for (i, o) in out.iter_mut().enumerate() {
            let cx: f64 = self.c[i * x.len()..(i + 1) * x.len()]
                .iter()
                .zip(x)
                .map(|(a, b)| a * b)
                .sum();
            *o = self.d.scale().mul_add(cx, self.d.center() * x[i]);
        }
        Ok(())
    }
}
fn id(n: usize) -> ChartOperatorIdentity {
    ChartOperatorIdentity {
        operator_key: 11,
        dimension: n,
        model_epoch: 3,
        provider_epoch: 5,
        operator_scale_bits: 0.125_f64.to_bits(),
        physical_norm: Norm::Infinity,
    }
}
fn provider(
    center: f64,
    scale: f64,
    c: Vec<f64>,
    q: &[f64],
    relation: f64,
    norm: f64,
) -> MatrixProvider {
    let n = (c.len() as f64).sqrt() as usize;
    MatrixProvider {
        d: ChartDeclaration::new(id(n), center, scale, q, relation, norm, 7).unwrap(),
        c,
        calls: Cell::new(0),
        partial: false,
        fail_original: false,
    }
}
fn apply(
    p: &MatrixProvider,
    plan: &ConditionedPolynomialPlan,
    rhs: &[f64],
    out: &mut [f64],
    policy: Policy,
    work: &mut ChartWork,
) -> Result<ChartReport, Error> {
    plan.apply(
        p,
        rhs,
        out,
        &mut ChartWorkspace::new(rhs.len()),
        policy,
        1e-12,
        1e-15,
        work,
    )
}

#[test]
fn shifted_jordan_chart_avoids_unrepresentable_expanded_coefficients() {
    let n = 4;
    let center = -1e100;
    let scale = 1e100;
    let mut c = vec![0.0; n * n];
    for i in 0..n - 1 {
        c[i * n + i + 1] = 0.25;
    }
    let p = provider(center, scale, c, &[0.0, 0.0, 0.0, 0.0, 1.0], 0.0, 0.25);
    assert!(((-center).powi(4)).is_infinite());
    assert!(DeclaredPolynomialResolvent::new(&[f64::INFINITY, 1.0], 0.0, 1.25e100, 1e-12).is_err());
    let mut w = ChartWork::default();
    let plan = ConditionedPolynomialPlan::prepare(&p.d, 1e-12, &mut w).unwrap();
    assert_eq!(plan.degree(), 4);
    assert!(plan.total_defect_upper() < 1e-12);
    let rhs = [1.0, 2.0, 3.0, 4.0];
    let mut out = [99.0; 4];
    let r = apply(&p, &plan, &rhs, &mut out, Policy::default(), &mut w).unwrap();
    assert!(r.accepted);
    let a = 1.0 - center;
    let mut exact = [0.0; 4];
    for i in (0..4).rev() {
        exact[i] = (rhs[i]
            + if i < 3 {
                scale * 0.25 * exact[i + 1]
            } else {
                0.0
            })
            / a;
    }
    for (x, y) in out.iter().zip(exact) {
        assert!((x - y).abs() < 2e-114);
    }
    assert_eq!(
        (w.chart_actions_attempted, w.original_actions_attempted),
        (3, 1)
    );
}

#[test]
fn coalescent_roots_keep_nonnormal_nilpotent_limit() {
    for delta in [0.0, 2_f64.powi(-24), 0.125] {
        let p = provider(
            -2.0,
            4.0,
            vec![delta, 0.5, 0.0, -delta],
            &[-delta * delta, 0.0, 1.0],
            0.0,
            0.5 + delta,
        );
        let mut w = ChartWork::default();
        let plan = ConditionedPolynomialPlan::prepare(&p.d, 1e-12, &mut w).unwrap();
        let mut out = [0.0; 2];
        assert!(
            apply(&p, &plan, &[1.0, 2.0], &mut out, Policy::default(), &mut w)
                .unwrap()
                .accepted
        );
        let y = 2.0 / (3.0 + 4.0 * delta);
        let x = (1.0 + 2.0 * y) / (3.0 - 4.0 * delta);
        assert!((out[0] - x).abs() < 2e-14);
        assert!((out[1] - y).abs() < 2e-14);
        assert_eq!(plan.degree(), 2);
    }
}

#[test]
fn coefficient_rounding_is_included_in_prepared_defect() {
    let p = provider(
        -0.1,
        0.3,
        vec![0.0, 0.2, 0.0, 0.0],
        &[0.0, 0.0, 1.0],
        0.0,
        0.2,
    );
    let mut w = ChartWork::default();
    let plan = ConditionedPolynomialPlan::prepare(&p.d, 1e-12, &mut w).unwrap();
    assert!(plan.preparation_defect_upper() > 0.0);
    assert!(plan.total_defect_upper() >= plan.preparation_defect_upper());
    let mut out = [0.0; 2];
    assert!(
        apply(&p, &plan, &[1.0, 1.0], &mut out, Policy::default(), &mut w)
            .unwrap()
            .accepted
    );
}

#[test]
fn approximate_relation_refines_and_counts_paid_actions() {
    // C=0.01 I, declared q(C)=C^2 with exact infinity bound 0.0001.
    let p = provider(
        -1.0,
        2.0,
        vec![0.01, 0.0, 0.0, 0.01],
        &[0.0, 0.0, 1.0],
        0.0001_f64.next_up(),
        0.01,
    );
    let mut w = ChartWork::default();
    let plan = ConditionedPolynomialPlan::prepare(&p.d, 1e-12, &mut w).unwrap();
    let mut out = [11.0; 2];
    let policy = Policy {
        max_corrections: 8,
        ..Policy::default()
    };
    let r = apply(&p, &plan, &[1.0, 2.0], &mut out, policy, &mut w).unwrap();
    assert!(r.accepted);
    assert!(r.corrections >= 2);
    assert!((out[0] - 1.0 / 1.98).abs() < 1e-12);
    assert_eq!(
        w.chart_actions_attempted + w.original_actions_attempted,
        2 * (r.corrections as u64 + 1)
    );
}

#[test]
fn every_identity_and_witness_change_refuses_before_actions() {
    let p = provider(-1.0, 1.0, vec![0.0; 4], &[0.0, 0.0, 1.0], 0.0, 0.0);
    let mut w = ChartWork::default();
    let plan = ConditionedPolynomialPlan::prepare(&p.d, 1e-12, &mut w).unwrap();
    let mut changes = Vec::new();
    for field in 0..6 {
        let mut identity = id(2);
        match field {
            0 => identity.operator_key += 1,
            1 => identity.dimension = 3,
            2 => identity.model_epoch += 1,
            3 => identity.provider_epoch += 1,
            4 => identity.operator_scale_bits = 0.25_f64.to_bits(),
            _ => identity.operator_scale_bits = (-0.125_f64).to_bits(),
        }
        changes.push(
            ChartDeclaration::new(identity, -1.0, 1.0, &[0.0, 0.0, 1.0], 0.0, 0.0, 7).unwrap(),
        );
    }
    for (center, scale, q, relation, norm, evidence) in [
        (-2.0, 1.0, vec![0.0, 0.0, 1.0], 0.0, 0.0, 7),
        (-1.0, 2.0, vec![0.0, 0.0, 1.0], 0.0, 0.0, 7),
        (-1.0, 1.0, vec![0.0, 1.0], 0.0, 0.0, 7),
        (-1.0, 1.0, vec![0.0, 0.0, 2.0], 0.0, 0.0, 7),
        (-1.0, 1.0, vec![0.0, 0.0, 1.0], 1e-9, 0.0, 7),
        (-1.0, 1.0, vec![0.0, 0.0, 1.0], 0.0, 1e-9, 7),
        (-1.0, 1.0, vec![0.0, 0.0, 1.0], 0.0, 0.0, 8),
    ] {
        changes.push(
            ChartDeclaration::new(id(2), center, scale, &q, relation, norm, evidence).unwrap(),
        );
    }
    for declaration in changes {
        let stale = MatrixProvider {
            d: declaration,
            c: vec![0.0; 4],
            calls: Cell::new(0),
            partial: false,
            fail_original: false,
        };
        let mut out = [17.0; 2];
        assert_eq!(
            apply(
                &stale,
                &plan,
                &[1.0; 2],
                &mut out,
                Policy::default(),
                &mut w
            )
            .unwrap_err(),
            Error::WitnessMismatch
        );
        assert_eq!(stale.calls.get(), 0);
        assert_eq!(out, [17.0; 2]);
    }
    assert_eq!(w.chart_actions_attempted + w.original_actions_attempted, 0);
    assert_eq!(w.applications_refused, 13);
}

#[test]
fn singular_pole_refuses_and_near_pole_retains_residual_check() {
    let p = provider(1.0, 1.0, vec![0.0], &[0.0, 1.0], 0.0, 0.0);
    let mut w = ChartWork::default();
    assert_eq!(
        ConditionedPolynomialPlan::prepare(&p.d, 1e-12, &mut w).unwrap_err(),
        Error::DenominatorMargin
    );
    let p = provider(1.0 - 2_f64.powi(-20), 1.0, vec![0.0], &[0.0, 1.0], 0.0, 0.0);
    let plan = ConditionedPolynomialPlan::prepare(&p.d, 1e-12, &mut w).unwrap();
    let mut out = [0.0];
    let r = apply(&p, &plan, &[1.0], &mut out, Policy::default(), &mut w).unwrap();
    assert!(r.accepted);
    assert_eq!(out[0], 2_f64.powi(20));
    assert!(plan.inverse_norm_upper() >= 2_f64.powi(20));
}

#[test]
fn callback_failure_and_partial_write_preserve_output_and_attempt_counts() {
    for partial in [false, true] {
        let mut p = provider(-1.0, 1.0, vec![0.0; 4], &[0.0, 0.0, 1.0], 0.0, 0.0);
        p.partial = partial;
        p.fail_original = !partial;
        let mut w = ChartWork::default();
        let plan = ConditionedPolynomialPlan::prepare(&p.d, 1e-12, &mut w).unwrap();
        let mut out = [31.0; 2];
        let result = apply(&p, &plan, &[1.0; 2], &mut out, Policy::default(), &mut w);
        assert_eq!(
            result.unwrap_err(),
            if partial {
                Error::NonFinite
            } else {
                Error::OperatorFailure
            }
        );
        assert_eq!(out, [31.0; 2]);
        assert_eq!(w.chart_actions_attempted, 1);
        assert_eq!(w.original_actions_attempted, if partial { 0 } else { 1 });
    }
}

#[test]
fn budgets_and_residual_refusal_remain_transactional() {
    let p = provider(
        -1.0,
        2.0,
        vec![0.1],
        &[0.0, 0.0, 1.0],
        0.010000000000000002,
        0.1,
    );
    let mut w = ChartWork::default();
    let plan = ConditionedPolynomialPlan::prepare(&p.d, 1e-12, &mut w).unwrap();
    let mut policy = Policy {
        max_operator_actions: 1,
        ..Policy::default()
    };
    let mut out = [41.0];
    let r = apply(&p, &plan, &[1.0], &mut out, policy, &mut w).unwrap();
    assert_eq!(r.stop, Stop::ActionBudget);
    assert_eq!(p.calls.get(), 0);
    policy.max_operator_actions = 100;
    policy.max_corrections = 0;
    let r = apply(&p, &plan, &[1.0], &mut out, policy, &mut w).unwrap();
    assert_eq!(r.stop, Stop::CorrectionBudget);
    assert_eq!(out, [41.0]);
    policy.maximum_relation_ratio = 0.001;
    let calls = p.calls.get();
    let r = apply(&p, &plan, &[1.0], &mut out, policy, &mut w).unwrap();
    assert_eq!(r.stop, Stop::RelationTooWeak);
    assert_eq!(p.calls.get(), calls);
}

#[test]
fn invalid_norm_and_nonfinite_metadata_are_refused() {
    let mut identity = id(1);
    identity.physical_norm = Norm::Other(1);
    assert!(ChartDeclaration::new(identity, 0.0, 1.0, &[0.0, 1.0], 0.0, 1.0, 1).is_err());
    for (center, scale, rel, norm, evidence) in [
        (0.0, 0.0, 0.0, 1.0, 1),
        (f64::NAN, 1.0, 0.0, 1.0, 1),
        (0.0, 1.0, -1.0, 1.0, 1),
        (0.0, 1.0, 0.0, 1.0, 0),
    ] {
        assert!(
            ChartDeclaration::new(id(1), center, scale, &[0.0, 1.0], rel, norm, evidence).is_err()
        );
    }
}

#[test]
fn prepared_chart_application_has_no_internal_heap_allocations() {
    let p = provider(
        -1.0,
        2.0,
        vec![0.01, 0.0, 0.0, 0.01],
        &[0.0, 0.0, 1.0],
        0.0001_f64.next_up(),
        0.01,
    );
    let mut w = ChartWork::default();
    let plan = ConditionedPolynomialPlan::prepare(&p.d, 1e-12, &mut w).unwrap();
    let mut ws = ChartWorkspace::new(2);
    let mut out = [0.0; 2];
    ALLOCATION_COUNT.with(|n| n.set(0));
    COUNT_ENABLED.with(|on| on.set(true));
    let result = plan.apply(
        &p,
        &[1.0, 2.0],
        &mut out,
        &mut ws,
        Policy::default(),
        1e-12,
        1e-15,
        &mut w,
    );
    COUNT_ENABLED.with(|on| on.set(false));
    assert!(result.unwrap().accepted);
    assert_eq!(ALLOCATION_COUNT.with(Cell::get), 0);
}

#[test]
fn an_ill_conditioned_affine_chart_refuses_without_rejecting_the_original_operator() {
    // Shift/scale compresses the target against the C=1 root. The original
    // operator remains invertible; refusal belongs to this chart only.
    let p = provider(-1e100, 1e100, vec![0.5; 4], &[0.0, -1.0, 1.0], 0.0, 1.0);
    let mut w = ChartWork::default();
    assert_eq!(
        ConditionedPolynomialPlan::prepare(&p.d, 1e-12, &mut w).unwrap_err(),
        Error::DenominatorMargin
    );
    assert_eq!(p.calls.get(), 0);
    let old = DeclaredPolynomialResolvent::new(&[0.0, 1e100, 1.0], 0.0, 2e100, 1e-12).unwrap();
    let mut out = [0.0; 2];
    let mut ws = rodas5p_integrators::polynomial_resolvent::PolynomialResolventWorkspace::new(2);
    let report = old
        .apply_checked(&[1.0; 2], &mut out, &mut ws, 1e-12, 1e-15, |x, y| {
            p.apply_original(x, y)
        })
        .unwrap();
    assert!(report.residual_accepted);
    assert_eq!(out, [1.0; 2]);
}
