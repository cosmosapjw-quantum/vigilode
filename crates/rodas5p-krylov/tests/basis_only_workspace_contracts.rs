//! Live-storage refinement contracts: retain numeric/callback semantics while
//! omitting the unused direction pool of GMRES-into.

use std::sync::Mutex;

use rodas5p_core::{
    CoreResult, DenseMatrix, IdentityPreconditioner, LinearOperator, Preconditioner,
    PreconditionerSide, WorkCounters,
};
use rodas5p_krylov::{
    GmresCapacity, GmresConfig, GmresMonitorDecision, GmresWorkspace, solve_gmres_into,
    solve_gmres_into_right_monitored, solve_gmres_with_workspace_and_residual_scale,
};

struct RecordedOperator {
    matrix: DenseMatrix,
    calls: Mutex<Vec<Vec<u64>>>,
}

impl RecordedOperator {
    fn new(n: usize) -> Self {
        let mut matrix = DenseMatrix::zeros(n, n);
        for i in 0..n {
            matrix[(i, i)] = 3.0 + i as f64 / 8.0;
            if i + 1 < n {
                matrix[(i, i + 1)] = 0.25;
            }
        }
        Self {
            matrix,
            calls: Mutex::default(),
        }
    }

    fn take_calls(&self) -> Vec<Vec<u64>> {
        std::mem::take(&mut *self.calls.lock().unwrap())
    }
}

impl LinearOperator for RecordedOperator {
    fn dimension(&self) -> usize {
        self.matrix.nrows()
    }

    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        self.calls.lock().unwrap().push(bits(x));
        self.matrix.matvec_into(x, y)
    }
}

struct RightIdentity(usize);

impl Preconditioner for RightIdentity {
    fn dimension(&self) -> usize {
        self.0
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        y.copy_from_slice(x);
        Ok(())
    }
    fn side(&self) -> PreconditionerSide {
        PreconditionerSide::Right
    }
}

fn bits(x: &[f64]) -> Vec<u64> {
    x.iter().map(|v| v.to_bits()).collect()
}

fn config(n: usize) -> GmresConfig {
    GmresConfig {
        restart: n,
        max_arnoldi: 4 * n,
        rtol: 1.0e-11,
        atol: 1.0e-13,
    }
}

#[test]
fn cold_into_omits_direction_storage_but_full_reserve_stays_compatible() {
    let n = 8;
    let op = RecordedOperator::new(n);
    let pc = IdentityPreconditioner::new(n);
    let rhs = vec![1.0; n];
    let mut cold = GmresWorkspace::default();
    let mut reserved = GmresWorkspace::default();
    reserved.reserve(n, n).unwrap();
    let mut outputs = [vec![0.0; n], vec![0.0; n]];
    let mut counters = [WorkCounters::default(), WorkCounters::default()];
    let a = solve_gmres_into(
        &op,
        &pc,
        &rhs,
        None,
        &config(n),
        None,
        &mut outputs[0],
        &mut cold,
        GmresCapacity::unbounded(),
        &mut counters[0],
    )
    .unwrap();
    let first_calls = op.take_calls();
    let b = solve_gmres_into(
        &op,
        &pc,
        &rhs,
        None,
        &config(n),
        None,
        &mut outputs[1],
        &mut reserved,
        GmresCapacity::unbounded(),
        &mut counters[1],
    )
    .unwrap();
    assert_eq!(bits(&outputs[0]), bits(&outputs[1]));
    assert_eq!(counters[0], counters[1]);
    assert_eq!(first_calls, op.take_calls());
    assert_eq!(a.residual_norm.to_bits(), b.residual_norm.to_bits());
    assert!(a.workspace_grew);
    assert!(!b.workspace_grew);
    assert_eq!(reserved.capacity_f64() - cold.capacity_f64(), n * n);
    assert!(reserved.retained_bytes() >= cold.retained_bytes() + n * n * 8);
}

#[test]
fn mixed_legacy_and_into_reuse_preserves_bits_and_callback_words_after_shape_changes() {
    let mut reused = GmresWorkspace::default();
    for n in [3, 11, 2, 7, 1, 11] {
        let op = RecordedOperator::new(n);
        let pc = IdentityPreconditioner::new(n);
        let rhs: Vec<f64> = (0..n).map(|i| 1.0 + i as f64 / 16.0).collect();
        let cfg = config(n);
        let mut legacy_work = WorkCounters::default();
        let legacy = solve_gmres_with_workspace_and_residual_scale(
            &op,
            &pc,
            &rhs,
            None,
            &cfg,
            None,
            &mut reused,
            &mut legacy_work,
        )
        .unwrap();
        let legacy_calls = op.take_calls();
        let mut output = vec![f64::NAN; n];
        let mut work = WorkCounters::default();
        let got = solve_gmres_into(
            &op,
            &pc,
            &rhs,
            None,
            &cfg,
            None,
            &mut output,
            &mut reused,
            GmresCapacity::unbounded(),
            &mut work,
        )
        .unwrap();
        assert_eq!(bits(&legacy.x), bits(&output), "dimension {n}");
        assert_eq!(legacy_calls, op.take_calls(), "dimension {n}");
        assert_eq!(legacy_work, work);
        assert_eq!(legacy.residual_norm.to_bits(), got.residual_norm.to_bits());
        // Switch back again, so retained directions must be correctly sized
        // after any preceding basis-only preparation and dimension change.
        let again = solve_gmres_with_workspace_and_residual_scale(
            &op,
            &pc,
            &rhs,
            None,
            &cfg,
            None,
            &mut reused,
            &mut WorkCounters::default(),
        )
        .unwrap();
        assert_eq!(bits(&again.x), bits(&output));
    }
}

#[test]
fn right_monitored_prefixes_match_full_reserved_storage() {
    let n = 7;
    let op = RecordedOperator::new(n);
    let pc = RightIdentity(n);
    let rhs = vec![1.0; n];
    let x0 = vec![0.0; n];
    let mut cold = GmresWorkspace::default();
    let mut reserved = GmresWorkspace::default();
    reserved.reserve(n, n).unwrap();
    let mut outputs = [vec![7.0; n], vec![7.0; n]];
    let mut words = [Vec::new(), Vec::new()];
    let mut works = [WorkCounters::default(), WorkCounters::default()];
    for (index, ws) in [&mut cold, &mut reserved].into_iter().enumerate() {
        let mut monitor = |column: u64, candidate: &[f64], _: &mut WorkCounters| {
            words[index].push((column, bits(candidate)));
            Ok(if column == 3 {
                GmresMonitorDecision::Accept
            } else {
                GmresMonitorDecision::Continue
            })
        };
        let report = solve_gmres_into_right_monitored(
            &op,
            &pc,
            &rhs,
            &x0,
            &config(n),
            &mut outputs[index],
            ws,
            &mut works[index],
            &mut monitor,
        )
        .unwrap();
        assert!(report.accepted);
    }
    assert_eq!(words[0], words[1]);
    assert_eq!(bits(&outputs[0]), bits(&outputs[1]));
    assert_eq!(works[0], works[1]);
    // Full reserve also retains a larger Hessenberg-prefix capacity.
    assert!(reserved.capacity_f64() >= cold.capacity_f64() + n * n);
}
