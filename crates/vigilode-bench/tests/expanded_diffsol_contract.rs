use vigilode_bench::{
    arms::{Arm, RunStatus, run},
    model::{Kind, Model},
};
#[test]
fn actual_builtin_methods_are_present_and_solve_the_shared_grid() {
    let model = Model::new(Kind::LinearExact, 2).unwrap();
    let grid: Vec<_> = (0..=100).map(|i| 2. * i as f64 / 100.).collect();
    for name in [
        "diffsol-trbdf2-dense",
        "diffsol-trbdf2-sparse",
        "diffsol-esdirk34-dense",
        "diffsol-esdirk34-sparse",
        "diffsol-tsit45",
    ] {
        let a: Arm = serde_json::from_value(serde_json::json!(name))
            .unwrap_or_else(|e| panic!("{name} unavailable: {e}"));
        let r = run(a, &model, &grid, 1e-8, 1e-10);
        assert_eq!(r.status, RunStatus::Success, "{name}: {:?}", r.status);
        assert_eq!(r.states.len(), grid.len());
        assert_eq!(r.states[0], model.y0());
        for (&t, y) in grid.iter().zip(&r.states) {
            for (&a, b) in y.iter().zip(model.exact(t).unwrap()) {
                assert!((a - b).abs() < 1e-6, "{name} t={t}");
            }
        }
    }
}
