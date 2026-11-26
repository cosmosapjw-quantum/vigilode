//! Library of the V09 measurement program (loop L1 node V09_LAGGED_LARGE_N,
//! `../PREDECLARED.json`): the problem families and the arms. The binaries `v09-runner`
//! (counts, timing, setup, self-tests) and `v09-trace` (exploratory per-attempt trace) use it.

pub mod harness;
pub mod problems;
