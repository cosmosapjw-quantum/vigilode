"""Single source for task edits, tests and machine fields of the remediation design.

build.py renders DESIGN.md and TASKS.json from this file and replays every edit on
the base commit in DAG order, so the document cannot drift from the dry-run.
"""

BASE = "4339facc62258b5fb9a2469f16a3af8b93763e24"
SW = "crates/vigilode/src/switching.rs"
RG = "crates/vigilode/src/registry.rs"
ADM = "crates/vigilode/tests/switcher_v2_admission_contracts.rs"
CT = "crates/vigilode/tests/switching_v2_remediation_contracts.rs"
ECON = "crates/vigilode/tests/switching_economy_contracts.rs"
DOC_DIR = "docs/agent/switcher_v2_remediation"

# Commands (exact). The gate runs them with `sh -c` from the repository root.
C_CONTRACTS = {"name": "contracts", "cmd": "cargo test --locked -p vigilode --release --test switching_v2_remediation_contracts", "timeout_s": 1800}
C_LIB = {"name": "lib-unit-tests", "cmd": "cargo test --locked -p vigilode --release --lib", "timeout_s": 1800}
C_FULL = {"name": "vigilode-suite", "cmd": "cargo test --locked -p vigilode --release --no-fail-fast", "timeout_s": 3600}
C_CLIPPY = {"name": "clippy", "cmd": "cargo clippy --locked -p vigilode --all-targets --release --no-deps -- -D warnings", "timeout_s": 1800}
C_FMT = {"name": "fmt", "cmd": "cargo fmt -p vigilode -- --check", "timeout_s": 300}
C_NODE = {"name": "research-node-check", "cmd": "python3 tools/check-research-node.py", "timeout_s": 600}
C_INTEG_TESTS = {"name": "integrators-tests", "cmd": "cargo test --locked -p rodas5p-integrators --release", "timeout_s": 3600}
C_INTEG_CLIPPY = {"name": "integrators-clippy", "cmd": "cargo clippy --locked -p rodas5p-integrators --all-targets --release --no-deps -- -D warnings", "timeout_s": 1800}
C_INTEG_FMT = {"name": "integrators-fmt", "cmd": "cargo fmt -p rodas5p-integrators -- --check", "timeout_s": 300}
C_KRYLOV_TESTS = {"name": "krylov-tests", "cmd": "cargo test --locked -p rodas5p-krylov --release", "timeout_s": 3600}
C_KRYLOV_CLIPPY = {"name": "krylov-clippy", "cmd": "cargo clippy --locked -p rodas5p-krylov --all-targets --release --no-deps -- -D warnings", "timeout_s": 1800}

STD_CMDS = [C_CONTRACTS, C_LIB, C_FULL, C_CLIPPY, C_FMT]


def mutation(package="vigilode", test_args=None, max_missed=0):
    return {
        "kind": "mutation",
        "package": package,
        "cargo_test_args": test_args if test_args is not None else ["--test", "switching_v2_remediation_contracts", "--lib"],
        "max_missed": max_missed,
        "jobs": 1,
        "timeout_s": 600,
    }


INVARIANTS = [
    ("INV-01", "Default-path neutrality: `SwitchingConfig::evidence_driven`, `SwitchingConfig::default()` and the direct Rodas5p/Dopri5 solves keep bit-identical states and identical counters on the registered cells, unless the task kind is default_behaviour. Evidence: `skeleton_helpers_reproduce_registered_counters` stays green."),
    ("INV-02", "research/LEDGER.jsonl is append-only: one JSON object per line with the ten fields of docs/RESEARCH_LEDGER.md. Existing rows use mixed key orders and separators; new rows are written by the row script of DESIGN.md 13.2."),
    ("INV-03", "Units: a guard ceiling is in callback units (RHS 1, JVP 1 per vector, dense Jacobian n). WorkModel V2 values are instruction units (Ir). Convert V2 to a callback ceiling only through `v2_entry_callback_cap` / `v2_return_callback_cap`."),
    ("INV-04", "Decision preservation: an early stop armed by a ceiling never changes a strict entry/return decision compared with the same comparison run without the ceiling."),
    ("INV-05", "Determinism: no wall clock, randomness, thread timing or HashMap iteration order in any decision path; epoch digests use the FNV-1a hasher after F7."),
    ("INV-06", "Accounting completeness: every user-callback evaluation made by the library is added to `stats.rhs_calls` / `stats.jvp_vectors` (or the matching counter)."),
    ("INV-07", "Scope: only files listed in the task's files_allowed (plus always_allowed) change."),
    ("INV-08", "Public API is additive: no public item is removed, renamed or changes signature unless the task says so."),
    ("INV-09", "Opt-in: new behaviour is reachable only through the named factory or a config field whose default keeps the old behaviour (task kinds opt_in_behaviour, experiment)."),
    ("INV-10", "Tests first: every behavioural test of the task fails on the task's base commit (`agent_gate.py red`) and passes after the change."),
    ("INV-11", "No weakening: existing assertions, tolerances and test functions are not deleted or relaxed, except the exact lines listed in the task's removals_allowed."),
    ("INV-12", "No new dependencies, toolchain or lockfile changes; no unsafe code; no lint suppression attributes."),
    ("INV-13", "Research claims: a verdict comes only from the preregistered gate; PREREGISTRATION.md is never edited above its results section; a ledger row whose commit is not reachable from a remote is not written."),
    ("INV-14", "Exactness: anchors are matched as exact text with the stated occurrence count; never edit by line number or by approximate match."),
    ("INV-15", "Traceability: every finished task has one commit with the task's commit_message, a PROGRESS.md entry and a passing gate record in .agent/STATE.json."),
]

