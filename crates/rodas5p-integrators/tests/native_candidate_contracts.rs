use std::collections::BTreeSet;

use rodas5p_integrators::{
    CandidateCatalog, CandidateFamily, CandidateStatus, run_native_integrator_gates,
};

#[test]
fn catalog_registers_four_native_implicit_anchors_and_keeps_production_variants_deferred() {
    let catalog = CandidateCatalog::research_default().unwrap();
    let executable: BTreeSet<_> = catalog
        .entries()
        .iter()
        .filter(|candidate| matches!(candidate.status(), CandidateStatus::Executable))
        .map(|candidate| candidate.id().to_string())
        .collect();
    for id in [
        "bdf1-fixed",
        "bdf2-fixed",
        "radau-iia1-fixed",
        "radau-iia3-fixed",
    ] {
        assert!(executable.contains(id), "missing {id}");
    }
    for (family, id) in [
        (CandidateFamily::Bdf, "bdf-variable-order"),
        (CandidateFamily::RadauIrk, "radau-adaptive"),
    ] {
        assert!(catalog.entries().iter().any(|candidate| {
            candidate.family() == family
                && candidate.id() == id
                && matches!(candidate.status(), CandidateStatus::Deferred { .. })
        }));
    }
}

#[test]
fn native_integrator_gate_executes_order_stiff_and_mass_checks_deterministically() {
    let first = run_native_integrator_gates().unwrap();
    let second = run_native_integrator_gates().unwrap();
    assert_eq!(first, second);
    assert_eq!(first.rows.len(), 4);
    for row in &first.rows {
        assert!(
            row.order_pass,
            "{} order {:?}",
            row.candidate_id, row.observed_order
        );
        assert!(
            row.stiff_pass,
            "{} stiff {}",
            row.candidate_id, row.stiff_amplification
        );
        assert!(
            row.mass_pass,
            "{} mass {}",
            row.candidate_id, row.mass_error_l2
        );
        assert_eq!(row.failures, 0);
    }
}
