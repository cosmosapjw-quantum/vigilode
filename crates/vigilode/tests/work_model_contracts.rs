//! Comprehensive unit and contract tests for E1 WorkModel v2.

use rodas5p_core::WorkCounters;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use vigilode::work_model::*;
use vigilode::{
    Options, PolynomialWeight, Problem, RhsCost, SwitchingConfig, SwitchingSegment,
    SwitchingSolver, WorkModelVersion, integer_work_v2, integer_work_v2_from_counters,
    strict_entry_work_preferred, strict_return_work_preferred,
};

thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
}

struct CountingAlloc;

fn count_allocation() {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = COUNT.try_with(|count| count.set(count.get() + 1));
    }
}

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count_allocation();
        unsafe { System.realloc(ptr, layout, size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAlloc = CountingAlloc;

fn measured<T>(f: impl FnOnce() -> T) -> (T, usize) {
    COUNT.with(|count| count.set(0));
    TRACK.with(|track| track.set(true));
    let result = f();
    TRACK.with(|track| track.set(false));
    (result, COUNT.with(Cell::get))
}

fn dummy_segment() -> SwitchingSegment {
    let problem = Problem::new(1, |_, _, out| {
        out[0] = 0.0;
        Ok(())
    });
    let mut seg = SwitchingSolver::solve(
        &problem,
        (0.0, 0.0),
        &[0.0],
        &Options::default(),
        &SwitchingConfig::default(),
    )
    .unwrap()
    .stats
    .segments
    .remove(0);
    seg.rhs_calls = 10;
    seg.jacobian_builds = 1;
    seg.jvp_vectors = 2;
    seg.ft_calls = 3;
    seg.linear_solves = 4;
    seg.start_time = 0.0;
    seg.target_time = 1.0;
    seg.reached_time = 1.0;
    seg.accepted_steps = 1;
    seg.success = true;
    seg
}

#[test]
fn polynomial_weight_exactness_and_powers() {
    let p = PolynomialWeight::new(5, 7, 11, 13);
    assert_eq!(p.eval(0), Some(5));
    // 5 + 7*1 + 11*1 + 13*1 = 36
    assert_eq!(p.eval(1), Some(36));
    // 5 + 7*2 + 11*4 + 13*8 = 5 + 14 + 44 + 104 = 167
    assert_eq!(p.eval(2), Some(167));
    // 5 + 7*10 + 11*100 + 13*1000 = 5 + 70 + 1100 + 13000 = 14175
    assert_eq!(p.eval(10), Some(14175));
}

#[test]
fn polynomial_weight_monotonicity() {
    let p = PolynomialWeight::new(100, 20, 5, 2);
    let mut prev = p.eval(0).unwrap();
    for n in 1..=200 {
        let curr = p.eval(n).unwrap();
        assert!(
            curr >= prev,
            "Monotonicity violated at n={n}: {curr} < {prev}"
        );
        prev = curr;
    }
}

#[test]
fn polynomial_weight_overflow_protection() {
    let huge = PolynomialWeight::new(u64::MAX, u64::MAX, u64::MAX, u64::MAX);
    // At n = usize::MAX, u128 arithmetic must detect overflow and safely return None
    assert_eq!(huge.eval(usize::MAX), None);
    assert_eq!(huge.eval(1 << 40), None);
}

#[test]
fn frozen_weights_match_toml_source() {
    let toml_str: &str = include_str!("../data/work_model_v2.toml");

    let mut current_section = "";
    let mut denom: Option<u64> = None;
    let mut weights = std::collections::BTreeMap::<String, [u64; 4]>::new();

    for line in toml_str.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            current_section = &line[1..line.len() - 1];
            weights.insert(current_section.to_string(), [0, 0, 0, 0]);
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim();
            let v_str = v.split('#').next().unwrap().trim();
            let val: u64 = v_str
                .parse()
                .unwrap_or_else(|_| panic!("Failed to parse integer from {v_str}"));
            if k == "denominator" {
                denom = Some(val);
            } else if !current_section.is_empty() {
                let entry = weights.get_mut(current_section).unwrap();
                match k {
                    "a0" => entry[0] = val,
                    "a1" => entry[1] = val,
                    "a2" => entry[2] = val,
                    "a3" => entry[3] = val,
                    other => panic!("Unknown key {other} in section [{current_section}]"),
                }
            }
        }
    }

    assert_eq!(
        denom,
        Some(WORK_MODEL_DENOMINATOR),
        "Denominator mismatch between TOML and code constant"
    );

    let check = |name: &str, pw: PolynomialWeight| {
        let arr = weights
            .get(name)
            .unwrap_or_else(|| panic!("Missing section [{name}] in work_model_v2.toml"));
        assert_eq!(
            pw,
            PolynomialWeight::new(arr[0], arr[1], arr[2], arr[3]),
            "Weights mismatch for event {name}"
        );
    };

    check("linear_solves", FROZEN_WEIGHTS.linear_solves);
    check("linear_iterations", FROZEN_WEIGHTS.linear_iterations);
    check(
        "orthogonalization_inner_products",
        FROZEN_WEIGHTS.orthogonalization_inner_products,
    );
    check(
        "orthogonalization_vector_updates",
        FROZEN_WEIGHTS.orthogonalization_vector_updates,
    );
    check(
        "direct_factorizations",
        FROZEN_WEIGHTS.direct_factorizations,
    );
    check("direct_solve_calls", FROZEN_WEIGHTS.direct_solve_calls);
    check("phi_krylov_vectors", FROZEN_WEIGHTS.phi_krylov_vectors);
    check(
        "phi_projected_exponentials",
        FROZEN_WEIGHTS.phi_projected_exponentials,
    );
    check("preconditioner_apps", FROZEN_WEIGHTS.preconditioner_apps);
}

