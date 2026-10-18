//! Frozen A03-SCALAR-RATE-01 inputs, allocator controls, raw result export.
use rodas5p_core::damped_moments::*;
use rodas5p_core::directed::Interval;
use serde_json::{Value, json};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
struct Counter;
thread_local! {static TRACK: Cell<bool> = const {Cell::new(false)}; static COUNT: Cell<u64> = const {Cell::new(0)};}
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
    TRACK.with(|c| c.set(true));
    let x = f();
    TRACK.with(|c| c.set(false));
    (x, COUNT.with(Cell::get))
}
fn p(x: f64) -> Interval {
    Interval { lo: x, hi: x }
}
fn bits(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}
fn work_json(w: ScalarRateWork) -> Value {
    json!({"validation_attempts":w.validation_attempts,"endpoint_attempts":w.endpoint_attempts,"width_checks":w.width_checks,"fallback_attempts":w.fallback_attempts,"cells_attempted":w.cells_attempted,"query_refusals":w.query_refusals,"interval_primitive_attempts":w.interval_primitive_attempts,"coefficient_items":w.coefficient_items,"output_items":w.output_items,"scratch_bytes":w.scratch_bytes,"allocation_requests":w.allocation_requests})
}
fn output(result: &Result<DampedForcingMomentResult, ScalarRateRefusal>) -> Value {
    match result {
        Ok(v) => {
            json!({"status":"ENCLOSED","lo_bits":bits(v.interval.lo),"hi_bits":bits(v.interval.hi),"width_bits":bits(v.width_upper)})
        }
        Err(e) => json!({"status":"REFUSED","refusal":format!("{e:?}")}),
    }
}
#[test]
fn frozen_s01_s10_export() {
    let control = [vec![1.0; 4], vec![2.0; 4], vec![3.0; 4]];
    let (_, positive) = measured(|| {
        for v in &control {
            std::hint::black_box(v.clone());
        }
    });
    let width = f64::from_bits(0x3d70000000000000); // 2^-40
    assert_eq!(width, 2.0_f64.powi(-40));
    let fixtures = [
        ("S01", 0, 0x3fe0000000000000, 0, 2, 3, p(1.0), width),
        ("S02", 1, 0x3fe0000000000000, 0, 0, 0, p(1.0), width),
        (
            "S03",
            0x3fb999999999999a,
            0x3ff0000000000000,
            2,
            0,
            1,
            p(1.0),
            width,
        ),
        (
            "S04",
            0x3fb999999999999a,
            0x4024000000000000,
            0,
            1,
            0,
            p(0.5),
            width,
        ),
        (
            "S05",
            0x412e848000000000,
            0x3ff0000000000000,
            0,
            0,
            0,
            p(1.0),
            2.0_f64.powi(-35),
        ),
        (
            "S06",
            0x7fefffffffffffff,
            0x4000000000000000,
            0,
            0,
            0,
            p(1.0),
            width,
        ),
        (
            "S07",
            0x3fd0000000000000,
            0x3fe0000000000000,
            1,
            0,
            1,
            Interval {
                lo: 0.5,
                hi: 0.5_f64.next_up(),
            },
            width,
        ),
        (
            "S08",
            0x3fd0000000000000,
            0x3fe0000000000000,
            1,
            65,
            0,
            p(1.0),
            width,
        ),
        (
            "S10",
            0x3fb999999999999a,
            0x3ff0000000000000,
            2,
            0,
            1,
            p(1.0),
            0.0,
        ),
    ];
    let mut rows = Vec::new();
    for (id, kbits, hbits, q, r, d, time, limit) in fixtures {
        let k = f64::from_bits(kbits);
        let h = f64::from_bits(hbits);
        let mut scratch = [p(0.0); SCALAR_RATE_SCRATCH_LEN];
        let mut work = ScalarRateWork::default();
        let (rate, setup_alloc) =
            measured(|| NormalizedDampingRate::from_inputs(k, h, q, &mut work));
        let owner = rate;
        let setup_work = work;
        let setup_result = match rate {
            Ok(a) => {
                json!({"status":"OWNER_CREATED","rate_bits":[bits(a.interval().lo),bits(a.interval().hi)]})
            }
            Err(e) => json!({"status":"REFUSED","refusal":format!("{e:?}")}),
        };
        let mut warm_work = ScalarRateWork::default();
        let (warmup_result, warmup_allocations, warmup_status) = if let Ok(a) = rate {
            let (warm, allocation_count) =
                measured(|| a.query(k, h, q, time, r, d, limit, &mut scratch, &mut warm_work));
            (output(&warm), Some(allocation_count), "ATTEMPTED")
        } else {
            (
                json!({"status":"NOT_RUN_SETUP_REFUSAL"}),
                None,
                "NOT_RUN_SETUP_REFUSAL",
            )
        };
        let measured_work_before = work;
        let (result, alloc) = measured(|| match rate {
            Ok(a) => a.query(k, h, q, time, r, d, limit, &mut scratch, &mut work),
            Err(e) => Err(e),
        });
        assert_eq!(rate, owner);
        assert!(work.interval_primitive_attempts <= SCALAR_RATE_PRIMITIVE_CAP + 1);
        if id == "S06" {
            assert!(matches!(
                result,
                Err(ScalarRateRefusal::ArithmeticRefusal { .. })
            ));
        } else if id == "S08" {
            assert!(matches!(
                result,
                Err(ScalarRateRefusal::UnsupportedMomentOrder {
                    required: 65,
                    supported: 64
                })
            ));
        } else if id == "S10" {
            assert!(matches!(
                result,
                Err(ScalarRateRefusal::MomentWidthExceeded { .. })
            ));
        } else if id == "S02" {
            assert!(matches!(
                result,
                Ok(_) | Err(ScalarRateRefusal::MomentWidthExceeded { .. })
            ));
        } else if id == "S05" {
            assert!(matches!(
                result,
                Ok(_)
                    | Err(ScalarRateRefusal::MomentWidthExceeded { .. })
                    | Err(ScalarRateRefusal::MomentWorkBudgetExceeded { .. })
            ));
        } else {
            assert!(result.is_ok(), "{id}: {result:?}");
        }
        if let Ok(v) = result {
            assert!(v.width_upper <= limit);
        }
        let rate_bits = rate
            .ok()
            .map(|a| json!([bits(a.interval().lo), bits(a.interval().hi)]));
        rows.push(json!({"id":id,"kappa_bits":bits(k),"horizon_bits":bits(h),"q":q,"r":r,"d":d,"time_bits":[bits(time.lo),bits(time.hi)],"max_width_bits":bits(limit),"rate_bits":rate_bits,"setup_result":setup_result,"setup_work":work_json(setup_work),"setup_allocations":setup_alloc,"warmup_status":warmup_status,"warmup_result":warmup_result,"warmup_work":work_json(warm_work),"warmup_allocations":warmup_allocations,"result":output(&result),"measured_query_work_before":work_json(measured_work_before),"measured_query_work_after":work_json(work),"work":work_json(work),"query_allocations":alloc,"allocation_status":if positive==3 {if alloc==0 {"ZERO_OBSERVED"}else{"NONZERO_OBSERVED"}}else{"ALLOCATION_UNAVAILABLE"}}));
    }
    let mut invalid = Vec::new();
    for (name, k) in [
        ("nan", f64::from_bits(0x7ff8000000000001)),
        ("inf", f64::INFINITY),
        ("negative", f64::from_bits(0xbfc0000000000000)),
    ] {
        let mut w = ScalarRateWork::default();
        let (res, a) = measured(|| NormalizedDampingRate::from_inputs(k, 0.5, 1, &mut w));
        assert!(matches!(res, Err(ScalarRateRefusal::InvalidInput { .. })));
        invalid.push(json!({"id":format!("S09-{name}"),"allocations":a,"refusal":format!("{:?}",res.unwrap_err()),"work":work_json(w)}));
    }
    let mut w = ScalarRateWork::default();
    let rate = NormalizedDampingRate::from_inputs(0.25, 0.5, 1, &mut w).unwrap();
    let frozen = rate;
    for (name, time, len) in [
        ("inverted", Interval { lo: 1.0, hi: 0.5 }, 129),
        (
            "nan_interval",
            Interval {
                lo: f64::NAN,
                hi: 1.0,
            },
            129,
        ),
        ("scratch", p(1.0), 128),
    ] {
        let mut scratch = [p(7.0); 129];
        let before = scratch;
        let old = w;
        let (res, a) =
            measured(|| rate.query(0.25, 0.5, 1, time, 0, 1, width, &mut scratch[..len], &mut w));
        assert!(res.is_err());
        assert_eq!(scratch, before);
        assert_eq!(rate, frozen);
        assert_eq!(
            w.interval_primitive_attempts,
            old.interval_primitive_attempts
        );
        assert!(w.validation_attempts > old.validation_attempts);
        invalid.push(json!({"id":format!("S09-{name}"),"allocations":a,"result":output(&res),"work":work_json(w)}));
    }
    let mut scratch = [p(0.0); 129];
    let res = rate.query(0.5, 0.5, 1, p(1.0), 0, 1, width, &mut scratch, &mut w);
    assert!(matches!(
        res,
        Err(ScalarRateRefusal::BindingMismatch { .. })
    ));
    assert_eq!(rate, frozen);
    invalid.push(json!({"id":"S09-binding","result":output(&res),"work":work_json(w)}));
    let report = json!({"node":"A03-SCALAR-RATE-01","authority":"SCALAR_ARITHMETIC_ONLY","positive_control_allocations":positive,"positive_control_status":if positive==3 {"CALIBRATED"}else{"ALLOCATION_UNAVAILABLE"},"fixtures":rows,"invalid":invalid,"timing_authority":"DIAGNOSTIC_ONLY"});
    if let Ok(path) = std::env::var("VIGILODE_SCALAR_RATE_OUTPUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}
