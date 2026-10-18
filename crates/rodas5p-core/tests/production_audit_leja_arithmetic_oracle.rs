//! A04-PROOF-ORACLE-01: frozen fixture export; independent Fraction oracle is separate.
use rodas5p_core::leja_action::{
    CertifiedLejaCoeffCache, CertifiedLejaWorkspace, leja_divided_differences_certified,
    leja_phi_action_certified_with_setup_work, leja_points,
};
use rodas5p_core::polynomial_action::{JointPhiInput, SymmetricNonpositiveOperator};
use rodas5p_core::{CoreError, DenseMatrix, LejaSetupWork, WorkCounters};
use serde_json::{Value, json};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::path::PathBuf;

struct Counter;
thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<u64> = const { Cell::new(0) };
}
#[global_allocator]
static ALLOC: Counter = Counter;
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(l) };
        if !p.is_null() && TRACK.with(Cell::get) {
            COUNT.with(|c| c.set(c.get() + 1));
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let q = unsafe { System.realloc(p, l, n) };
        if !q.is_null() && TRACK.with(Cell::get) {
            COUNT.with(|c| c.set(c.get() + 1));
        }
        q
    }
}
fn measured<T>(f: impl FnOnce() -> T) -> (T, u64) {
    COUNT.with(|c| c.set(0));
    TRACK.with(|t| t.set(true));
    let r = f();
    TRACK.with(|t| t.set(false));
    (r, COUNT.with(Cell::get))
}
fn decode(v: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().unwrap().trim_start_matches("0x"), 16).unwrap())
}
fn bits(x: f64) -> String {
    format!("0x{:016x}", x.to_bits())
}
fn vec_bits(v: &[f64]) -> Vec<String> {
    v.iter().copied().map(bits).collect()
}
fn decode_vec(v: &Value) -> Vec<f64> {
    v.as_array().unwrap().iter().map(decode).collect()
}
fn coeff(c: f64, g: f64, xi: &[f64]) -> Value {
    let (mid, rad) = leja_divided_differences_certified(c, g, xi).unwrap();
    json!({"c_bits":bits(c),"gamma_bits":bits(g),"xi_bits":vec_bits(xi),
        "mid_bits":mid.iter().map(|a|vec_bits(a)).collect::<Vec<_>>(),
        "radius_bits":rad.iter().map(|a|vec_bits(a)).collect::<Vec<_>>()})
}

