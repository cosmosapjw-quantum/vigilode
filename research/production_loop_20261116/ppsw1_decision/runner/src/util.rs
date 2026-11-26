//! Helpers of the measurement program: hashes, host readings, comparisons.

use rodas5p_core::WorkCounters;

use crate::driver::PeerRun;

/// FNV-1a 64 over the bits of a sequence of `f64`.
#[derive(Clone, Copy)]
pub struct Fnv(u64);

impl Default for Fnv {
    fn default() -> Self {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
}

impl Fnv {
    pub fn f(&mut self, x: f64) {
        for b in x.to_bits().to_le_bytes() {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    pub fn all(&mut self, xs: &[f64]) {
        for &x in xs {
            self.f(x);
        }
    }
    pub fn hex(self) -> String {
        format!("{:016x}", self.0)
    }
}

/// Hash of every output time and state of a run.
pub fn hash_outputs(t: &[f64], y: &[Vec<f64>]) -> String {
    let mut h = Fnv::default();
    h.all(t);
    for v in y {
        h.all(v);
    }
    h.hex()
}

/// Hash of one state vector.
pub fn hash_vec(v: &[f64]) -> String {
    let mut h = Fnv::default();
    h.all(v);
    h.hex()
}

/// The 1-minute load average from /proc/loadavg (NaN if unreadable).
pub fn load1() -> f64 {
    std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|s| s.split_whitespace().next().map(str::to_string))
        .and_then(|s| s.parse().ok())
        .unwrap_or(f64::NAN)
}

/// The full /proc/loadavg line.
pub fn loadavg_line() -> String {
    std::fs::read_to_string("/proc/loadavg")
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// `Cpus_allowed_list` of this process.
pub fn cpus_allowed() -> String {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("Cpus_allowed_list"))
                .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string())
        })
        .unwrap_or_default()
}

/// Bitwise equality of two `f64` slices.
pub fn bits_eq(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}

/// Bitwise equality of two state lists.
pub fn states_eq(a: &[Vec<f64>], b: &[Vec<f64>]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| bits_eq(x, y))
}

/// Bitwise equality of two research runs in every field the identity checks compare
/// (the attempt trace and the cache counters are diagnostics and not compared).
pub fn runs_eq(a: &PeerRun, b: &PeerRun) -> bool {
    bits_eq(&a.t, &b.t)
        && states_eq(&a.y, &b.y)
        && a.success == b.success
        && a.h0.to_bits() == b.h0.to_bits()
        && a.starter.h0_requested.to_bits() == b.starter.h0_requested.to_bits()
        && a.starter.halvings == b.starter.halvings
        && a.starter.estimate.to_bits() == b.starter.estimate.to_bits()
        && a.starter.covers_span == b.starter.covers_span
        && a.startup_work == b.startup_work
        && a.step_work == b.step_work
        && a.attempts == b.attempts
}

/// Step attempts of a work record (accepted + rejected).
pub fn attempts_of(w: &WorkCounters) -> u64 {
    w.accepted_steps + w.rejected_steps
}