GLOBAL = {
    "forbidden_paths": [
        "Cargo.lock", "Cargo.toml", "crates/*/Cargo.toml", "rust-toolchain.toml", ".cargo/*",
        ".github/*", "CLAUDE.md", ".claude/*", "tools/agent_gate.py",
        DOC_DIR + "/DESIGN.md", DOC_DIR + "/TASKS.json", DOC_DIR + "/TASKS.schema.json",
        DOC_DIR + "/build/*", DOC_DIR + "/reference/*",
        "crates/vigilode/tests/golden/*", "fixtures/*",
    ],
    "always_allowed": [".agent/*", DOC_DIR + "/PROGRESS.md"],
    "append_only": ["research/LEDGER.jsonl", DOC_DIR + "/PROGRESS.md"],
    "forbidden_added_patterns": [
        {"regex": r"\bunsafe\b", "paths": ["crates/*.rs"], "reason": "INV-12 no unsafe code"},
        {"regex": r"#!?\[allow\(", "paths": ["crates/*.rs"], "reason": "INV-12 no lint suppression"},
        {"regex": r"#\[ignore", "paths": ["crates/*.rs"], "reason": "INV-11 no ignored tests"},
        {"regex": r"\b(todo|unimplemented|dbg)!\(", "paths": ["crates/*.rs"], "reason": "no placeholders or debug output"},
        {"regex": r"\b(e?println)!\(", "paths": ["crates/*/src/*.rs"], "reason": "no printing in library code"},
        {"regex": r"\.unwrap\(\)", "paths": ["crates/*/src/*.rs"], "reason": "code under src (unit tests included) uses expect(\"reason\") instead of unwrap()"},
        {"regex": r"(Instant::now|SystemTime|thread_rng|rand::)", "paths": ["crates/*/src/*.rs"], "reason": "INV-05 determinism"},
        {"regex": r"collections::hash_map::DefaultHasher", "paths": ["crates/vigilode/src/*.rs"], "reason": "INV-05 epochs use Fnv1a64"},
        {"regex": r"assert!\(\s*true\s*\)", "paths": ["crates/*.rs"], "reason": "vacuous assertion"},
    ],
    "forbidden_removed_patterns": [
        {"regex": r"^\s*#\[test\]", "paths": ["crates/*.rs"], "reason": "INV-11 removing a test"},
        {"regex": r"\bassert(_eq|_ne)?!", "paths": ["crates/*/tests/*.rs", "crates/*/src/*.rs"], "reason": "INV-11 removing or rewriting an assertion"},
        {"regex": r".", "paths": ["research/*/PREREGISTRATION.md"], "reason": "INV-13 preregistrations are append-only"},
    ],
}

# --------------------------------------------------------------------------- edits
# Each edit: (file, find_text, replace_text, expected_count). Applied in order.

F1_OLD = """    pub fn adaptive_evidence(fallback: DerivativeFallback) -> Self {
        Self {
            entry_probe_adaptation: Some(EntryProbeAdaptation {
                loss_margin_backoff: true,
                ..EntryProbeAdaptation::default()
            }),"""
F1_NEW = """    pub fn adaptive_evidence(fallback: DerivativeFallback) -> Self {
        Self {
            entry_probe_adaptation: Some(EntryProbeAdaptation::default()),"""

F2_E1_OLD = """    let cost = match work_model {
        WorkModelVersion::V1 => integer_work(incumbent, dimension, false)?,
        WorkModelVersion::V2 => integer_work_v2(incumbent, dimension, rhs_cost)?,
    };
    let ceiling = cost.saturating_sub(1);"""
F2_E1_NEW = """    let ceiling = match work_model {
        WorkModelVersion::V1 => integer_work(incumbent, dimension, false)?.saturating_sub(1),
        WorkModelVersion::V2 => v2_entry_callback_cap(
            integer_work_v2(incumbent, dimension, rhs_cost)?,
            rhs_cost.ir_cost(),
        )?,
    };"""
F2_E2_OLD = """        WorkModelVersion::V2 => integer_work_v2(candidate, dimension, rhs_cost)?,
    };
    (cost < u64::MAX as u128).then_some(cost as u64)"""
F2_E2_NEW = """        WorkModelVersion::V2 => v2_return_callback_cap(
            integer_work_v2(candidate, dimension, rhs_cost)?,
            rhs_cost.ir_cost(),
        )?,
    };
    (cost < u64::MAX as u128).then_some(cost as u64)"""