#[test]
fn frozen_arithmetic_fixture_export() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest_path=root.join("research/production_audit_20261018/evidence/continuation_a04/arithmetic_research_01/RESEARCH_DECISION.json");
    let manifest: Value = serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
    let contract = &manifest["next_action"]["frozen_oracle_contract"];
    let shared = &contract["shared_input_bit_arrays"];
    let control_vectors = [vec![1.0_f64; 4], vec![2.0_f64; 4], vec![3.0_f64; 4]];
    let (_, positive) = measured(|| {
        for v in &control_vectors {
            std::hint::black_box(v.clone());
        }
    });
    let mut actions = Vec::new();
    let mut failures = Vec::new();
    for f in contract["action_fixtures"].as_array().unwrap() {
        let id = f["id"].as_str().unwrap();
        let rows = shared[f["operator_bits_ref"].as_str().unwrap()]
            .as_array()
            .unwrap();
        let n = rows.len();
        let mut a = DenseMatrix::zeros(n, n);
        for i in 0..n {
            for j in 0..n {
                a[(i, j)] = decode(&rows[i][j]);
            }
        }
        let op = SymmetricNonpositiveOperator::gershgorin(a).unwrap();
        let input = &f["input"];
        let same = input["kind"] == "SameVector";
        let vectors: Vec<Vec<f64>> = if same {
            vec![decode_vec(
                &shared[input["vector_bits_ref"].as_str().unwrap()],
            )]
        } else {
            shared[input["vectors_bits_ref"].as_str().unwrap()]
                .as_array()
                .unwrap()
                .iter()
                .map(decode_vec)
                .collect()
        };
        let scales: [f64; 5] = if same {
            decode_vec(&shared[input["scales_bits_ref"].as_str().unwrap()])
                .try_into()
                .unwrap()
        } else {
            [1.; 5]
        };
        let distinct: Option<&[Vec<f64>; 5]> = if same {
            None
        } else {
            Some(vectors.as_slice().try_into().unwrap())
        };
        let get_input = || {
            if same {
                JointPhiInput::SameVector {
                    vector: &vectors[0],
                    scales,
                }
            } else {
                JointPhiInput::Distinct(distinct.unwrap())
            }
        };
        let h = decode(&f["h_bits"]);
        let budget = decode(&f["total_budget_bits"]);
        let cap = f["degree_cap"].as_u64().unwrap() as usize;
        let mut ws = CertifiedLejaWorkspace::new(n, if same { 1 } else { 5 });
        let mut cache = CertifiedLejaCoeffCache::new();
        let mut work = WorkCounters::default();
        let mut setup = LejaSetupWork::default();
        // Every refused attempt has a last successful report on this exact workspace.
        if f["expected"]["return"] == "Err" {
            let seed = vec![0.; n];
            leja_phi_action_certified_with_setup_work(
                &op,
                h,
                JointPhiInput::SameVector {
                    vector: &seed,
                    scales: [1.; 5],
                },
                budget,
                cap,
                Some(&mut cache),
                &mut work,
                &mut ws,
                &mut setup,
            )
            .unwrap();
        }
        let prior = ws.report().cloned();
        let before = work;
        let setup_before = setup.snapshot();
        let result = leja_phi_action_certified_with_setup_work(
            &op,
            h,
            get_input(),
            budget,
            cap,
            Some(&mut cache),
            &mut work,
            &mut ws,
            &mut setup,
        );
        let mut row = json!({"id":id,"fixture":f,"work_before":before,"f09_coverage_caveat":if id=="A04-F09-MATVEC-OVERFLOW" {Some("finite-input arithmetic-overflow refusal; initial L2 norm, not matvec coverage")}else{None}});
        match result {
            Ok(report) => {
                row["return"] = json!("Ok");
                row["report"] = serde_json::to_value(report).unwrap();
                row["fused_bits"] = json!(vec_bits(report.fused()));
                row["total_bound_bits"] = json!(bits(report.total_bound()));
                row["derived_coefficient_table"] = coeff(
                    report.center(),
                    report.scale(),
                    if report.branch() == "newton" {
                        &leja_points()[..=report.degree()]
                    } else {
                        &[0.]
                    },
                );
                row["derived_table_semantics"] = json!(match report.branch() {
                    "newton" =>
                        "public coefficient helper queried at actual action c/gamma and actual used Leja prefix",
                    "scalar" =>
                        "public coefficient helper query at action c/gamma/node 0; production scalar branch uses its scalar helper",
                    _ =>
                        "additional public helper query at action c/gamma/node 0; zero branch uses no coefficient table",
                });
                if f["expected"]["return"] != "Ok"
                    || f["expected"]["branch"] != report.branch()
                    || f["expected"]["converged"] != report.converged()
                {
                    failures.push(format!("{id}: unexpected return/branch/convergence"));
                }
                if row["report"]["total_error"]["status"] != "estimate-only" {
                    failures.push(format!("{id}: unexpected authority"));
                }
            }
            Err(e) => {
                row["return"] = json!("Err");
                row["error"] = json!(e.to_string());
                row["error_family"] = json!(if matches!(e, CoreError::NonFinite(_)) {
                    "CoreError::NonFinite"
                } else {
                    "OTHER"
                });
                let preserved = serde_json::to_vec(&ws.report()).unwrap()
                    == serde_json::to_vec(&prior.as_ref()).unwrap();
                row["last_report_preserved"] = json!(preserved);
                row["prior_success_report"] = json!(prior);
                row["report_after_refusal"] = json!(ws.report());
                if f["expected"]["return"] != "Err"
                    || !matches!(e, CoreError::NonFinite(_))
                    || !preserved
                {
                    failures.push(format!("{id}: refusal contract failure"));
                }
            }
        }
        row["work_after"] = json!(work);
        row["work_delta"] = json!(work.checked_delta(before).expect("work decreased"));
        row["last_certification_ops"] = json!(ws.last_certification_ops());
        row["setup_before"] = json!(setup_before);
        row["setup_after"] = json!(setup.snapshot());
        if contract["allocator"]["warmed_fixture_ids"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == id)
            && row["return"] == "Ok"
        {
            let warm_before = work;
            let (ok, count) = measured(|| {
                leja_phi_action_certified_with_setup_work(
                    &op,
                    h,
                    get_input(),
                    budget,
                    cap,
                    Some(&mut cache),
                    &mut work,
                    &mut ws,
                    &mut setup,
                )
                .is_ok()
            });
            row["warmed_allocations"] = json!(count);
            row["warm_work_delta"] = json!(work.checked_delta(warm_before).unwrap());
            row["warm_setup_after"] = json!(setup.snapshot());
            row["allocation_status"] = json!(if positive < 3 {
                "ALLOCATION_UNAVAILABLE"
            } else if !ok || count != 0 {
                "ALLOCATION_NONZERO"
            } else {
                "ZERO_ALLOCATION_PASS_SCOPED"
            });
            if positive >= 3 && (!ok || count != 0) {
                failures.push(format!("{id}: warmed allocation contract"));
            }
        }
        actions.push(row);
    }
    let coefficients:Vec<Value>=contract["coefficient_fixtures"].as_array().unwrap().iter().map(|f|{
        let xi=decode_vec(&f["xi_bits"]);json!({"id":f["id"],"fixture":f,"table":coeff(decode(&f["c_bits"]),decode(&f["gamma_bits"]),&xi)})
    }).collect();
    let output = json!({"schema":"a04-proof-oracle-native-v1","allocation_positive_control":positive,
        "allocation_calibration_status":if positive>=3{"CALIBRATED"}else{"ALLOCATION_UNAVAILABLE"},
        "actions":actions,"coefficients":coefficients,"contract_failures":failures});
    if let Some(target) = std::env::var_os("A04_ORACLE_NATIVE_OUTPUT") {
        std::fs::write(
            PathBuf::from(target),
            serde_json::to_vec_pretty(&output).unwrap(),
        )
        .unwrap();
    }
    assert!(
        failures.is_empty(),
        "contract failures: {failures:?}; raw JSON retained"
    );
    // Missing calibration withholds allocation authority; numeric oracle inputs remain valid.
    // The native receipt reports ALLOCATION_UNAVAILABLE explicitly in that case.
}
