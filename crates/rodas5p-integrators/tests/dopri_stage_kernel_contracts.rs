use rodas5p_core::WorkCounters;
use rodas5p_integrators::{Dopri5Workspace, DopriStageKernel};
#[test]
fn component_stage_recurrence_reduces_buffer_writes() {
    let rhs = |t: f64, y: &[f64], out: &mut [f64]| {
        for (i, (v, x)) in out.iter_mut().zip(y).enumerate() {
            *v = -(i as f64 + 1.) * x + t;
        }
        Ok(())
    };
    let y = [0.125, -0.25, 0.5, 0.75];
    let mut a = [0.; 4];
    let mut b = [0.; 4];
    let mut e = [0.; 4];
    let mut f = [0.; 4];
    let mut old = Dopri5Workspace::new_with_kernel(4, DopriStageKernel::Rows).unwrap();
    let mut new = Dopri5Workspace::new_with_kernel(4, DopriStageKernel::Components).unwrap();
    let mut c = WorkCounters::default();
    let mut d = WorkCounters::default();
    old.step_into(&rhs, 0., &y, 0.001, &mut a, &mut e, &mut c)
        .unwrap();
    new.step_into(&rhs, 0., &y, 0.001, &mut b, &mut f, &mut d)
        .unwrap();
    assert_eq!(a.map(f64::to_bits), b.map(f64::to_bits));
    assert_eq!(e.map(f64::to_bits), f.map(f64::to_bits));
    assert_eq!(c.rhs_calls, d.rhs_calls);
    println!(
        "DOPRI_RED_GREEN {:?} {:?}",
        old.stage_work(),
        new.stage_work()
    );
    assert!(new.stage_work().state_writes < old.stage_work().state_writes);
    assert!(new.stage_work().error_writes < old.stage_work().error_writes);
}