F2_E3_OLD = """fn strict_entry_callback_ceiling("""
F2_E3_NEW = """// WorkModel v2 costs are instruction units: v2 = kappa * callbacks + overhead,
// overhead >= 0. The pilot guard counts callbacks. A pilot whose charged
// callbacks c exceed floor((E - 1) / kappa) has v2 >= kappa * c >= E, so it
// cannot be strictly cheaper than the incumbent E: the cap is decision-preserving.
pub(crate) fn v2_entry_callback_cap(incumbent_v2: u128, kappa: u64) -> Option<u128> {
    if kappa == 0 {
        return None;
    }
    Some(incumbent_v2.saturating_sub(1) / kappa as u128)
}

// An implicit comparator whose charged callbacks L exceed floor(E / kappa) has
// v2 >= kappa * L > E, so the completed explicit candidate E is strictly cheaper.
pub(crate) fn v2_return_callback_cap(candidate_v2: u128, kappa: u64) -> Option<u128> {
    if kappa == 0 {
        return None;
    }
    Some(candidate_v2 / kappa as u128)
}

fn strict_entry_callback_ceiling("""
F2_E4_OLD = """    fn strict_entry_callback_ceiling_is_exact_and_never_truncates_large_costs() {"""
F2_E4_NEW = """    fn v2_entry_callback_cap_matches_boundary_table() {
        for (e, k, cap) in [
            (0u128, 50u64, Some(0u128)),
            (1, 50, Some(0)),
            (50, 50, Some(0)),
            (51, 50, Some(1)),
            (100, 50, Some(1)),
            (101, 50, Some(2)),
            (7, 1, Some(6)),
            (10, 0, None),
        ] {
            assert_eq!(super::v2_entry_callback_cap(e, k), cap, "e={e} kappa={k}");
        }
    }

    #[test]
    fn v2_return_callback_cap_matches_boundary_table() {
        for (e, k, cap) in [
            (0u128, 50u64, Some(0u128)),
            (49, 50, Some(0)),
            (50, 50, Some(1)),
            (100, 50, Some(2)),
            (7, 1, Some(7)),
            (10, 0, None),
        ] {
            assert_eq!(super::v2_return_callback_cap(e, k), cap, "e={e} kappa={k}");
        }
    }

    #[test]
    fn v2_caps_are_decision_preserving() {
        for kappa in [1u64, 7, 50, 500] {
            for e in 0u128..400 {
                let entry = super::v2_entry_callback_cap(e, kappa).expect("kappa > 0");
                let ret = super::v2_return_callback_cap(e, kappa).expect("kappa > 0");
                for overhead in [0u128, 1, 13, 1000] {
                    for c in 0u128..60 {
                        let pilot = kappa as u128 * c + overhead;
                        if c > entry {
                            assert!(pilot >= e, "entry cap {entry} would stop a winning pilot");
                        }
                        if c > ret {
                            assert!(
                                pilot > e,
                                "return cap {ret} would stop before E < I is proved"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn strict_entry_callback_ceiling_is_exact_and_never_truncates_large_costs() {"""

F3_OLD = """            work_model: WorkModelVersion::V2,
            progress_bound: true,"""
F3_NEW = """            work_model: WorkModelVersion::V1,
            progress_bound: true,"""
F3_ADM_OLD = """    assert_eq!(cfg.work_model, WorkModelVersion::V2);\n"""
F3_ADM_NEW = """    assert_eq!(cfg.work_model, WorkModelVersion::V1);\n"""

F4_E1_OLD = """/// Progress model for the progress-bound screen (Section 8.2)."""
F4_E1_NEW = """/// Minimum guard-callback units charged by any accepted RODAS5P attempt:
/// stages 2..=8 each evaluate the RHS once (stage 1 may reuse f(y_n)).
pub const RODAS5P_MIN_CALLBACKS_PER_ACCEPTED_ATTEMPT: u64 = 7;

/// Upper bound on the distance an adaptive run can cover before its guard
/// stops it: at most `ceiling / min_callbacks` accepted attempts fit under the
/// ceiling, the first trial step is at most `h0`, and each accepted step grows
/// the proposal by at most `max_growth`. Returns `INFINITY` when no bound applies.
pub fn progress_reach_upper_bound(
    h0: f64,
    max_growth: f64,
    ceiling_callbacks: u64,
    min_callbacks_per_accepted_attempt: u64,
) -> f64 {
    if !(h0.is_finite() && h0 > 0.0 && max_growth.is_finite() && max_growth >= 1.0)
        || min_callbacks_per_accepted_attempt == 0
    {
        return f64::INFINITY;
    }
    let m = ceiling_callbacks / min_callbacks_per_accepted_attempt;
    if m == 0 {
        return 0.0;
    }
    if m > 400 {
        return f64::INFINITY;
    }
    let reach = if max_growth == 1.0 {
        h0 * m as f64
    } else {
        h0 * (max_growth.powi(m as i32) - 1.0) / (max_growth - 1.0)
    };
    if reach.is_finite() {
        reach.next_up()
    } else {
        f64::INFINITY
    }
}

/// Progress model for the progress-bound screen (Section 8.2)."""
F4_E2_OLD = """            let c_min = 16u64 + if problem.is_autonomous() { 0 } else { 1 };
            let m = (ceiling / c_min) as usize;
            let h0 = req.proposed_h.unwrap_or(req.options.h0);
            let g = 5.0f64;
            let reach = if m == 0 {
                0.0
            } else if m > 400 {
                f64::INFINITY
            } else {
                h0 * (g.powi(m as i32) - 1.0) / (g - 1.0)
            };"""
F4_E2_NEW = """            let h0 = req.proposed_h.unwrap_or(req.options.h0);
            let reach = progress_reach_upper_bound(
                h0,
                rodas5p_integrators::AdaptiveStepConfig::default().max_factor,
                ceiling,
                RODAS5P_MIN_CALLBACKS_PER_ACCEPTED_ATTEMPT,
            );"""
