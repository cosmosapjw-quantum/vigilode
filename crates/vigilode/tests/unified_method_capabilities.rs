//! U06 contracts for the pure unified method capability registry.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use serde_json::json;
use vigilode::{
    CapabilitySet, DerivativeFallback, GeneralOdePolicy, Jacobian, MethodAccess, MethodAdmission,
    MethodPhase, MethodQualification, MethodRegistry, MethodRequest, Problem, ProblemRegime,
    RegistryRefusal,
};

fn set_from_bits(b: u32) -> CapabilitySet {
    CapabilitySet {
        provided_jvp: b & 1 != 0,
        finite_difference: b & 2 != 0,
        automatic_differentiation: b & 4 != 0,
        explicit_jacobian: b & 8 != 0,
        dense_matrix: b & 16 != 0,
        newton: b & 32 != 0,
    }
}

/// Independent specification table (restated from the U06 contract, not from the code):
/// (method, qualified, [(label, per_step bits, startup bits, mass, events, dense)]).
/// bits: 1 jvp, 2 fd, 4 ad, 8 explicit J, 16 dense, 32 newton.
type SpecVariant = (&'static str, u32, u32, bool, bool, bool);
type SpecMethod = (&'static str, bool, Vec<SpecVariant>);

fn spec() -> Vec<SpecMethod> {
    vec![
        (
            "dopri5",
            true,
            vec![("rhs-only", 0, 0, false, false, false)],
        ),
        (
            "rodas5p",
            true,
            vec![
                ("explicit-jacobian-dense", 8 | 16, 0, true, true, true),
                ("finite-difference-dense", 2 | 16, 0, true, true, true),
                ("jvp-matrix-free", 1, 0, false, false, false),
            ],
        ),
        (
            "ppsw1-6b-schmitt-weiner-singly-implicit-peer-w",
            true,
            vec![
                (
                    "explicit-jacobian-dense",
                    8 | 16,
                    8 | 16 | 32,
                    false,
                    false,
                    false,
                ),
                (
                    "jvp-columns-dense",
                    1 | 16,
                    1 | 16 | 32,
                    false,
                    false,
                    false,
                ),
            ],
        ),
        (
            "ppsw1-6b-matrix-free-common-w",
            false,
            vec![
                (
                    "jvp-steps-explicit-jacobian-startup",
                    1,
                    8 | 16 | 32,
                    false,
                    false,
                    false,
                ),
                (
                    "jvp-steps-jvp-columns-startup",
                    1,
                    1 | 16 | 32,
                    false,
                    false,
                    false,
                ),
            ],
        ),
        (
            "bdf1-bdf2-variable",
            false,
            vec![
                (
                    "explicit-jacobian-dense-newton",
                    8 | 16 | 32,
                    8 | 16 | 32,
                    false,
                    false,
                    false,
                ),
                (
                    "jvp-columns-dense-newton",
                    1 | 16 | 32,
                    1 | 16 | 32,
                    false,
                    false,
                    false,
                ),
            ],
        ),
        (
            "radau-iia3",
            false,
            vec![
                (
                    "explicit-jacobian-dense-newton",
                    8 | 16 | 32,
                    0,
                    false,
                    false,
                    false,
                ),
                (
                    "jvp-columns-dense-newton",
                    1 | 16 | 32,
                    0,
                    false,
                    false,
                    false,
                ),
            ],
        ),
    ]
}

fn expected(
    allowed: u32,
    feat: (bool, bool, bool),
    admit_unq: bool,
) -> Vec<(&'static str, Option<&'static str>)> {
    spec()
        .into_iter()
        .map(|(id, qualified, vars)| {
            if !qualified && !admit_unq {
                return (id, None);
            }
            let v = vars.iter().find(|(_, ps, st, m, e, d)| {
                ps & !allowed == 0
                    && st & !allowed == 0
                    && (!feat.0 || *m)
                    && (!feat.1 || *e)
                    && (!feat.2 || *d)
            });
            (id, v.map(|x| x.0))
        })
        .collect()
}

fn request(allowed: CapabilitySet, feat: (bool, bool, bool), admit: bool) -> MethodRequest {
    MethodRequest {
        allowed,
        needs_mass: feat.0,
        needs_events: feat.1,
        needs_dense_output: feat.2,
        admit_unqualified: admit,
    }
}

#[test]
fn r1_exhaustive_permission_and_feature_combinations_match_the_spec_table() {
    let mut checked = 0usize;
    for bits in 0..64u32 {
        for f in 0..8u32 {
            for admit in [false, true] {
                let feat = (f & 1 != 0, f & 2 != 0, f & 4 != 0);
                let got: Vec<(&str, Option<&str>)> =
                    MethodRegistry::query(&request(set_from_bits(bits), feat, admit))
                        .into_iter()
                        .map(|a| match a {
                            MethodAdmission::Admitted {
                                method_id, variant, ..
                            } => (method_id, Some(variant)),
                            MethodAdmission::Refused { method_id, reasons } => {
                                assert!(!reasons.is_empty());
                                (method_id, None)
                            }
                        })
                        .collect();
                assert_eq!(
                    got,
                    expected(bits, feat, admit),
                    "bits={bits} f={f} admit={admit}"
                );
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 1024);
    println!(
        "QUAL {}",
        json!({"test": "r1_exhaustive", "data": {"requests": checked}})
    );
}

#[test]
fn r2_request_construction_and_query_call_no_user_code() {
    let calls = Arc::new(AtomicUsize::new(0));
    let rhs_calls = calls.clone();
    let make = move || {
        let c = rhs_calls.clone();
        Problem::new(3, move |_t, _y, _dy| {
            c.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    };
    let (c2, c3) = (calls.clone(), calls.clone());
    let dense = make().jacobian(Jacobian::dense(move |_t, _y, _j| {
        c2.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }));
    let jvp = make().jacobian(Jacobian::jvp(move |_t, _y, _v, _o| {
        c3.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }));
    for p in [make(), dense, jvp] {
        for derivatives in [
            DerivativeFallback::Forbidden,
            DerivativeFallback::ProvidedOnly,
            DerivativeFallback::AllowFiniteDifference,
        ] {
            for allow_matrix_free in [false, true] {
                let policy = GeneralOdePolicy {
                    regime: ProblemRegime::Stiff,
                    derivatives,
                    max_dense_dimension: 64,
                    allow_matrix_free,
                };
                let allowed = CapabilitySet::from_problem_and_policy(&p, &policy);
                assert!(!allowed.newton, "Newton is never implied by the policy");
                let _ = MethodRegistry::query(&request(allowed, (false, false, false), true));
            }
        }
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn r3_policy_mapping_is_a_set_and_newton_methods_stay_refused() {
    let jvp =
        Problem::new(2, |_t, _y, _dy| Ok(())).jacobian(Jacobian::jvp(|_t, _y, _v, _o| Ok(())));
    let policy = |d, mf| GeneralOdePolicy {
        regime: ProblemRegime::Stiff,
        derivatives: d,
        max_dense_dimension: 64,
        allow_matrix_free: mf,
    };
    let s = CapabilitySet::from_problem_and_policy(
        &jvp,
        &policy(DerivativeFallback::ProvidedOnly, true),
    );
    assert!(
        s.provided_jvp
            && !s.finite_difference
            && !s.explicit_jacobian
            && s.dense_matrix
            && !s.newton
    );
    let s = CapabilitySet::from_problem_and_policy(
        &jvp,
        &policy(DerivativeFallback::ProvidedOnly, false),
    );
    assert!(!s.provided_jvp, "JVP needs allow_matrix_free");
    let s =
        CapabilitySet::from_problem_and_policy(&jvp, &policy(DerivativeFallback::Forbidden, true));
    assert_eq!(
        s,
        CapabilitySet {
            dense_matrix: true,
            ..CapabilitySet::NONE
        }
    );
    let big = Problem::new(100, |_t, _y, _dy| Ok(()));
    let s = CapabilitySet::from_problem_and_policy(
        &big,
        &policy(DerivativeFallback::AllowFiniteDifference, false),
    );
    assert!(s.finite_difference && !s.dense_matrix);
    // from a policy, PPSW1 (its starter needs Newton) is refused at the startup phase
    let s = CapabilitySet::from_problem_and_policy(
        &jvp,
        &policy(DerivativeFallback::ProvidedOnly, true),
    );
    let out = MethodRegistry::query(&request(s, (false, false, false), true));
    let reasons = out
        .iter()
        .find_map(|a| match a {
            MethodAdmission::Refused { method_id, reasons }
                if method_id.starts_with("ppsw1-6b-schmitt") =>
            {
                Some(reasons.clone())
            }
            _ => None,
        })
        .expect("ppsw1 refused");
    assert!(reasons.iter().any(|r| matches!(
        r,
        RegistryRefusal::Permission {
            phase: MethodPhase::Startup,
            ..
        }
    )));
}

#[test]
fn r4_serde_round_trip_and_unknown_or_missing_fields_are_refused() {
    let req = request(set_from_bits(1 | 16 | 32), (false, true, false), true);
    let text = serde_json::to_string(&req).unwrap();
    assert_eq!(serde_json::from_str::<MethodRequest>(&text).unwrap(), req);
    let base: serde_json::Value = serde_json::from_str(&text).unwrap();
    let mut v = base.clone();
    v["extra"] = json!(true);
    assert!(serde_json::from_value::<MethodRequest>(v).is_err());
    let mut v = base.clone();
    v.as_object_mut().unwrap().remove("needs_mass");
    assert!(serde_json::from_value::<MethodRequest>(v).is_err());
    let mut v = base.clone();
    v["allowed"]["quantum_oracle"] = json!(true);
    assert!(serde_json::from_value::<MethodRequest>(v).is_err());
    let mut v = base;
    v["allowed"].as_object_mut().unwrap().remove("newton");
    assert!(serde_json::from_value::<MethodRequest>(v).is_err());
    let d = serde_json::to_value(MethodRegistry::descriptors()).unwrap();
    println!("QUAL {}", json!({"test": "r4_descriptors", "data": d}));
}

#[test]
fn r5_routes_name_real_exports_and_ids_match_the_code() {
    // Compile-time resolution of every route string.
    let _ = vigilode::Dopri5::solve;
    let _ = vigilode::Rodas5p::solve;
    let _ = rodas5p_integrators::ppsw1::Session::new;
    let _ = rodas5p_integrators::ppsw1::linear_backend::LinearBackend::Dense;
    let _ = rodas5p_integrators::integrate_bdf_adaptive_observed;
    let _ = rodas5p_integrators::integrate_radau_adaptive_observed;
    let routes: Vec<&str> = MethodRegistry::descriptors()
        .iter()
        .map(|d| d.route)
        .collect();
    assert_eq!(
        routes,
        vec![
            "vigilode::Dopri5::solve",
            "vigilode::Rodas5p::solve",
            "rodas5p_integrators::ppsw1::Session",
            "rodas5p_integrators::ppsw1::linear_backend::LinearBackend",
            "rodas5p_integrators::integrate_bdf_adaptive_observed",
            "rodas5p_integrators::integrate_radau_adaptive_observed",
        ]
    );
    let ids: Vec<&str> = MethodRegistry::descriptors()
        .iter()
        .map(|d| d.method_id)
        .collect();
    assert!(ids.contains(&rodas5p_integrators::ppsw1::METHOD_ID));
    for d in MethodRegistry::descriptors() {
        if d.access == MethodAccess::GeneralOdeSelectable {
            assert_eq!(
                d.qualification,
                MethodQualification::Production,
                "{}",
                d.method_id
            );
        }
        assert!(
            d.variants.iter().all(|v| v.fallback.is_none()),
            "no automatic fallback exists"
        );
    }
}

#[test]
fn r6_peer_mass_and_events_are_refused_before_work() {
    for feat in [(true, false, false), (false, true, false)] {
        for a in MethodRegistry::query(&request(set_from_bits(63), feat, true)) {
            match a {
                MethodAdmission::Refused { method_id, reasons }
                    if method_id.starts_with("ppsw1") =>
                {
                    assert!(reasons.iter().any(|r| matches!(
                        r,
                        RegistryRefusal::MassUnsupported | RegistryRefusal::EventsUnsupported
                    )));
                }
                MethodAdmission::Admitted { method_id, .. } => {
                    assert!(!method_id.starts_with("ppsw1"), "ppsw1 admitted");
                }
                _ => {}
            }
        }
    }
}
