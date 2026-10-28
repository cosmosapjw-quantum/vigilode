//! Exhaustive current feature combinations use the same refusal predicates as solve.
use std::collections::BTreeSet;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use vigilode::{
    Atol, CallbackError, Direction, FacadeRoute, InteriorError, Jacobian, Options, OutputAccuracy,
    Problem, Rodas5p, SolveError, Tier,
};

#[test]
fn query_matches_solve_for_all_384_well_formed_combinations_without_callbacks() {
    let mut checked = 0;
    let mut rows = Vec::new();
    let mut refusals = BTreeSet::new();
    let mut routes = BTreeSet::new();
    for provider in 0..4 {
        for declared_band in [false, true] {
            for mass in [false, true] {
                for dense in [false, true] {
                    for vector in [false, true] {
                        for events in [false, true] {
                            for interior in [
                                InteriorError::Off,
                                InteriorError::Enforce,
                                InteriorError::WFilteredEnforce,
                            ] {
                                let called = Arc::new(AtomicUsize::new(0));
                                let in_callback = called.clone();
                                let mut p = Problem::new(1, move |_t, _y, dy| {
                                    in_callback.fetch_add(1, Ordering::Relaxed);
                                    dy[0] = 0.0;
                                    Ok::<(), CallbackError>(())
                                })
                                .autonomous();
                                let callback_calls = called.clone();
                                p = match provider {
                                    0 => p.jacobian(Jacobian::dense(move |_t, _y, j| {
                                        callback_calls.fetch_add(1, Ordering::Relaxed);
                                        j[0] = 0.0;
                                        Ok(())
                                    })),
                                    1 => p.jacobian(Jacobian::banded(0, 0, move |_t, _y, j| {
                                        callback_calls.fetch_add(1, Ordering::Relaxed);
                                        j[0] = 0.0;
                                        Ok(())
                                    })),
                                    2 => p,
                                    3 => p.jacobian(Jacobian::jvp(move |_t, _y, _v, out| {
                                        callback_calls.fetch_add(1, Ordering::Relaxed);
                                        out[0] = 0.0;
                                        Ok(())
                                    })),
                                    _ => unreachable!(),
                                };
                                if declared_band {
                                    p = p.banded(0, 0);
                                }
                                if mass {
                                    p = p.mass(vec![1.0]);
                                }
                                if events {
                                    let event_calls = called.clone();
                                    p = p.event(
                                        move |_t, _y| {
                                            event_calls.fetch_add(1, Ordering::Relaxed);
                                            1.0
                                        },
                                        Direction::Either,
                                    );
                                }
                                let opts = Options {
                                    dense_output: dense,
                                    interior_error: interior,
                                    atol: if vector {
                                        Atol::Vector(vec![1e-9])
                                    } else {
                                        Atol::Scalar(1e-9)
                                    },
                                    tier: Tier::Performance,
                                    ..Default::default()
                                };
                                let query = Rodas5p::capabilities(&p, &opts);
                                rows.push(serde_json::json!({"provider":(["dense","banded","finite-difference","jvp"][provider]),
                                    "declared_band":declared_band,"mass":mass,"dense_output":dense,"vector_atol":vector,
                                    "events":events,"interior_error":interior,"result":query}));
                                assert_eq!(
                                    called.load(Ordering::Relaxed),
                                    0,
                                    "capability query entered user code"
                                );
                                let actual = Rodas5p::solve(&p, (0.0, 0.0), &[1.0], &opts);
                                match (query, actual) {
                                    (Err(expected), Err(SolveError::Unsupported(actual))) => {
                                        assert_eq!(expected, actual);
                                        refusals.insert(format!("{actual:?}"));
                                        assert!(
                                            serde_json::to_string(&expected)
                                                .unwrap()
                                                .starts_with('"')
                                        );
                                    }
                                    (Ok(cap), Ok(sol)) => {
                                        assert!(sol.is_success());
                                        assert_eq!(sol.stats.attempts, 0);
                                        assert_eq!(cap.tier_requested, Tier::Performance);
                                        assert_eq!(cap.tier_executed, Tier::Protected);
                                        assert!(!cap.global_error_certified);
                                        assert!(!cap.consistent_dae_initialization);
                                        assert_eq!(cap.events_are_postprocessed, events);
                                        let expected = match cap.route {
                                            FacadeRoute::DenseFast => "rodas5p-fast-transformed-v2",
                                            FacadeRoute::BandedFast => "rodas5p-fast-banded-v1",
                                            FacadeRoute::MatrixFree => {
                                                "rodas5p-mf-fast-transformed-v1"
                                            }
                                            FacadeRoute::FastDense => "rodas5p-fast-dense-v1",
                                            FacadeRoute::FastBandedDense => {
                                                "rodas5p-fast-banded-dense-v1"
                                            }
                                            FacadeRoute::SequentialDense => {
                                                "rodas5p-sequential-dense-v2"
                                            }
                                            _ => panic!("new route needs contract"),
                                        };
                                        assert_eq!(sol.stats.route.driver_id, expected);
                                        routes.insert(format!("{:?}", cap.route));
                                        assert_eq!(
                                            cap.output_accuracy,
                                            if !dense {
                                                OutputAccuracy::EndpointLocalEstimate
                                            } else if interior == InteriorError::Off {
                                                OutputAccuracy::EndpointEstimateWithUncheckedInterior
                                            } else {
                                                OutputAccuracy::EndpointAndInteriorLocalEstimates
                                            }
                                        );
                                    }
                                    other => panic!(
                                        "feature query/solve mismatch at provider={provider}, band={declared_band}, mass={mass}, dense={dense}, vector={vector}, events={events}, interior={interior:?}: {other:?}"
                                    ),
                                }
                                checked += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(checked, 384);
    assert_eq!(refusals.len(), 7);
    assert_eq!(routes.len(), 6);
    if let Ok(path) = std::env::var("VIGILODE_CAPABILITY_MATRIX_OUTPUT") {
        std::fs::write(
            path,
            serde_json::to_string_pretty(&serde_json::json!({
                "schema":"vigilode.capability-matrix.v1","checked":checked,"rows":rows
            }))
            .unwrap()
                + "\n",
        )
        .unwrap();
    }
}