F4_E3_OLD = """            min_work_per_attempt: 16 + if autonomous { 0 } else { 1 },"""
F4_E3_NEW = """            min_work_per_attempt: RODAS5P_MIN_CALLBACKS_PER_ACCEPTED_ATTEMPT,"""
F4_E4_OLD = """    fn progress_model(&self, _n: usize, autonomous: bool) -> Option<ProgressModel> {"""
F4_E4_NEW = """    fn progress_model(&self, _n: usize, _autonomous: bool) -> Option<ProgressModel> {"""

F5_OLD = """                    budget = budget.max(k_d0_min);"""
F5_NEW = """                    // B7 keeps `max`: for n <= 35 it leaves the configured first
                    // batch unchanged, for larger dense n it lengthens it. The
                    // shortening `min` variant raised stiff-forced RHS calls from
                    // 400 to 1840 in the design dry run (DESIGN.md, task F5); a
                    // change needs the preregistered node of task X10.
                    budget = budget.max(k_d0_min);"""

F6_E1_OLD = """    let mut replay_safety_audited = false;\n"""
F6_E1_NEW = """    let mut replay_safety_audited = false;\n    let mut competitive_trials_disabled = false;\n"""
F6_E2_OLD = """                        if config.implicit_pilot_attempt_budget > 0 {
                            let remaining = opts.max_steps - result.stats.attempts;
                            if (pilot_eligible || rearm)"""
F6_E2_NEW = """                        if config.implicit_pilot_attempt_budget > 0 {
                            if config.audit_replay_safety && !replay_safety_audited {
                                // Plan section 10.4: evaluate the callbacks twice at the
                                // accepted checkpoint, charge the work, and disable all
                                // competitive trials for the rest of the solve on a mismatch.
                                replay_safety_audited = true;
                                let n = problem.dimension;
                                let (mut a, mut b) = (vec![0.0; n], vec![0.0; n]);
                                let ok_a = (problem.rhs)(start, &result.final_state, &mut a);
                                let ok_b = (problem.rhs)(start, &result.final_state, &mut b);
                                result.stats.rhs_calls += 2;
                                let mut replay_ok = ok_a.is_ok()
                                    && ok_b.is_ok()
                                    && a.iter().zip(&b).all(|(x, z)| x.to_bits() == z.to_bits());
                                if replay_ok && let Jacobian::Jvp(jvp) = &problem.jacobian {
                                    let v = vec![1.0; n];
                                    let ok_c = (jvp)(start, &result.final_state, &v, &mut a);
                                    let ok_d = (jvp)(start, &result.final_state, &v, &mut b);
                                    result.stats.jvp_vectors += 2;
                                    replay_ok = ok_c.is_ok()
                                        && ok_d.is_ok()
                                        && a.iter()
                                            .zip(&b)
                                            .all(|(x, z)| x.to_bits() == z.to_bits());
                                }
                                if !replay_ok {
                                    competitive_trials_disabled = true;
                                }
                            }
                            let remaining = opts.max_steps - result.stats.attempts;
                            if (pilot_eligible || rearm)
                                && !competitive_trials_disabled"""
# The old audit block (35 lines) is read from the base file by build.py.
F6_E3_NEW = ""
F6_E4_OLD = """                            record.decision = "implicit entry not supported by bounded pilot evidence; retain explicit route";"""
F6_E4_NEW = """                            record.decision = if competitive_trials_disabled {
                                "callback replay mismatch: RHS or JVP non-deterministic; competitive trials disabled; retain explicit route"
                            } else {
                                "implicit entry not supported by bounded pilot evidence; retain explicit route"
                            };"""

