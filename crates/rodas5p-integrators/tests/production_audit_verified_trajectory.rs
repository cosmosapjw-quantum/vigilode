//! A02-TXN-01 frozen V2 integration fixture export. Numerical execution is a
//! separate controller-frozen stage; no tolerance or reference-order tuning.
use rodas5p_integrators::{CollectiveModel, PacketWorkCounters};
mod packet_counter_contract {
    use super::{PacketWorkCounters, VerifiedTrajectoryWork};
    include!("support/a02_packet_counter_contract.rs");
}
use rodas5p_integrators::packet_trajectory::*;
use serde_json::{Value, json};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
struct Counter;
thread_local! {static TRACK:Cell<bool>=const{Cell::new(false)};static COUNT:Cell<u64>=const{Cell::new(0)};}
#[global_allocator]
static ALLOC: Counter = Counter;
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if TRACK.with(Cell::get) {
            COUNT.with(|x| x.set(x.get() + 1));
        }
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        if TRACK.with(Cell::get) {
            COUNT.with(|x| x.set(x.get() + 1));
        }
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        if TRACK.with(Cell::get) {
            COUNT.with(|x| x.set(x.get() + 1));
        }
        unsafe { System.realloc(p, l, n) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
fn measured<T>(f: impl FnOnce() -> T) -> (T, u64) {
    COUNT.with(|x| x.set(0));
    TRACK.with(|x| x.set(true));
    let v = f();
    TRACK.with(|x| x.set(false));
    (v, COUNT.with(Cell::get))
}
fn scalar(v: &Value) -> f64 {
    f64::from_bits(
        u64::from_str_radix(v["bits"].as_str().unwrap().trim_start_matches("0x"), 16).unwrap(),
    )
}
fn array(v: &Value) -> Vec<f64> {
    v.as_array().unwrap().iter().map(scalar).collect()
}
fn model(v: &Value) -> CollectiveModel {
    CollectiveModel::new(
        array(&v["xi"]),
        array(&v["weights"]),
        v["initial_centers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|z| (scalar(&z[0]), scalar(&z[1])))
            .collect(),
        scalar(&v["width"]),
        scalar(&v["chi"]),
        scalar(&v["carrier_metadata_only"]),
        scalar(&v["kappa"]),
    )
    .unwrap()
}
fn config(v: &Value) -> TrajectoryConfig {
    TrajectoryConfig {
        t_start: scalar(&v["t_start"]),
        t_end: scalar(&v["t_end"]),
        initial_h: scalar(&v["initial_h"]),
        min_h: scalar(&v["min_h"]),
        max_h: scalar(&v["max_h"]),
        order: v["order"].as_u64().unwrap() as usize,
        degree: v["degree"].as_u64().unwrap() as usize,
        sweeps: v["sweeps"].as_u64().unwrap() as usize,
        mean_tolerance: scalar(&v["mean_tolerance"]),
        mode_tolerance: scalar(&v["mode_tolerance"]),
        output_policy: OutputPolicy::AllModes,
        max_steps: v["max_steps"].as_u64().unwrap() as usize,
    }
}
fn point(p: &VerifiedTrajectoryPoint) -> Value {
    json!({"time":p.time(),"local_time":p.local_time(),"offset":[p.offset_interval().lo,p.offset_interval().hi],"temporal_delta":p.temporal_delta(),"identity":p.identity(),"mean":p.mean(),"mean_radius":p.mean_radius(),"modes":p.modes(),"mode_radii":p.mode_radii(),"weighted_seed":p.weighted_seed(),"snapshot_bits":p.snapshot_bits()})
}
fn frozen() -> Value {
    serde_json::from_str(include_str!("../../../research/production_audit_20261018/evidence/continuation_a02_research/supplement_02/FROZEN_ACCEPTANCE_INPUTS_V2.json")).unwrap()
}
#[test]
fn frozen_v2_trajectory_acceptance_export() {
    let manifest = frozen();
    let mut rows = Vec::new();
    let mut failures = Vec::new();
    for (id, f) in manifest["fixtures"].as_object().unwrap() {
        let m = model(&manifest["models"][f["model_id"].as_str().unwrap()]);
        let c = config(&manifest["configs"][f["config_id"].as_str().unwrap()]);
        let grid = array(&f["requested_times"]);
        let radii = array(&f["initial_component_radii"]);
        let mut w = VerifiedTrajectoryWork::default();
        let setup_full_before = w.clone();
        let setup_before = w.packet;
        let (setup, setup_alloc) =
            measured(|| VerifiedTrajectoryState::new(&m, &radii, &c, &grid, &mut w));
        let mut state = match setup {
            Ok(x) => x,
            Err(e) => {
                if id != "F_SUBNORMAL" {
                    failures.push(format!("{id}:setup {e:?}"));
                }
                rows.push(json!({"id":id,"setup_refusal":format!("{e:?}"),"work":w,"setup_allocations":setup_alloc}));
                continue;
            }
        };
        packet_counter_contract::assert_delta(
            &setup_before,
            &w.packet,
            &PacketWorkCounters::default(),
            "initial setup",
        );
        packet_counter_contract::assert_full_delta(
            &setup_full_before,
            &w,
            &packet_counter_contract::initial_adapter(m.num_modes(), grid.len()),
            "initial setup full",
        );
        let original_energy = state.global_energy().to_bits();
        let mut h = c.initial_h.min(c.max_h);
        let mut attempts = Vec::new();
        let mut failure = None;
        for _ in 0..c.max_steps {
            if state.time() >= c.t_end {
                break;
            }
            let end = (state.time() + h).min(c.t_end);
            let before = state.snapshot_bits();
            if end <= state.time() || h < c.min_h {
                failure = Some("minimum advancing step".to_owned());
                break;
            }
            let prepare_full_before = w.clone();
            let prepare_before = w.packet;
            let (prepared, alloc) = measured(|| state.prepare(end, &mut w));
            let mut p = match prepared {
                Ok(p) => p,
                Err(e) => {
                    attempts.push(json!({"end":end,"prepare_refusal":format!("{e:?}"),"prepare_allocations":alloc,"work":w}));
                    failure = Some(format!("prepare {e:?}"));
                    assert_eq!(state.snapshot_bits(), before);
                    break;
                }
            };
            packet_counter_contract::assert_delta(
                &prepare_before,
                &w.packet,
                &packet_counter_contract::prepare(m.num_modes(), c.order, c.degree, c.sweeps),
                "prepare success",
            );
            let k = grid
                .iter()
                .skip(state.output_cursor())
                .take_while(|&&t| t <= end)
                .count();
            packet_counter_contract::assert_full_delta(
                &prepare_full_before,
                &w,
                &packet_counter_contract::prepare_adapter(
                    m.num_modes(),
                    c.order,
                    c.degree,
                    c.sweeps,
                    grid.len(),
                    k,
                ),
                "prepare full",
            );
            let prior = w.clone();
            let (result, advance_alloc) = measured(|| advance_verified(&mut state, &mut p, &mut w));
            let evaluated = match &result {
                Ok(()) => k + 1,
                Err(VerifiedTrajectoryRefusal::ToleranceRejected { time, .. }) => grid
                    .iter()
                    .skip(state.output_cursor())
                    .take(k)
                    .position(|t| t.to_bits() == time.to_bits())
                    .map(|i| i + 1)
                    .unwrap_or(k + 1),
                Err(_) => 0,
            };
            if result.is_ok()
                || matches!(
                    result,
                    Err(VerifiedTrajectoryRefusal::ToleranceRejected { .. })
                )
            {
                if id == "F_REJECTION" && attempts.is_empty() {
                    assert_eq!(
                        evaluated, 1,
                        "frozen initial rejection is at first requested1/8"
                    );
                }
                let expected = match &result {
                    Ok(()) => packet_counter_contract::commit_adapter(
                        m.num_modes(),
                        c.order,
                        c.degree,
                        grid.len(),
                        k,
                    ),
                    Err(VerifiedTrajectoryRefusal::ToleranceRejected { mode, .. }) => {
                        packet_counter_contract::rejected_tolerance_adapter(
                            m.num_modes(),
                            c.order,
                            c.degree,
                            grid.len(),
                            evaluated - 1,
                            *mode,
                        )
                    }
                    _ => unreachable!(),
                };
                packet_counter_contract::assert_full_delta(
                    &prior,
                    &w,
                    &expected,
                    "advance/physical rejection full",
                );
                assert_eq!(
                    state.global_energy().to_bits(),
                    original_energy,
                    "E_global must retain original tube"
                );
            }
            attempts.push(json!({"end":end,"prepare_work_before":prepare_before,"prepare_work_after":prior.packet,"expected_prepare_packet":packet_counter_contract::prepare(m.num_modes(),c.order,c.degree,c.sweeps),"expected_advance_packet":if result.is_ok(){packet_counter_contract::commit_adapter(m.num_modes(),c.order,c.degree,grid.len(),k).packet}else{packet_counter_contract::query(m.num_modes(),c.order,c.degree,evaluated as u64)},"prepare_allocations":alloc,"advance_allocations":advance_alloc,"result":format!("{result:?}"),"work_before_advance":prior,"work_after_advance":w}));
            match result {
                Ok(()) => {}
                Err(VerifiedTrajectoryRefusal::ToleranceRejected { .. }) => {
                    assert_eq!(state.snapshot_bits(), before);
                    h *= 0.5;
                }
                Err(e) => {
                    assert_eq!(state.snapshot_bits(), before);
                    failure = Some(format!("advance {e:?}"));
                    break;
                }
            }
        }
        if state.time() != c.t_end && id != "F_SUBNORMAL" {
            failures.push(format!("{id}: incomplete {failure:?}"));
        }
        if state.retained_windows().len() < 2 && id != "F_OFFSET" && id != "F_SUBNORMAL" {
            failures.push(format!("{id}: fewer than two windows"));
        }
        let mut historical = Vec::new();
        let mut historical_work = Vec::new();
        for window in state.retained_windows() {
            let mut scratch = window.workspace(&mut w).unwrap();
            let midpoint = window.start_time() + (window.end_time() - window.start_time()) * 0.5;
            for t in [window.start_time(), midpoint, window.end_time()] {
                let query_full_before = w.clone();
                let query_before = w.packet;
                match window.query(t, &mut scratch, &mut w) {
                    Ok(p) => {
                        historical.push(point(p));
                        packet_counter_contract::assert_delta(
                            &query_before,
                            &w.packet,
                            &packet_counter_contract::query(m.num_modes(), c.order, c.degree, 1),
                            "historical query success",
                        );
                        packet_counter_contract::assert_full_delta(
                            &query_full_before,
                            &w,
                            &packet_counter_contract::query_adapter(
                                m.num_modes(),
                                c.order,
                                c.degree,
                            ),
                            "historical full",
                        );
                        historical_work.push(json!({"time":t,"before":query_before,"after":w.packet,"expected":packet_counter_contract::query(m.num_modes(),c.order,c.degree,1)}));
                    }
                    Err(e) => failures.push(format!("{id}: historical {e:?}")),
                }
            }
        }
        if id == "F_OFFSET" {
            let t = f64::from_bits(0x3feccccccccccccd);
            let window = &state.retained_windows()[0];
            let mut scratch = window.workspace(&mut w).unwrap();
            let p = window.query(t, &mut scratch, &mut w).unwrap();
            assert!(p.offset_interval().lo < p.offset_interval().hi);
            assert!(p.temporal_delta() > 0.0);
            historical.push(point(p));
        }
        // Replay exactly the accepted endpoint schedule, preserving original
        // initial values and uncertainty. Never restart the reference there.
        if id == "F_REJECTION" {
            let mut replay = VerifiedTrajectoryState::new(&m, &radii, &c, &grid, &mut w).unwrap();
            for window in state.retained_windows() {
                let mut p = replay.prepare(window.end_time(), &mut w).unwrap();
                advance_verified(&mut replay, &mut p, &mut w).unwrap();
            }
            assert_eq!(state.centers(), replay.centers());
            assert_eq!(state.component_radii(), replay.component_radii());
            assert_eq!(
                state.weighted_radius().to_bits(),
                replay.weighted_radius().to_bits()
            );
            if !attempts.first().unwrap()["result"]
                .as_str()
                .unwrap()
                .contains("ToleranceRejected")
            {
                failures.push(
                    "F_REJECTION initial trial was not physically tolerance-rejected".to_owned(),
                );
            }
        }
        rows.push(json!({"id":id,"model_id":f["model_id"],"t_start":c.t_start,"requested_times":grid,"initial_component_radii":radii,"observations":state.observations().iter().map(point).collect::<Vec<_>>(),"historical":historical,"historical_packet_work":historical_work,"attempts":attempts,"work":w,"setup_allocations":setup_alloc,"failure":failure,"accepted_windows":state.retained_windows().len()}));
    }
    let report = json!({"unit":"A02-TXN-01","manifest_version":2,"fixtures":rows,"failures":failures,"production":"HOLD","timing_authority":"DIAGNOSTIC_ONLY"});
    if let Ok(path) = std::env::var("VIGILODE_A02_OUTPUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    assert!(failures.is_empty(), "{failures:?}");
}
#[test]
fn stale_tokens_historical_seed_and_allocator_control() {
    let d = frozen();
    let f = &d["fixtures"]["F_FIXED"];
    let m = model(&d["models"]["M_FIXED"]);
    let mut c = config(&d["configs"]["C_FIXED"]);
    let grid = array(&f["requested_times"]);
    let (_, positive) = measured(|| {
        let x = std::hint::black_box(m.clone());
        std::hint::black_box(x.xi().as_ptr());
        std::hint::black_box(x.weights().as_ptr());
        std::hint::black_box(x.initial_amplitudes().as_ptr());
    });
    let mut metrics = Vec::new();
    let mut all_modes_numeric: Option<Vec<u64>> = None;
    for policy in [OutputPolicy::AllModes, OutputPolicy::MeanOnly] {
        c.output_policy = policy;
        let mut w = VerifiedTrajectoryWork::default();
        let mut s = VerifiedTrajectoryState::new(&m, &[0.0; 3], &c, &grid, &mut w).unwrap();
        let mut p0 = s.prepare(0.0625, &mut w).unwrap();
        let mut q0 = s.prepare(0.0625, &mut w).unwrap();
        advance_verified(&mut s, &mut p0, &mut w).unwrap();
        let mut scratch = s.retained_windows()[0].workspace(&mut w).unwrap();
        let original = s.retained_windows()[0]
            .query(0.03125, &mut scratch, &mut w)
            .unwrap()
            .snapshot_bits();
        let before = s.snapshot_bits();
        let stale_before = w.clone();
        let result = advance_verified(&mut s, &mut q0, &mut w);
        assert!(matches!(
            result,
            Err(VerifiedTrajectoryRefusal::StalePreparedWindow)
        ));
        assert_eq!(s.snapshot_bits(), before);
        assert_eq!(w.query_attempts, stale_before.query_attempts);
        packet_counter_contract::assert_full_delta(
            &stale_before,
            &w,
            &packet_counter_contract::fault_adapter(3, c.order, c.degree, grid.len(), 0),
            "stale Q0 full",
        );
        packet_counter_contract::assert_delta(
            &stale_before.packet,
            &w.packet,
            &PacketWorkCounters::default(),
            "stale Q0 refusal",
        );
        assert_eq!(
            w.identity_items - stale_before.identity_items,
            22 + 5 * 3 + grid.len() as u64
        );
        let mut p1 = s.prepare(0.125, &mut w).unwrap();
        let before_commit = w.clone();
        let (result, advance_alloc) = measured(|| advance_verified(&mut s, &mut p1, &mut w));
        result.unwrap();
        packet_counter_contract::assert_full_delta(
            &before_commit,
            &w,
            &packet_counter_contract::commit_adapter(3, c.order, c.degree, grid.len(), 1),
            "allocator measured epoch2 commit full",
        );
        let before = s.snapshot_bits();
        assert!(matches!(
            advance_verified(&mut s, &mut p0, &mut w),
            Err(VerifiedTrajectoryRefusal::StalePreparedWindow)
        ));
        assert_eq!(s.snapshot_bits(), before);
        let window = &s.retained_windows()[0];
        let before_query = w.clone();
        let (result, query_alloc) = measured(|| window.query(0.03125, &mut scratch, &mut w));
        let p = result.unwrap();
        assert_eq!(p.snapshot_bits(), original);
        packet_counter_contract::assert_full_delta(
            &before_query,
            &w,
            &packet_counter_contract::query_adapter(3, c.order, c.degree),
            "window0 historical after epoch2 full",
        );
        let numeric: Vec<u64> = p
            .snapshot_bits()
            .into_iter()
            .enumerate()
            .filter_map(|(i, x)| (![5, 6].contains(&i)).then_some(x))
            .collect();
        if policy == OutputPolicy::AllModes {
            all_modes_numeric = Some(numeric);
        } else {
            assert_eq!(
                all_modes_numeric.as_ref().unwrap(),
                &numeric,
                "MeanOnly must match every numeric payload word excluding lineage/generation"
            );
        }
        assert_eq!(p.modes().is_some(), policy == OutputPolicy::AllModes);
        for t in [f64::NAN, f64::INFINITY] {
            assert!(window.query(t, &mut scratch, &mut w).is_err());
        }
        metrics.push(json!({"policy":format!("{policy:?}"),"positive_control_allocations":positive,"warmed_query_allocations":query_alloc,"warmed_advance_allocations":advance_alloc,"allocation_status":if positive<3 {"ALLOCATION_UNAVAILABLE"}else if query_alloc==0&&advance_alloc==0 {"ZERO_OBSERVED"}else{"NONZERO_OBSERVED"},"work":w}));
        if positive >= 3 {
            assert_eq!(query_alloc, 0);
            assert_eq!(advance_alloc, 0);
        }
    }
    if let Ok(path) = std::env::var("VIGILODE_A02_ALLOCATION_OUTPUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&metrics).unwrap()).unwrap();
    }
}

#[test]
fn frozen_at08_invalid_inputs_preserve_accepted_state() {
    let d = frozen();
    let f = &d["fixtures"]["F_FIXED"];
    let m = model(&d["models"]["M_FIXED"]);
    let c = config(&d["configs"]["C_FIXED"]);
    let grid = array(&f["requested_times"]);
    let mut w = VerifiedTrajectoryWork::default();
    let state = VerifiedTrajectoryState::new(&m, &[0.0; 3], &c, &grid, &mut w).unwrap();
    let snapshot = state.snapshot_bits();
    let mut outcomes = Vec::new();
    for (id, r) in [
        ("NEG_RADIUS", vec![-1e-6, 0.0, 0.0]),
        ("NAN_RADIUS", vec![f64::NAN, 0.0, 0.0]),
        ("INF_RADIUS", vec![f64::INFINITY, 0.0, 0.0]),
        ("BAD_SHAPE", vec![0.0; 2]),
    ] {
        let result = VerifiedTrajectoryState::new(&m, &r, &c, &grid, &mut w);
        assert!(result.is_err());
        assert_eq!(snapshot, state.snapshot_bits());
        outcomes.push(json!({"id":id,"refused":true,"work":w}));
    }
    for id in ["HUGE_SIZE", "ZERO_H"] {
        let mut bad = c.clone();
        if id == "HUGE_SIZE" {
            bad.order = usize::MAX;
            bad.degree = usize::MAX;
        } else {
            bad.initial_h = 0.0;
        }
        assert!(VerifiedTrajectoryState::new(&m, &[0.0; 3], &bad, &grid, &mut w).is_err());
        assert_eq!(snapshot, state.snapshot_bits());
        outcomes.push(json!({"id":id,"refused":true,"work":w}));
    }
    for (id, xi, kappa) in [
        ("OVERFLOW_OMEGA", f64::from_bits(0x3ff0000000000008), 0.0),
        ("UNSUPPORTED_DAMPING", 0.5, 0.125),
        ("OVERFLOW_OMEGA_V1_DOMAIN_REFUSAL", 2.0, 0.0),
    ] {
        let candidate = CollectiveModel::new(
            vec![xi; 3],
            m.weights().to_vec(),
            m.initial_amplitudes().to_vec(),
            f64::MAX,
            0.0,
            0.0,
            kappa,
        );
        match candidate {
            Err(e) => {
                assert_eq!(id, "OVERFLOW_OMEGA_V1_DOMAIN_REFUSAL");
                outcomes.push(json!({"id":id,"domain_refusal":format!("{e:?}")}));
            }
            Ok(m) => match VerifiedTrajectoryState::new(&m, &[0.0; 3], &c, &grid, &mut w) {
                Err(e) => {
                    assert_eq!(id, "UNSUPPORTED_DAMPING");
                    outcomes.push(json!({"id":id,"refusal":format!("{e:?}")}));
                }
                Ok(s) => {
                    let before = s.snapshot_bits();
                    let result = s.prepare(0.0625, &mut w);
                    assert!(matches!(
                        result,
                        Err(VerifiedTrajectoryRefusal::Arithmetic(_))
                    ));
                    assert_eq!(s.snapshot_bits(), before);
                    outcomes.push(json!({"id":id,"refused":true,"work":w}));
                }
            },
        }
        assert_eq!(snapshot, state.snapshot_bits());
    }
    // The original stationary and inherited fixtures fix the subnormal arms.
    for id in ["SUBNORMAL_TIME", "SUBNORMAL_RADIUS"] {
        let fixture = &d["fixtures"][if id == "SUBNORMAL_TIME" {
            "F_SUBNORMAL"
        } else {
            "F_INHERITED"
        }];
        let m = model(&d["models"][fixture["model_id"].as_str().unwrap()]);
        let mut c = config(&d["configs"][fixture["config_id"].as_str().unwrap()]);
        let mut grid = array(&fixture["requested_times"]);
        let mut radii = array(&fixture["initial_component_radii"]);
        if id == "SUBNORMAL_TIME" {
            let tiny = f64::from_bits(1);
            c.t_start = 0.0;
            c.t_end = tiny;
            c.initial_h = tiny;
            c.min_h = tiny;
            c.max_h = tiny;
            grid = vec![0.0, tiny];
        } else {
            radii.fill(f64::from_bits(1));
        }
        let result = VerifiedTrajectoryState::new(&m, &radii, &c, &grid, &mut w);
        match result {
            Err(e) => outcomes.push(json!({"id":id,"setup_refusal":format!("{e:?}"),"work":w,"model_id":fixture["model_id"],"original_fixture":fixture,"effective_config":c,"t_start":c.t_start,"requested_times":grid,"initial_radius_bits":radii.iter().map(|x|format!("0x{:016x}",x.to_bits())).collect::<Vec<_>>()})),
            Ok(mut s) => {
                let before = s.snapshot_bits();
                let result = complete_verified_trajectory(&mut s, &mut w);
                if result.is_err() {
                    assert_eq!(before, s.snapshot_bits());
                }
                outcomes.push(json!({"id":id,"model_id":fixture["model_id"],"original_fixture":fixture,"effective_config":c,"t_start":c.t_start,"requested_times":grid,"initial_radius_bits":radii.iter().map(|x|format!("0x{:016x}",x.to_bits())).collect::<Vec<_>>(),"result":format!("{result:?}"),"observations":s.observations().iter().map(point).collect::<Vec<_>>(),"work":w}));
            }
        }
    }
    if let Ok(path) = std::env::var("VIGILODE_A02_INVALID_OUTPUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&outcomes).unwrap()).unwrap();
    }
}

#[test]
fn borrowed_admission_matches_constructor_and_counts_allocator() {
    let d = frozen();
    let m = model(&d["models"]["M_FIXED"]);
    let (_, positive) = measured(|| {
        let x = std::hint::black_box(m.clone());
        std::hint::black_box(x.initial_amplitudes().as_ptr());
    });
    for centers in [
        m.initial_amplitudes().to_vec(),
        vec![(2.0, 0.0); 3],
        vec![(0.0, 0.0); 2],
    ] {
        let expected = CollectiveModel::new(
            m.xi().to_vec(),
            m.weights().to_vec(),
            centers.clone(),
            m.width(),
            m.chi(),
            m.carrier(),
            m.kappa(),
        )
        .map(|x| x.initial_energy());
        let (result, alloc) = measured(|| m.validate_candidate_centers(&centers));
        assert_eq!(
            result, expected,
            "borrowed validator must preserve constructor semantics and refusal"
        );
        if positive >= 3 {
            assert_eq!(
                alloc, 0,
                "finite valid/energy/shape paths allocate no error String"
            );
        }
        println!(
            "borrowed_admission positive_control={positive} allocations={alloc} result={result:?}"
        );
    }
}