#[test]
fn rhs_cost_ir_scaling() {
    assert_eq!(RhsCost::Cheap.ir_cost(), 50);
    assert_eq!(RhsCost::Moderate.ir_cost(), 500);
    assert_eq!(RhsCost::Expensive.ir_cost(), 5000);
    assert_eq!(RhsCost::DeclaredIr(4242).ir_cost(), 4242);
    assert_eq!(RhsCost::default(), RhsCost::Cheap);
}

#[test]
fn compute_overhead_ir_calculation() {
    let empty = WorkCounters::default();
    assert_eq!(compute_overhead_ir(&empty, 10), Some(0));

    let counters = WorkCounters {
        direct_solve_calls: 1,
        ..Default::default()
    };
    // direct_solve_calls weight: a2 = 131072, over denominator 65536 is exactly 2.0 * n^2
    // For n = 10: 2 * 100 = 200 Ir.
    assert_eq!(compute_overhead_ir(&counters, 10), Some(200));

    // Overflow protection on compute_overhead_ir
    let huge = WorkCounters {
        direct_factorizations: u64::MAX,
        ..Default::default()
    };
    assert_eq!(compute_overhead_ir(&huge, 1_000_000), None);
}

#[test]
fn integer_work_v2_zero_allocation() {
    let segment = dummy_segment();
    // Warmup
    let _ = integer_work_v2(&segment, 16, RhsCost::Cheap);

    // Measure allocations in hot path
    let (work, allocs) = measured(|| integer_work_v2(&segment, 16, RhsCost::Cheap));
    assert!(work.is_some());
    assert_eq!(allocs, 0, "integer_work_v2 must not allocate on the heap");
}

#[test]
fn integer_work_v2_from_counters_matches_segment() {
    let counters = WorkCounters {
        rhs_calls: 10,
        jacobian_builds: 1,
        jvp_vectors: 2,
        ft_calls: 3,
        linear_solves: 4,
        ..Default::default()
    };

    let cost_from_c = integer_work_v2_from_counters(&counters, 16, RhsCost::Moderate);
    assert!(cost_from_c.is_some());

    let mut seg = dummy_segment();
    seg.rhs_calls = 10;
    seg.jacobian_builds = 1;
    seg.jvp_vectors = 2;
    seg.ft_calls = 3;
    seg.linear_solves = 4;

    let cost_from_s = integer_work_v2(&seg, 16, RhsCost::Moderate);
    assert_eq!(cost_from_c, cost_from_s);
}

#[test]
fn strict_preference_dispatch_by_work_model() {
    let mut incumbent = dummy_segment();
    let mut candidate = dummy_segment();

    incumbent.rhs_calls = 100;
    candidate.rhs_calls = 90;

    // Both V1 and V2 should prefer candidate here
    assert!(strict_entry_work_preferred(
        &incumbent,
        &candidate,
        10,
        WorkModelVersion::V1,
        RhsCost::Cheap,
    ));
    assert!(strict_entry_work_preferred(
        &incumbent,
        &candidate,
        10,
        WorkModelVersion::V2,
        RhsCost::Cheap,
    ));

    // When candidate has higher cost, candidate is not preferred
    candidate.rhs_calls = 110;
    assert!(!strict_entry_work_preferred(
        &incumbent,
        &candidate,
        10,
        WorkModelVersion::V1,
        RhsCost::Cheap,
    ));
    assert!(!strict_entry_work_preferred(
        &incumbent,
        &candidate,
        10,
        WorkModelVersion::V2,
        RhsCost::Cheap,
    ));

    // Return preference dispatch test
    let mut ret_incumbent = dummy_segment();
    let mut ret_candidate = dummy_segment();
    ret_incumbent.rhs_calls = 50;
    ret_candidate.rhs_calls = 40;
    assert!(strict_return_work_preferred(
        &ret_incumbent,
        &ret_candidate,
        10,
        WorkModelVersion::V1,
        RhsCost::Cheap,
    ));
    assert!(strict_return_work_preferred(
        &ret_incumbent,
        &ret_candidate,
        10,
        WorkModelVersion::V2,
        RhsCost::Cheap,
    ));
}