F7_E1_OLD = "let mut hasher = std::collections::hash_map::DefaultHasher::new();"
F7_E1_NEW = "let mut hasher = Fnv1a64::default();"
F7_E2_OLD = "    std::mem::discriminant(&opts.interior_error).hash(&mut hasher);\n"
F7_E2_NEW = """    let interior: u8 = match opts.interior_error {
        crate::options::InteriorError::WFilteredEnforce => 0,
        crate::options::InteriorError::Enforce => 1,
        crate::options::InteriorError::Off => 2,
    };
    interior.hash(&mut hasher);
"""
F7_E3_OLD = """/// Compute the 64-bit window epoch digest from checkpoint index and window endpoints.
pub fn compute_window_epoch(checkpoint_index: usize, start: f64, target: f64) -> u64 {
    let mut hasher = Fnv1a64::default();
    checkpoint_index.hash(&mut hasher);
"""
F7_E3_NEW = """/// Compute the 64-bit window epoch digest from the window endpoints.
///
/// `_checkpoint_index` is ignored: a window is identified by its endpoints, so
/// the identity built for a batch and the comparisons in the drive loop agree
/// for the same window. The parameter is kept so the public signature does not
/// change.
pub fn compute_window_epoch(_checkpoint_index: usize, start: f64, target: f64) -> u64 {
    let mut hasher = Fnv1a64::default();
"""
F7_E4_OLD = """/// Compute the 64-bit tolerance epoch digest from `Options`."""
F7_E4_NEW = """/// FNV-1a 64-bit hasher for epoch digests.
///
/// `DefaultHasher` output is not specified across Rust releases, so digests
/// built with it are not reproducible across toolchains. FNV-1a is fixed by
/// specification; integers are fed as little-endian bytes and `usize`/`isize`
/// as 64-bit values, so the digest is also platform independent.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Fnv1a64(u64);

impl Default for Fnv1a64 {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for Fnv1a64 {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    fn write_u8(&mut self, i: u8) {
        self.write(&[i]);
    }
    fn write_u16(&mut self, i: u16) {
        self.write(&i.to_le_bytes());
    }
    fn write_u32(&mut self, i: u32) {
        self.write(&i.to_le_bytes());
    }
    fn write_u64(&mut self, i: u64) {
        self.write(&i.to_le_bytes());
    }
    fn write_usize(&mut self, i: usize) {
        self.write(&(i as u64).to_le_bytes());
    }
    fn write_isize(&mut self, i: isize) {
        self.write(&(i as i64).to_le_bytes());
    }
}

#[cfg(test)]
mod fnv_tests {
    use super::Fnv1a64;
    use std::hash::Hasher;

    fn digest(bytes: &[u8]) -> u64 {
        let mut h = Fnv1a64::default();
        h.write(bytes);
        h.finish()
    }

    #[test]
    fn fnv1a64_matches_reference_vectors() {
        assert_eq!(digest(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(digest(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(digest(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn fnv1a64_feeds_integers_little_endian() {
        let mut a = Fnv1a64::default();
        a.write_u64(0x0102_0304_0506_0708);
        let mut b = Fnv1a64::default();
        b.write_usize(0x0102_0304_0506_0708);
        assert_eq!(a.finish(), digest(&[8, 7, 6, 5, 4, 3, 2, 1]));
        assert_eq!(b.finish(), a.finish());
    }
}

/// Compute the 64-bit tolerance epoch digest from `Options`."""

F8_E1_OLD = """    /// Route registry opt-in v2 factory setting (§8.3, §8.4, §8.6)."""
F8_E1_NEW = """    /// Entry policy actually implied by the scheduling fields. A declared
    /// `entry_policy` must equal this value (checked before any callback).
    pub fn effective_entry_policy(&self) -> EntryPolicy {
        if self.implicit_pilot_attempt_budget == 0 {
            EntryPolicy::PressureOnly
        } else if self.entry_probe_adaptation.is_some() {
            EntryPolicy::AdaptiveEvidence
        } else {
            EntryPolicy::Evidence
        }
    }

    /// Route registry opt-in v2 factory setting (§8.3, §8.4, §8.6)."""
F8_E2_OLD = """    if c.window_count == 0
        || c.window_count > 1_000_000"""
F8_E2_NEW = """    if let Some(declared) = c.entry_policy
        && declared != c.effective_entry_policy()
    {
        return Err(bad(
            "declared entry_policy does not match the effective entry schedule",
        ));
    }
    if c.window_count == 0
        || c.window_count > 1_000_000"""

F9_OLD = """    /// Create a Rodas5p driver for the given problem's Jacobian configuration.
    pub fn for_problem(problem: &Problem) -> Self {"""
F9_NEW = """    /// Create a Rodas5p driver for the given problem's Jacobian configuration.
    ///
    /// This is the only route selection the switcher performs: it yields
    /// `Rodas5pDense`, `Rodas5pBanded`, `Rodas5pMatrixFree` or
    /// `Rodas5pFiniteDifference`. `Rodas5pJvpAssembled` (via [`Self::with_route`])
    /// and `PexprbWindowDriver` are stand-alone drivers until a preregistered
    /// admission wires them in.
    pub fn for_problem(problem: &Problem) -> Self {"""

F1_ECON_OLD = """    assert!(
        adaptive_adaptation.loss_margin_backoff,
        "adaptive_evidence must enable loss_margin_backoff"
    );"""
F1_ECON_NEW = """    assert!(
        !adaptive_adaptation.loss_margin_backoff,
        "adaptive_evidence keeps loss_margin_backoff off until a node admits it (DESIGN.md F1)"
    );"""
F4_ECON_OLD_16 = """            min_work_per_attempt: 16,\n"""
F4_ECON_OLD_17 = """            min_work_per_attempt: 17,\n"""
F4_ECON_NEW = """            min_work_per_attempt: 7,\n"""

EDITS = {
    "F1": [(SW, F1_OLD, F1_NEW, 1), (ECON, F1_ECON_OLD, F1_ECON_NEW, 1)],
    "F2": [(SW, F2_E1_OLD, F2_E1_NEW, 1), (SW, F2_E2_OLD, F2_E2_NEW, 1), (SW, F2_E3_OLD, F2_E3_NEW, 1), (SW, F2_E4_OLD, F2_E4_NEW, 1)],
    "F3": [(SW, F3_OLD, F3_NEW, 1), (ADM, F3_ADM_OLD, F3_ADM_NEW, 1)],
    "F4": [(RG, F4_E1_OLD, F4_E1_NEW, 1), (RG, F4_E2_OLD, F4_E2_NEW, 1), (RG, F4_E3_OLD, F4_E3_NEW, 1), (RG, F4_E4_OLD, F4_E4_NEW, 1),
           (ECON, F4_ECON_OLD_16, F4_ECON_NEW, 1), (ECON, F4_ECON_OLD_17, F4_ECON_NEW, 1)],
    "F5": [(SW, F5_OLD, F5_NEW, 1)],
    "F6": [(SW, F6_E1_OLD, F6_E1_NEW, 1), (SW, F6_E2_OLD, F6_E2_NEW, 1), (SW, "@@F6_OLD_AUDIT_BLOCK@@", F6_E3_NEW, 1), (SW, F6_E4_OLD, F6_E4_NEW, 1)],
    "F7": [(RG, F7_E1_OLD, F7_E1_NEW, 3), (RG, F7_E2_OLD, F7_E2_NEW, 1), (RG, F7_E3_OLD, F7_E3_NEW, 1), (RG, F7_E4_OLD, F7_E4_NEW, 1)],
    "F8": [(SW, F8_E1_OLD, F8_E1_NEW, 1), (SW, F8_E2_OLD, F8_E2_NEW, 1)],
    "F9": [(RG, F9_OLD, F9_NEW, 1)],
}

