//! `vigilode-bench wp --corpus <file> --out <json>` and
//! `vigilode-bench report --in <json> --out <md>`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::ffi::c_void;
use std::process::{Command, ExitCode};
use std::sync::atomic::Ordering;

use serde_json::{Value, json};
use vigilode_bench::{alloc as counters, corpus::Corpus, ffi, harness, report};

/// Counting wrapper around the system allocator (Rust allocations).
struct Counting;

// SAFETY: forwards every call to `System` unchanged; only counts.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        counters::RUST_CALLS.fetch_add(1, Ordering::Relaxed);
        counters::RUST_BYTES.fetch_add(l.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        counters::RUST_CALLS.fetch_add(1, Ordering::Relaxed);
        counters::RUST_BYTES.fetch_add(l.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        counters::RUST_CALLS.fetch_add(1, Ordering::Relaxed);
        counters::RUST_BYTES.fetch_add(n as u64, Ordering::Relaxed);
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

// The C allocator interposer. glibc exports its implementation as
// `__libc_*`; these definitions in the executable take precedence over libc's
// for every object of the link and (exported by build.rs) for shared
// libraries. They count and forward; they never allocate themselves.
unsafe extern "C" {
    fn __libc_malloc(size: usize) -> *mut c_void;
    fn __libc_calloc(n: usize, size: usize) -> *mut c_void;
    fn __libc_realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn __libc_memalign(align: usize, size: usize) -> *mut c_void;
}

#[inline]
fn count(bytes: usize) {
    counters::HEAP_CALLS.fetch_add(1, Ordering::Relaxed);
    counters::HEAP_BYTES.fetch_add(bytes as u64, Ordering::Relaxed);
}

/// # Safety
/// C `malloc` contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
    count(size);
    unsafe { __libc_malloc(size) }
}

/// # Safety
/// C `calloc` contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn calloc(n: usize, size: usize) -> *mut c_void {
    count(n.saturating_mul(size));
    unsafe { __libc_calloc(n, size) }
}

/// # Safety
/// C `realloc` contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn realloc(p: *mut c_void, size: usize) -> *mut c_void {
    count(size);
    unsafe { __libc_realloc(p, size) }
}

/// # Safety
/// C `memalign` contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memalign(align: usize, size: usize) -> *mut c_void {
    count(size);
    unsafe { __libc_memalign(align, size) }
}

/// # Safety
/// C11 `aligned_alloc` contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aligned_alloc(align: usize, size: usize) -> *mut c_void {
    count(size);
    unsafe { __libc_memalign(align, size) }
}

/// # Safety
/// POSIX `posix_memalign` contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_memalign(out: *mut *mut c_void, align: usize, size: usize) -> i32 {
    const EINVAL: i32 = 22;
    const ENOMEM: i32 = 12;
    if !align.is_power_of_two() || !align.is_multiple_of(std::mem::size_of::<*mut c_void>()) {
        return EINVAL;
    }
    count(size);
    let p = unsafe { __libc_memalign(align, size) };
    if p.is_null() && size != 0 {
        return ENOMEM;
    }
    unsafe { *out = p };
    0
}

/// Verify that both counters move before any measurement: one Rust `Box`
/// (global allocator, then `malloc` through `System`) and one direct call of
/// the interposed `malloc`.
fn verify_counters() -> bool {
    let a = counters::snapshot();
    let b = std::hint::black_box(Box::new([0u8; 64]));
    drop(b);
    let p = unsafe { malloc(32) };
    unsafe { libc_free(p) };
    let z = counters::snapshot().since(a);
    let ok = z.rust_calls >= 1 && z.heap_calls >= 2;
    counters::MEASURED.store(ok, Ordering::SeqCst);
    ok
}

unsafe extern "C" {
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

fn cmd(args: &[&str]) -> String {
    Command::new(args[0])
        .args(&args[1..])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unavailable".into())
}

fn provenance() -> Value {
    let dir = env!("CARGO_MANIFEST_DIR");
    let cpu = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("model name"))
                .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string())
        })
        .unwrap_or_else(|| "unavailable".into());
    let dirty = cmd(&[
        "git",
        "-C",
        dir,
        "status",
        "--porcelain",
        "--untracked-files=no",
        "--",
        ".",
    ]);
    json!({
        "git_commit": cmd(&["git", "-C", dir, "rev-parse", "HEAD"]),
        "git_crate_tree_dirty": !dirty.is_empty() && dirty != "unavailable",
        "crate_version": env!("CARGO_PKG_VERSION"),
        "rustc": env!("BENCH_RUSTC_VERSION"),
        "gfortran": env!("BENCH_GFORTRAN_VERSION"),
        "fflags_f77": env!("BENCH_FFLAGS_F77"),
        "fflags_f90": env!("BENCH_FFLAGS_F90"),
        "cflags": env!("BENCH_CFLAGS"),
        "rodas_localized_symbols": env!("BENCH_RODAS_LOCALIZED"),
        "sundials_header_version": ffi::sundials_version(),
        "blas_library": std::fs::canonicalize("/usr/lib/x86_64-linux-gnu/libblas.so.3").map(|p| p.display().to_string()).unwrap_or_else(|_| "unavailable".into()),
        "diffsol": "0.16.1 (Cargo.lock)",
        "profile": if cfg!(debug_assertions) { "debug" } else { "optimized" },
        "host_cpu": cpu,
        "host_logical_cpus": std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0),
        "host_kernel": cmd(&["uname", "-sr"]),
        "sources": serde_json::from_str::<Value>(include_str!("../native/SOURCES.json")).unwrap_or(Value::Null),
    })
}

fn arg<'a>(args: &'a [String], flag: &str) -> Result<&'a str, String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
        .ok_or_else(|| format!("missing {flag} <value>"))
}

fn real_main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("wp") => {
            let corpus_path = arg(&args, "--corpus")?;
            let out = arg(&args, "--out")?;
            if !verify_counters() {
                return Err("allocation counters did not move; refusing to run".into());
            }
            let text = std::fs::read_to_string(corpus_path).map_err(|e| e.to_string())?;
            let corpus = Corpus::from_json(&text)?;
            let mut log = |s: &str| eprintln!("{s}");
            let mut doc = harness::run_corpus(&corpus, &mut log)?;
            doc["provenance"] = provenance();
            doc["corpus_file"] = json!(corpus_path);
            let s = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
            std::fs::write(out, s + "\n").map_err(|e| e.to_string())
        }
        Some("report") => {
            let input = arg(&args, "--in")?;
            let out = arg(&args, "--out")?;
            let text = std::fs::read_to_string(input).map_err(|e| e.to_string())?;
            let doc: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
            let md = report::render(&doc)?;
            std::fs::write(out, md).map_err(|e| e.to_string())
        }
        _ => Err(
            "usage: vigilode-bench wp --corpus <file> --out <json> | report --in <json> --out <md>"
                .into(),
        ),
    }
}

fn main() -> ExitCode {
    match real_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("vigilode-bench: {e}");
            ExitCode::FAILURE
        }
    }
}
