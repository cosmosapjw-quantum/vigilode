//! Research node `research/storage_runtime_20261004` (RVJ DAG node
//! STORAGE-RUNTIME): live and peak heap bytes of `shared_shift_jet` under a
//! counting global allocator, against its explicit scalar-slot bound. Run
//! with one test thread; the allocator counts every thread.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use rodas5p_core::shared_shift_jet::{SharedShiftJetConfig, shared_shift_jet};
use rodas5p_core::{DenseMatrix, LuFactorization};
use serde_json::{Value, json};

struct Tracking;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static COUNT: AtomicUsize = AtomicUsize::new(0);
static LARGEST: AtomicUsize = AtomicUsize::new(0);

fn grow(size: usize) {
    let live = LIVE.fetch_add(size, Ordering::SeqCst) + size;
    PEAK.fetch_max(live, Ordering::SeqCst);
    COUNT.fetch_add(1, Ordering::SeqCst);
    LARGEST.fetch_max(size, Ordering::SeqCst);
}

unsafe impl GlobalAlloc for Tracking {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        grow(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        grow(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        LIVE.fetch_sub(layout.size(), Ordering::SeqCst);
        grow(new_size);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::SeqCst);
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Tracking = Tracking;

/// Peak live bytes above the entry level, allocation count and largest
/// single allocation during `f`.
fn measure<T>(f: impl FnOnce() -> T) -> (T, usize, usize, usize) {
    let entry = LIVE.load(Ordering::SeqCst);
    PEAK.store(entry, Ordering::SeqCst);
    let count = COUNT.load(Ordering::SeqCst);
    LARGEST.store(0, Ordering::SeqCst);
    let value = f();
    let peak = PEAK.load(Ordering::SeqCst) - entry;
    (
        value,
        peak,
        COUNT.load(Ordering::SeqCst) - count,
        LARGEST.load(Ordering::SeqCst),
    )
}

struct SplitMix(u64);

impl SplitMix {
    fn step(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    fn symmetric(&mut self) -> f64 {
        2.0 * ((self.step() >> 11) as f64 / (1u64 << 53) as f64) - 1.0
    }
}

fn dissipative(n: usize, seed: u64) -> DenseMatrix {
    let mut rng = SplitMix(seed);
    let s: Vec<f64> = (0..n * n).map(|_| rng.symmetric()).collect();
    let mut j = DenseMatrix::zeros(n, n);
    for i in 0..n {
        let mut row = 0.0;
        for k in 0..n {
            row += (0.5 * (s[i * n + k] + s[k * n + i])).abs();
        }
        for k in 0..n {
            j[(i, k)] = s[i * n + k];
        }
        j[(i, i)] -= row + 0.1;
    }
    j
}

const H: f64 = 0.1;
const GAMMA0: f64 = 0.25;

fn targets(m: usize) -> Vec<f64> {
    (0..m)
        .map(|i| {
            let t = if m == 1 {
                0.0
            } else {
                -1.0 + 2.0 * i as f64 / (m - 1) as f64
            };
            GAMMA0 * (1.0 + 0.2 * t)
        })
        .collect()
}

#[test]
#[ignore = "recorded run of research/storage_runtime_20261004; one test thread"]
fn export_storage_sweep() {
    let mut rows: Vec<Value> = Vec::new();
    let mut g1 = true;
    let mut g2 = true;
    for n in [4usize, 16, 64, 128] {
        let j = dissipative(n, 5000 + n as u64);
        let mut center = DenseMatrix::identity(n);
        for i in 0..n {
            for k in 0..n {
                center[(i, k)] -= GAMMA0 * H * j[(i, k)];
            }
        }
        for r in [1usize, 4] {
            let mut rng = SplitMix(6000 + n as u64 + r as u64);
            let rhs: Vec<Vec<f64>> = (0..r)
                .map(|_| (0..n).map(|_| rng.symmetric()).collect())
                .collect();
            let ((), p_lu, lu_allocs, _) = measure(|| {
                let lu = LuFactorization::new(&center).unwrap();
                let x = lu.solve_rows(&rhs).unwrap();
                assert_eq!(x.len(), r);
            });
            for degree in [0usize, 8, 24] {
                for m in [1usize, 17, 65] {
                    let gammas = targets(m);
                    let config = SharedShiftJetConfig {
                        degree,
                        absolute_tolerance: 1.0e-10,
                        max_stored_scalars: usize::MAX,
                        max_work_units: usize::MAX,
                    };
                    let (report, p_total, allocs, largest) =
                        measure(|| shared_shift_jet(&j, H, GAMMA0, &rhs, &gammas, config).unwrap());
                    let work = report.work().clone();
                    let explicit_bytes = 8 * work.explicit_storage_upper_scalars_excluding_lu;
                    let allowed = explicit_bytes + 64 * allocs;
                    let non_lu = p_total.saturating_sub(p_lu);
                    let ok1 = non_lu <= allowed;
                    g1 &= ok1;
                    drop(report);
                    // G2: one below the requirement, for each budget.
                    let mut refusals = Vec::new();
                    for which in ["storage", "work"] {
                        let mut tight = config;
                        if which == "storage" {
                            tight.max_stored_scalars =
                                work.explicit_storage_upper_scalars_excluding_lu - 1;
                        } else {
                            tight.max_work_units = work.planned_work_units - 1;
                        }
                        let (result, _, refusal_allocs, refusal_largest) =
                            measure(|| shared_shift_jet(&j, H, GAMMA0, &rhs, &gammas, tight));
                        let ok2 = result.is_err() && refusal_allocs <= 1 && refusal_largest <= 256;
                        g2 &= ok2;
                        refusals.push(json!({
                            "budget": which, "refused": result.is_err(),
                            "allocations": refusal_allocs, "largest_bytes": refusal_largest, "ok": ok2,
                        }));
                    }
                    rows.push(json!({
                        "n": n, "r": r, "degree": degree, "m": m,
                        "p_total_bytes": p_total,
                        "p_lu_bytes": p_lu,
                        "lu_allocations": lu_allocs,
                        "explicit_bytes": explicit_bytes,
                        "allocations": allocs,
                        "largest_allocation_bytes": largest,
                        "non_lu_bytes": non_lu,
                        "g1_allowed_bytes": allowed,
                        "g1_ok": ok1,
                        "p_lu_over_8n2": p_lu as f64 / (8 * n * n) as f64,
                        "p_total_over_explicit": p_total as f64 / explicit_bytes as f64,
                        "refusals": refusals,
                    }));
                }
            }
        }
    }
    let out = json!({
        "schema": "vigilode-storage-runtime-v1",
        "note": "explicit_storage_upper_scalars_excluding_lu is a bound on explicit f64 slots, not an RSS cap",
        "gate": {"g1_explicit_covers_non_lu": g1, "g2_refusal_allocates_only_message": g2},
        "verdict": if g1 && g2 { "PASS" } else { "FAIL" },
        "rows": rows,
    });
    if let Ok(path) = std::env::var("STORAGE_RUNTIME_OUTPUT") {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        assert!(
            !path.exists(),
            "immutable output exists: {}",
            path.display()
        );
        std::fs::write(&path, serde_json::to_string_pretty(&out).unwrap() + "\n").unwrap();
        println!("wrote {}", path.display());
    }
    println!("{}", out["gate"]);
}