# Lines that a task may remove although a forbidden_removed_pattern matches them.
REMOVALS_ALLOWED = {
    "F3": [{"file": ADM, "line": "assert_eq!(cfg.work_model, WorkModelVersion::V2);"}],
}

# --------------------------------------------------------------------------- tests
TESTS = {}
TESTS["F1"] = r'''
#[test]
fn f1_adaptive_evidence_factory_has_no_loss_margin_backoff() {
    let a = SwitchingConfig::adaptive_evidence(provided());
    let adaptation = a
        .entry_probe_adaptation
        .expect("adaptive_evidence sets an adaptation");
    assert!(!adaptation.loss_margin_backoff);
    let r = SwitchingConfig::route_registry(provided());
    assert!(
        r.entry_probe_adaptation
            .expect("route_registry sets an adaptation")
            .loss_margin_backoff
    );
}
'''
TESTS["F2"] = r'''
#[test]
fn f2_v2_entry_ceiling_stops_losing_rotation_pilots_like_v1() {
    let mut v2 = SwitchingConfig::route_registry(provided());
    v2.work_model = vigilode::WorkModelVersion::V2;
    let mut v1 = v2.clone();
    v1.work_model = vigilode::WorkModelVersion::V1;
    let a = solve(&rotation(), &[1.0, 0.0], &v2);
    let b = solve(&rotation(), &[1.0, 0.0], &v1);
    assert!(a.success && b.success);
    assert_eq!(a.stats.switches, b.stats.switches);
    assert!(
        discarded_rodas_jvp(&a) <= discarded_rodas_jvp(&b) + 50,
        "V2 pilot JVPs {} vs V1 {}",
        discarded_rodas_jvp(&a),
        discarded_rodas_jvp(&b)
    );
}
'''
TESTS["F3"] = r'''
#[test]
fn f3_route_registry_defaults_to_work_model_v1() {
    let r = SwitchingConfig::route_registry(provided());
    assert_eq!(r.work_model, vigilode::WorkModelVersion::V1);
    let sf = solve(&stiff_forced(), &[1.0], &r);
    assert!(sf.success);
    assert_eq!(sf.stats.switches, 1);
    assert!(
        sf.stats.rhs_calls <= 420,
        "stiff-forced RHS {}",
        sf.stats.rhs_calls
    );
}
'''
TESTS["F4"] = r'''
#[test]
fn f4_progress_screen_preserves_entry_decisions() {
    for (p, y0) in [(stiff_forced(), vec![1.0]), (rotation(), vec![1.0, 0.0])] {
        let on = SwitchingConfig::route_registry(provided());
        let mut off = on.clone();
        off.progress_bound = false;
        let a = solve(&p, &y0, &on);
        let b = solve(&p, &y0, &off);
        assert!(a.success && b.success);
        assert_eq!(a.stats.switches, b.stats.switches);
        let entry = |s: &SwitchingSolution| {
            s.stats
                .segments
                .iter()
                .find(|x| x.method == SwitchingMethod::Rodas5p && x.committed)
                .map(|x| x.start_time.to_bits())
        };
        assert_eq!(entry(&a), entry(&b));
    }
}

#[test]
fn f4_progress_reach_upper_bound_boundary_table() {
    use vigilode::progress_reach_upper_bound as reach;
    assert_eq!(reach(1e-3, 5.0, 6, 7), 0.0);
    assert_eq!(reach(1e-3, 5.0, 7, 7), 1e-3f64.next_up());
    assert_eq!(reach(1e-3, 5.0, 14, 7), (1e-3f64 * 6.0).next_up());
    assert_eq!(reach(1e-3, 1.0, 21, 7), (1e-3f64 * 3.0).next_up());
    assert_eq!(reach(1e-3, 5.0, 7 * 401, 7), f64::INFINITY);
    assert_eq!(reach(1e-3, 5.0, 100, 0), f64::INFINITY);
    assert_eq!(reach(0.0, 5.0, 100, 7), f64::INFINITY);
    assert_eq!(reach(1e-3, 0.5, 100, 7), f64::INFINITY);
    assert_eq!(reach(f64::NAN, 5.0, 100, 7), f64::INFINITY);
}
'''
TESTS["F5"] = r'''
#[test]
fn f5_first_explicit_batch_keeps_configured_budget_on_small_dense_routes() {
    let cfg = SwitchingConfig::route_registry(provided());
    assert_eq!(cfg.explicit_attempt_budget, 32);
    let sf = solve(&stiff_forced(), &[1.0], &cfg);
    let first = sf
        .stats
        .segments
        .iter()
        .find(|s| s.method == SwitchingMethod::Dopri5 && !s.is_probe)
        .expect("an explicit batch");
    assert_eq!(first.attempts, 32);
}
'''
TESTS["F6"] = r'''
#[test]
fn f6_replay_mismatch_keeps_explicit_progress_and_disables_pilots() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    CALLS.store(0, Ordering::SeqCst);
    let p = Problem::new(1, |t, y, o| {
        let k = CALLS.fetch_add(1, Ordering::SeqCst);
        o[0] = -10000.0 * (y[0] - t.cos()) - t.sin();
        if k % 2 == 1 {
            o[0] = f64::from_bits(o[0].to_bits() ^ 1);
        }
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, j| {
        j[0] = -10000.0;
        Ok(())
    }))
    .time_derivative(|t, _, o| {
        o[0] = -10000.0 * t.sin() - t.cos();
        Ok(())
    });
    let mut cfg = SwitchingConfig::route_registry(provided());
    cfg.audit_replay_safety = true;
    let sol = solve(&p, &[1.0], &cfg);
    assert!(sol.success);
    assert_eq!(sol.final_time, 1.0);
    assert!(
        !sol.stats
            .segments
            .iter()
            .any(|s| s.method == SwitchingMethod::Rodas5p && s.is_probe),
        "no RODAS pilot may run after a replay mismatch"
    );
    assert!(
        sol.stats
            .segments
            .iter()
            .any(|s| s.decision.contains("competitive trials disabled"))
    );
    let segment_rhs: u64 = sol.stats.segments.iter().map(|s| s.rhs_calls).sum();
    assert_eq!(
        sol.stats.rhs_calls,
        segment_rhs + 2,
        "the two audit RHS calls must be charged"
    );
    assert_eq!(CALLS.load(Ordering::SeqCst) as u64, sol.stats.rhs_calls);
}
'''
TESTS["F6"] += r"""
#[test]
fn f6_replay_audit_charges_deterministic_jvp_calls() {
    let mut cfg = SwitchingConfig::route_registry(provided());
    cfg.audit_replay_safety = true;
    let sol = solve(&rotation(), &[1.0, 0.0], &cfg);
    assert!(sol.success);
    let segment_rhs: u64 = sol.stats.segments.iter().map(|s| s.rhs_calls).sum();
    let segment_jvp: u64 = sol.stats.segments.iter().map(|s| s.jvp_vectors).sum();
    assert_eq!(sol.stats.rhs_calls, segment_rhs + 2, "two audit RHS calls");
    assert_eq!(
        sol.stats.jvp_vectors,
        segment_jvp + 2,
        "two audit JVP calls"
    );
    assert!(
        sol.stats
            .segments
            .iter()
            .any(|s| s.method == SwitchingMethod::Rodas5p && s.is_probe),
        "deterministic callbacks keep competitive pilots enabled"
    );
}

#[test]
fn f6_nondeterministic_jvp_disables_competitive_trials() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static JVP_CALLS: AtomicUsize = AtomicUsize::new(0);
    let p = Problem::new(2, |_, y, o| {
        o[0] = -100.0 * y[1];
        o[1] = 100.0 * y[0];
        Ok(())
    })
    .jacobian(Jacobian::jvp(|_, _, v, o| {
        let k = JVP_CALLS.fetch_add(1, Ordering::SeqCst);
        o[0] = -100.0 * v[1];
        o[1] = 100.0 * v[0];
        if k % 2 == 1 {
            o[0] = f64::from_bits(o[0].to_bits() ^ 1);
        }
        Ok(())
    }))
    .autonomous();
    let mut cfg = SwitchingConfig::route_registry(provided());
    cfg.audit_replay_safety = true;
    let sol = solve(&p, &[1.0, 0.0], &cfg);
    assert!(sol.success);
    assert!(
        !sol.stats
            .segments
            .iter()
            .any(|s| s.method == SwitchingMethod::Rodas5p && s.is_probe),
        "no RODAS pilot may run after a JVP replay mismatch"
    );
    assert!(
        sol.stats
            .segments
            .iter()
            .any(|s| s.decision.contains("competitive trials disabled"))
    );
}
"""
TESTS["F7"] = r'''
#[test]
fn f7_window_epoch_ignores_checkpoint_index_and_is_fnv_stable() {
    use vigilode::compute_window_epoch;
    assert_eq!(compute_window_epoch(0, 0.0, 1.0), 0x85c9_d0b9_5f02_e858);
    assert_eq!(compute_window_epoch(7, 0.0, 1.0), 0x85c9_d0b9_5f02_e858);
    assert_eq!(compute_window_epoch(3, 0.25, 0.5), 0x6a67_54f5_60b8_db95);
}

#[test]
fn f7_tolerance_epoch_is_fnv_stable() {
    use vigilode::{InteriorError, compute_tolerance_epoch};
    let o = Options {
        rtol: 1e-6,
        atol: Atol::Scalar(1e-9),
        interior_error: InteriorError::WFilteredEnforce,
        ..Default::default()
    };
    assert_eq!(compute_tolerance_epoch(&o), 0xd700_4aac_7795_c1b0);
    let off = Options {
        interior_error: InteriorError::Off,
        ..o.clone()
    };
    assert_eq!(compute_tolerance_epoch(&off), 0xd700_4cac_7795_c516);
    let vector = Options {
        atol: Atol::Vector(vec![1e-9, 1e-8]),
        ..o.clone()
    };
    assert_eq!(compute_tolerance_epoch(&vector), 0xa1d2_d72c_3b87_d01d);
}
'''
TESTS["F8"] = r'''
#[test]
fn f8_declared_entry_policy_must_match_effective_schedule() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use vigilode::EntryPolicy;
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    let counting = Problem::new(1, |_, y, o| {
        CALLS.fetch_add(1, Ordering::SeqCst);
        o[0] = -y[0];
        Ok(())
    })
    .jacobian(Jacobian::jvp(|_, _, v, o| {
        o[0] = -v[0];
        Ok(())
    }))
    .autonomous();
    assert_eq!(
        SwitchingConfig::route_registry(provided()).effective_entry_policy(),
        EntryPolicy::AdaptiveEvidence
    );
    let mut c = SwitchingConfig::evidence_driven(provided());
    assert_eq!(c.effective_entry_policy(), EntryPolicy::Evidence);
    c.implicit_pilot_attempt_budget = 0;
    assert_eq!(c.effective_entry_policy(), EntryPolicy::PressureOnly);
    c.entry_policy = Some(EntryPolicy::Evidence);
    CALLS.store(0, Ordering::SeqCst);
    let err = SwitchingSolver::solve(&counting, (0.0, 1.0), &[1.0], &bench_options(), &c)
        .expect_err("a declared policy that the schedule does not implement is refused");
    assert!(
        err.0.contains("declared entry_policy does not match"),
        "{}",
        err.0
    );
    assert_eq!(
        CALLS.load(Ordering::SeqCst),
        0,
        "refusal happens before callbacks"
    );
    c.entry_policy = Some(EntryPolicy::PressureOnly);
    assert!(SwitchingSolver::solve(&counting, (0.0, 1.0), &[1.0], &bench_options(), &c).is_ok());
    c.entry_policy = None;
    assert!(SwitchingSolver::solve(&counting, (0.0, 1.0), &[1.0], &bench_options(), &c).is_ok());
}
'''
TESTS["F9"] = r'''
#[test]
fn f9_switcher_route_selection_is_limited_to_legacy_routes() {
    use vigilode::{Rodas5pWindowDriver, RouteId, WindowDriver};
    assert_eq!(
        Rodas5pWindowDriver::for_problem(&stiff_forced()).route(),
        RouteId::Rodas5pDense
    );
    assert_eq!(
        Rodas5pWindowDriver::for_problem(&rotation()).route(),
        RouteId::Rodas5pMatrixFree
    );
}

#[test]
fn f9_reference_counters_after_remediation() {
    let cases = [
        (
            "evidence_driven",
            SwitchingConfig::evidence_driven(provided()),
            (892, 5371, 355),
            (66, 456),
        ),
        (
            "adaptive_evidence",
            SwitchingConfig::adaptive_evidence(provided()),
            (892, 5371, 355),
            (58, 394),
        ),
        (
            "route_registry",
            SwitchingConfig::route_registry(provided()),
            (883, 5307, 149),
            (59, 400),
        ),
    ];
    for (name, cfg, rot_expected, sf_expected) in cases {
        let rot = solve(&rotation(), &[1.0, 0.0], &cfg);
        assert!(rot.success, "{name}");
        assert_eq!(
            (
                rot.stats.attempts,
                rot.stats.rhs_calls,
                discarded_rodas_jvp(&rot)
            ),
            rot_expected,
            "{name} rotation"
        );
        let sf = solve(&stiff_forced(), &[1.0], &cfg);
        assert!(sf.success, "{name}");
        assert_eq!(
            (sf.stats.attempts, sf.stats.rhs_calls),
            sf_expected,
            "{name} stiff-forced"
        );
        assert_eq!(sf.stats.switches, 1, "{name} stiff-forced switches");
    }
}
'''

