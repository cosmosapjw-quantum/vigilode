//! `vigilode-bench wp --corpus <file> --out <json>` and
//! `vigilode-bench report --in <json> --out <md>`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::ffi::c_void;
use std::process::{Command, ExitCode};
use std::sync::atomic::Ordering;

use serde_json::{Value, json};
use vigilode_bench::{alloc as counters, corpus::Corpus, ffi, harness, report, source_identity};

const BUILT_SOURCE: &str = include_str!(concat!(env!("OUT_DIR"), "/source_manifest.txt"));

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

fn provenance(source_receipt: Value) -> Value {
    let cpu = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("model name"))
                .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string())
        })
        .unwrap_or_else(|| "unavailable".into());
    let build_clean = env!("BENCH_BUILD_GIT_DIRTY") == "false";
    let build_commit = env!("BENCH_BUILD_GIT_COMMIT");
    json!({
        "git_commit": if build_clean && build_commit != "unavailable" { Some(build_commit) } else { None },
        "git_commit_scope": "build-time context only; source_identity is authoritative",
        "build_git_commit_context": build_commit,
        "build_git_dirty_context": env!("BENCH_BUILD_GIT_DIRTY"),
        "source_identity": source_receipt,
        "build_target": env!("BENCH_BUILD_TARGET"),
        "build_profile": env!("BENCH_BUILD_PROFILE"),
        "build_opt_level": env!("BENCH_BUILD_OPT_LEVEL"),
        "build_debug": env!("BENCH_BUILD_DEBUG"),
        "build_target_features": env!("BENCH_BUILD_CARGO_CFG_TARGET_FEATURE"),
        "build_rustflags": env!("BENCH_BUILD_CARGO_ENCODED_RUSTFLAGS"),
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
            let default_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap();
            let root = args
                .iter()
                .position(|a| a == "--source-root")
                .map(|_| arg(&args, "--source-root").map(std::path::Path::new))
                .transpose()?
                .unwrap_or(default_root);
            let executable = std::env::current_exe().map_err(|e| e.to_string())?;
            let guard = source_identity::RunGuard::begin(
                root,
                BUILT_SOURCE,
                std::path::Path::new(corpus_path),
                &executable,
                text.as_bytes(),
                args.iter().any(|a| a == "--allow-dirty-source"),
            )?;
            if args.iter().any(|a| a == "--require-independent-reference")
                && corpus.reference.independent_protocol.is_none()
            {
                return Err("independent RADAU5/CVODE refinement protocol required by CLI".into());
            }
            let mut log = |s: &str| eprintln!("{s}");
            let mut doc = harness::run_corpus(&corpus, &mut log)?;
            let git = guard.finish()?;
            let source_receipt = json!({
                "schema": source_identity::SCHEMA,
                "status": "VERIFIED_BUILD_SOURCE",
                "manifest_sha256": source_identity::sha256_bytes(BUILT_SOURCE.as_bytes())?,
                "manifest": BUILT_SOURCE,
                "binary_sha256": guard.binary_sha256,
                "corpus_sha256": guard.corpus_sha256,
                "verified_boundaries": ["before_run", "after_run"],
                "git_available": git.is_some(),
                "git_dirty": git.as_ref().map(|c| !c.1.is_empty()),
                "dirty_source_allowed": guard.dirty_source_allowed,
                "scope": "repository crates, manifests/locks, fixtures, literal compile-time assets and native bridge sources; registry dependencies trusted through Cargo.lock checksum verification; system shared-library bytes are not build-source inputs",
            });
            doc["provenance"] = provenance(source_receipt);
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
            "usage: vigilode-bench wp --corpus <file> --out <json> [--source-root <repo>] [--require-independent-reference] [--allow-dirty-source] | report --in <json> --out <md>"
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