# Tests that must be red on the task's base commit (gate command `red`).
RED = {
    "F1": {"tests": ["f1_adaptive_evidence_factory_has_no_loss_margin_backoff"], "allow_compile_red": False, "new_symbols": []},
    "F2": {"tests": ["f2_v2_entry_ceiling_stops_losing_rotation_pilots_like_v1"], "allow_compile_red": False, "new_symbols": []},
    "F3": {"tests": ["f3_route_registry_defaults_to_work_model_v1"], "allow_compile_red": False, "new_symbols": []},
    "F4": {"tests": ["f4_progress_reach_upper_bound_boundary_table"], "allow_compile_red": True, "new_symbols": ["progress_reach_upper_bound"]},
    "F6": {"tests": ["f6_replay_mismatch_keeps_explicit_progress_and_disables_pilots", "f6_replay_audit_charges_deterministic_jvp_calls", "f6_nondeterministic_jvp_disables_competitive_trials"], "allow_compile_red": False, "new_symbols": []},
    "F7": {"tests": ["f7_window_epoch_ignores_checkpoint_index_and_is_fnv_stable", "f7_tolerance_epoch_is_fnv_stable"], "allow_compile_red": False, "new_symbols": []},
    "F8": {"tests": ["f8_declared_entry_policy_must_match_effective_schedule"], "allow_compile_red": True, "new_symbols": ["effective_entry_policy"]},
}
