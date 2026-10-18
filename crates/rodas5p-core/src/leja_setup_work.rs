//! Separate accounting for Leja cold setup and cache preparation.
//!
//! These counters do not alter the historical `WorkCounters` or the
//! `CertifiedLejaReport` wire schema. Directed work is counted at the
//! `Interval`/kernel invocation boundary: one call to `exp_interval` is one
//! invocation, regardless of its internal scalar operations. Ordinary
//! binary64, trigonometric, and logarithmic work is recorded as documented
//! proxy units, not as an exact operation count.
//!
//! `scalar_proxy_units` is a source-level proxy for ordinary operations in
//! Leja point, prefix, and suffix table generation; it is not a hardware-op
//! count. Coefficient-table numerical work is represented by interval/kernel
//! invocations, coefficient-table attempts/completions, and requested setup
//! allocation sizes. Diagnostic error-string allocations are outside these
//! setup-buffer request counters.

use serde::Serialize;
use std::sync::OnceLock;

pub const LEJA_SETUP_RECEIPT_SCHEMA_V1: &str = "vigilode-leja-setup-cost-v1";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SetupCoverage {
    #[default]
    Complete,
    PartialUnattributed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OnceLockObservation {
    #[default]
    NotObserved,
    InitializedByThisCall,
    WarmCountedInitializer,
    PreexistingUnattributed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CoefficientCacheObservation {
    #[default]
    NotObserved,
    PreparedByThisCall,
    WarmCountedEntry,
    PreexistingUnattributed,
}

/// Immutable, dimensionally named work subtotal for setup and cache activity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct LejaSetupWorkSubtotal {
    interval_kernel_invocations: u64,
    scalar_proxy_units: u64,
    allocation_requests: u64,
    allocation_items_requested: u64,
    coefficient_tables_attempted: u64,
    coefficient_tables_completed: u64,
    cache_api_attempts: u64,
    cache_lookup_attempts: u64,
    cache_hits: u64,
    cache_misses: u64,
    cache_insert_attempts: u64,
    cache_inserts_completed: u64,
}

impl LejaSetupWorkSubtotal {
    pub const fn interval_kernel_invocations(self) -> u64 {
        self.interval_kernel_invocations
    }
    pub const fn scalar_proxy_units(self) -> u64 {
        self.scalar_proxy_units
    }
    pub const fn allocation_requests(self) -> u64 {
        self.allocation_requests
    }
    pub const fn allocation_items_requested(self) -> u64 {
        self.allocation_items_requested
    }
    pub const fn coefficient_tables_attempted(self) -> u64 {
        self.coefficient_tables_attempted
    }
    pub const fn coefficient_tables_completed(self) -> u64 {
        self.coefficient_tables_completed
    }
    pub const fn cache_api_attempts(self) -> u64 {
        self.cache_api_attempts
    }
    pub const fn cache_lookup_attempts(self) -> u64 {
        self.cache_lookup_attempts
    }
    pub const fn cache_hits(self) -> u64 {
        self.cache_hits
    }
    pub const fn cache_misses(self) -> u64 {
        self.cache_misses
    }
    pub const fn cache_insert_attempts(self) -> u64 {
        self.cache_insert_attempts
    }
    pub const fn cache_inserts_completed(self) -> u64 {
        self.cache_inserts_completed
    }
}

/// Immutable v1 snapshot of one caller's setup-cost accumulator.
///
/// ```compile_fail
/// use rodas5p_core::LejaSetupWork;
/// let mut receipt = LejaSetupWork::new().snapshot();
/// receipt.subtotal.interval_kernel_invocations = 99;
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct LejaSetupReceiptV1 {
    schema: &'static str,
    subtotal: LejaSetupWorkSubtotal,
    setup_coverage: SetupCoverage,
    leja_points_observation: OnceLockObservation,
    node_bounds_observation: OnceLockObservation,
    suffix_bounds_observation: OnceLockObservation,
    leja_points_history_available: bool,
    node_bounds_history_available: bool,
    suffix_bounds_history_available: bool,
    leja_points_history: Option<LejaSetupWorkSubtotal>,
    node_bounds_history: Option<LejaSetupWorkSubtotal>,
    suffix_bounds_history: Option<LejaSetupWorkSubtotal>,
    leja_points_history_coverage: Option<SetupCoverage>,
    node_bounds_history_coverage: Option<SetupCoverage>,
    suffix_bounds_history_coverage: Option<SetupCoverage>,
    coefficient_cache_observation: CoefficientCacheObservation,
    coefficient_cache_key_bits: Option<[u64; 3]>,
    coefficient_cache_history: Option<LejaSetupWorkSubtotal>,
    counter_overflowed: bool,
}

impl LejaSetupReceiptV1 {
    pub const fn schema(self) -> &'static str {
        self.schema
    }
    pub const fn subtotal(self) -> LejaSetupWorkSubtotal {
        self.subtotal
    }
    pub const fn setup_coverage(self) -> SetupCoverage {
        self.setup_coverage
    }
    pub const fn leja_points_observation(self) -> OnceLockObservation {
        self.leja_points_observation
    }
    pub const fn node_bounds_observation(self) -> OnceLockObservation {
        self.node_bounds_observation
    }
    pub const fn suffix_bounds_observation(self) -> OnceLockObservation {
        self.suffix_bounds_observation
    }
    pub const fn leja_points_history_available(self) -> bool {
        self.leja_points_history_available
    }
    pub const fn node_bounds_history_available(self) -> bool {
        self.node_bounds_history_available
    }
    pub const fn suffix_bounds_history_available(self) -> bool {
        self.suffix_bounds_history_available
    }
    pub const fn leja_points_history(self) -> Option<LejaSetupWorkSubtotal> {
        self.leja_points_history
    }
    pub const fn node_bounds_history(self) -> Option<LejaSetupWorkSubtotal> {
        self.node_bounds_history
    }
    pub const fn suffix_bounds_history(self) -> Option<LejaSetupWorkSubtotal> {
        self.suffix_bounds_history
    }
    pub const fn leja_points_history_coverage(self) -> Option<SetupCoverage> {
        self.leja_points_history_coverage
    }
    pub const fn node_bounds_history_coverage(self) -> Option<SetupCoverage> {
        self.node_bounds_history_coverage
    }
    pub const fn suffix_bounds_history_coverage(self) -> Option<SetupCoverage> {
        self.suffix_bounds_history_coverage
    }
    pub const fn coefficient_cache_observation(self) -> CoefficientCacheObservation {
        self.coefficient_cache_observation
    }
    pub const fn coefficient_cache_key_bits(self) -> Option<[u64; 3]> {
        self.coefficient_cache_key_bits
    }
    pub const fn coefficient_cache_history(self) -> Option<LejaSetupWorkSubtotal> {
        self.coefficient_cache_history
    }
    pub const fn counter_overflowed(self) -> bool {
        self.counter_overflowed
    }
}

/// Caller-owned mutable accumulator. Its fields are private; snapshots are
/// immutable receipts, and attempted work remains visible after refusal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LejaSetupWork {
    subtotal: LejaSetupWorkSubtotal,
    setup_coverage: SetupCoverage,
    leja_points_observation: OnceLockObservation,
    node_bounds_observation: OnceLockObservation,
    suffix_bounds_observation: OnceLockObservation,
    leja_points_history_available: bool,
    node_bounds_history_available: bool,
    suffix_bounds_history_available: bool,
    leja_points_history: Option<LejaSetupWorkSubtotal>,
    node_bounds_history: Option<LejaSetupWorkSubtotal>,
    suffix_bounds_history: Option<LejaSetupWorkSubtotal>,
    leja_points_history_coverage: Option<SetupCoverage>,
    node_bounds_history_coverage: Option<SetupCoverage>,
    suffix_bounds_history_coverage: Option<SetupCoverage>,
    coefficient_cache_observation: CoefficientCacheObservation,
    coefficient_cache_key_bits: Option<[u64; 3]>,
    coefficient_cache_history: Option<LejaSetupWorkSubtotal>,
    counter_overflowed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StaticLejaTable {
    Points,
    NodeBounds,
    SuffixBounds,
}

pub(crate) struct CountedStaticTable<T> {
    pub(crate) value: T,
    counted_initializer: bool,
    receipt: Option<LejaSetupWorkSubtotal>,
    coverage: SetupCoverage,
}

impl<T> CountedStaticTable<T> {}

/// Shared `OnceLock` attribution helper. The closure's local work is merged
/// only by the closure winner; warm callers observe metadata but receive no
/// historical work as current-call cost.
pub(crate) fn get_or_init_static<'a, T, F>(
    cell: &'a OnceLock<CountedStaticTable<T>>,
    table: StaticLejaTable,
    work: Option<&mut LejaSetupWork>,
    build: F,
) -> &'a T
where
    F: FnOnce(Option<&mut LejaSetupWork>) -> T,
{
    let counted_request = work.is_some();
    let mut initialized_here = false;
    let mut local = LejaSetupWork::new();
    let stored = cell.get_or_init(|| {
        initialized_here = true;
        let value = if counted_request {
            build(Some(&mut local))
        } else {
            build(None)
        };
        let subtotal = if counted_request {
            Some(local.subtotal)
        } else {
            None
        };
        CountedStaticTable {
            value,
            counted_initializer: counted_request,
            receipt: subtotal,
            coverage: if counted_request {
                local.setup_coverage
            } else {
                SetupCoverage::PartialUnattributed
            },
        }
    });
    if let Some(work) = work {
        work.observe_static_table(
            table,
            initialized_here,
            stored.counted_initializer,
            stored.receipt.is_some(),
            stored.receipt,
            if stored.counted_initializer {
                Some(stored.coverage)
            } else {
                None
            },
        );
        if initialized_here {
            work.accumulate(&local);
        }
    }
    &stored.value
}

impl LejaSetupWork {
    pub const fn new() -> Self {
        Self {
            subtotal: LejaSetupWorkSubtotal {
                interval_kernel_invocations: 0,
                scalar_proxy_units: 0,
                allocation_requests: 0,
                allocation_items_requested: 0,
                coefficient_tables_attempted: 0,
                coefficient_tables_completed: 0,
                cache_api_attempts: 0,
                cache_lookup_attempts: 0,
                cache_hits: 0,
                cache_misses: 0,
                cache_insert_attempts: 0,
                cache_inserts_completed: 0,
            },
            setup_coverage: SetupCoverage::Complete,
            leja_points_observation: OnceLockObservation::NotObserved,
            node_bounds_observation: OnceLockObservation::NotObserved,
            suffix_bounds_observation: OnceLockObservation::NotObserved,
            leja_points_history_available: false,
            node_bounds_history_available: false,
            suffix_bounds_history_available: false,
            leja_points_history: None,
            node_bounds_history: None,
            suffix_bounds_history: None,
            leja_points_history_coverage: None,
            node_bounds_history_coverage: None,
            suffix_bounds_history_coverage: None,
            coefficient_cache_observation: CoefficientCacheObservation::NotObserved,
            coefficient_cache_key_bits: None,
            coefficient_cache_history: None,
            counter_overflowed: false,
        }
    }

    pub const fn snapshot(self) -> LejaSetupReceiptV1 {
        LejaSetupReceiptV1 {
            schema: LEJA_SETUP_RECEIPT_SCHEMA_V1,
            subtotal: self.subtotal,
            setup_coverage: self.setup_coverage,
            leja_points_observation: self.leja_points_observation,
            node_bounds_observation: self.node_bounds_observation,
            suffix_bounds_observation: self.suffix_bounds_observation,
            leja_points_history_available: self.leja_points_history_available,
            node_bounds_history_available: self.node_bounds_history_available,
            suffix_bounds_history_available: self.suffix_bounds_history_available,
            leja_points_history: self.leja_points_history,
            node_bounds_history: self.node_bounds_history,
            suffix_bounds_history: self.suffix_bounds_history,
            leja_points_history_coverage: self.leja_points_history_coverage,
            node_bounds_history_coverage: self.node_bounds_history_coverage,
            suffix_bounds_history_coverage: self.suffix_bounds_history_coverage,
            coefficient_cache_observation: self.coefficient_cache_observation,
            coefficient_cache_key_bits: self.coefficient_cache_key_bits,
            coefficient_cache_history: self.coefficient_cache_history,
            counter_overflowed: self.counter_overflowed,
        }
    }

    pub(crate) fn interval_kernel_invocation(&mut self) {
        let subtotal = &mut self.subtotal;
        Self::add_counter(
            &mut subtotal.interval_kernel_invocations,
            1,
            &mut self.counter_overflowed,
        );
    }

    pub(crate) fn scalar_proxy_units(&mut self, count: u64) {
        Self::add_counter(
            &mut self.subtotal.scalar_proxy_units,
            count,
            &mut self.counter_overflowed,
        );
    }

    pub(crate) fn allocation_request(&mut self, items: u64) {
        Self::add_counter(
            &mut self.subtotal.allocation_requests,
            1,
            &mut self.counter_overflowed,
        );
        Self::add_counter(
            &mut self.subtotal.allocation_items_requested,
            items,
            &mut self.counter_overflowed,
        );
    }

    pub(crate) fn coefficient_table_attempt(&mut self) {
        Self::add_counter(
            &mut self.subtotal.coefficient_tables_attempted,
            1,
            &mut self.counter_overflowed,
        );
    }

    pub(crate) fn coefficient_table_complete(&mut self) {
        Self::add_counter(
            &mut self.subtotal.coefficient_tables_completed,
            1,
            &mut self.counter_overflowed,
        );
    }

    pub(crate) fn cache_api_attempt(&mut self) {
        Self::add_counter(
            &mut self.subtotal.cache_api_attempts,
            1,
            &mut self.counter_overflowed,
        );
    }

    pub(crate) fn cache_lookup(&mut self, hit: bool) {
        Self::add_counter(
            &mut self.subtotal.cache_lookup_attempts,
            1,
            &mut self.counter_overflowed,
        );
        if hit {
            Self::add_counter(
                &mut self.subtotal.cache_hits,
                1,
                &mut self.counter_overflowed,
            );
        } else {
            Self::add_counter(
                &mut self.subtotal.cache_misses,
                1,
                &mut self.counter_overflowed,
            );
        }
    }

    pub(crate) fn cache_insert_attempt(&mut self) {
        Self::add_counter(
            &mut self.subtotal.cache_insert_attempts,
            1,
            &mut self.counter_overflowed,
        );
    }

    pub(crate) fn cache_insert_complete(&mut self) {
        Self::add_counter(
            &mut self.subtotal.cache_inserts_completed,
            1,
            &mut self.counter_overflowed,
        );
    }

    pub(crate) fn record_coefficient_cache_entry(
        &mut self,
        key_bits: [u64; 3],
        prepared_here: bool,
        history: Option<LejaSetupWorkSubtotal>,
        history_coverage: Option<SetupCoverage>,
    ) {
        self.coefficient_cache_observation = if prepared_here {
            CoefficientCacheObservation::PreparedByThisCall
        } else if history.is_some() {
            CoefficientCacheObservation::WarmCountedEntry
        } else {
            self.setup_coverage = SetupCoverage::PartialUnattributed;
            CoefficientCacheObservation::PreexistingUnattributed
        };
        if history_coverage == Some(SetupCoverage::PartialUnattributed) {
            self.setup_coverage = SetupCoverage::PartialUnattributed;
        }
        self.coefficient_cache_key_bits = Some(key_bits);
        self.coefficient_cache_history = history;
    }

    pub(crate) fn current_subtotal(&self) -> LejaSetupWorkSubtotal {
        self.subtotal
    }

    pub(crate) const fn current_coverage(&self) -> SetupCoverage {
        self.setup_coverage
    }

    pub(crate) fn delta_since(
        &self,
        previous: LejaSetupWorkSubtotal,
    ) -> Option<LejaSetupWorkSubtotal> {
        if self.counter_overflowed {
            return None;
        }
        let current = self.subtotal;
        Some(LejaSetupWorkSubtotal {
            interval_kernel_invocations: current
                .interval_kernel_invocations
                .checked_sub(previous.interval_kernel_invocations)?,
            scalar_proxy_units: current
                .scalar_proxy_units
                .checked_sub(previous.scalar_proxy_units)?,
            allocation_requests: current
                .allocation_requests
                .checked_sub(previous.allocation_requests)?,
            allocation_items_requested: current
                .allocation_items_requested
                .checked_sub(previous.allocation_items_requested)?,
            coefficient_tables_attempted: current
                .coefficient_tables_attempted
                .checked_sub(previous.coefficient_tables_attempted)?,
            coefficient_tables_completed: current
                .coefficient_tables_completed
                .checked_sub(previous.coefficient_tables_completed)?,
            cache_api_attempts: current
                .cache_api_attempts
                .checked_sub(previous.cache_api_attempts)?,
            cache_lookup_attempts: current
                .cache_lookup_attempts
                .checked_sub(previous.cache_lookup_attempts)?,
            cache_hits: current.cache_hits.checked_sub(previous.cache_hits)?,
            cache_misses: current.cache_misses.checked_sub(previous.cache_misses)?,
            cache_insert_attempts: current
                .cache_insert_attempts
                .checked_sub(previous.cache_insert_attempts)?,
            cache_inserts_completed: current
                .cache_inserts_completed
                .checked_sub(previous.cache_inserts_completed)?,
        })
    }

    pub(crate) fn observe_static_table(
        &mut self,
        table: StaticLejaTable,
        initialized_here: bool,
        initializer_was_counted: bool,
        history_available: bool,
        history_receipt: Option<LejaSetupWorkSubtotal>,
        history_coverage: Option<SetupCoverage>,
    ) {
        let observation = if initialized_here {
            OnceLockObservation::InitializedByThisCall
        } else if initializer_was_counted {
            OnceLockObservation::WarmCountedInitializer
        } else {
            self.setup_coverage = SetupCoverage::PartialUnattributed;
            OnceLockObservation::PreexistingUnattributed
        };
        if history_coverage == Some(SetupCoverage::PartialUnattributed) {
            self.setup_coverage = SetupCoverage::PartialUnattributed;
        }
        match table {
            StaticLejaTable::Points => {
                self.leja_points_observation = observation;
                self.leja_points_history_available = history_available;
                self.leja_points_history = history_receipt;
                self.leja_points_history_coverage = history_coverage;
            }
            StaticLejaTable::NodeBounds => {
                self.node_bounds_observation = observation;
                self.node_bounds_history_available = history_available;
                self.node_bounds_history = history_receipt;
                self.node_bounds_history_coverage = history_coverage;
            }
            StaticLejaTable::SuffixBounds => {
                self.suffix_bounds_observation = observation;
                self.suffix_bounds_history_available = history_available;
                self.suffix_bounds_history = history_receipt;
                self.suffix_bounds_history_coverage = history_coverage;
            }
        }
    }

    pub(crate) fn accumulate(&mut self, other: &Self) {
        Self::add_counter(
            &mut self.subtotal.interval_kernel_invocations,
            other.subtotal.interval_kernel_invocations,
            &mut self.counter_overflowed,
        );
        Self::add_counter(
            &mut self.subtotal.scalar_proxy_units,
            other.subtotal.scalar_proxy_units,
            &mut self.counter_overflowed,
        );
        Self::add_counter(
            &mut self.subtotal.allocation_requests,
            other.subtotal.allocation_requests,
            &mut self.counter_overflowed,
        );
        Self::add_counter(
            &mut self.subtotal.allocation_items_requested,
            other.subtotal.allocation_items_requested,
            &mut self.counter_overflowed,
        );
        Self::add_counter(
            &mut self.subtotal.coefficient_tables_attempted,
            other.subtotal.coefficient_tables_attempted,
            &mut self.counter_overflowed,
        );
        Self::add_counter(
            &mut self.subtotal.coefficient_tables_completed,
            other.subtotal.coefficient_tables_completed,
            &mut self.counter_overflowed,
        );
        Self::add_counter(
            &mut self.subtotal.cache_api_attempts,
            other.subtotal.cache_api_attempts,
            &mut self.counter_overflowed,
        );
        Self::add_counter(
            &mut self.subtotal.cache_lookup_attempts,
            other.subtotal.cache_lookup_attempts,
            &mut self.counter_overflowed,
        );
        Self::add_counter(
            &mut self.subtotal.cache_hits,
            other.subtotal.cache_hits,
            &mut self.counter_overflowed,
        );
        Self::add_counter(
            &mut self.subtotal.cache_misses,
            other.subtotal.cache_misses,
            &mut self.counter_overflowed,
        );
        Self::add_counter(
            &mut self.subtotal.cache_insert_attempts,
            other.subtotal.cache_insert_attempts,
            &mut self.counter_overflowed,
        );
        Self::add_counter(
            &mut self.subtotal.cache_inserts_completed,
            other.subtotal.cache_inserts_completed,
            &mut self.counter_overflowed,
        );
        if other.setup_coverage == SetupCoverage::PartialUnattributed {
            self.setup_coverage = SetupCoverage::PartialUnattributed;
        }
        self.counter_overflowed |= other.counter_overflowed;
        if other.leja_points_observation != OnceLockObservation::NotObserved {
            self.leja_points_observation = other.leja_points_observation;
            self.leja_points_history_available = other.leja_points_history_available;
            self.leja_points_history = other.leja_points_history;
            self.leja_points_history_coverage = other.leja_points_history_coverage;
        }
        if other.node_bounds_observation != OnceLockObservation::NotObserved {
            self.node_bounds_observation = other.node_bounds_observation;
            self.node_bounds_history_available = other.node_bounds_history_available;
            self.node_bounds_history = other.node_bounds_history;
            self.node_bounds_history_coverage = other.node_bounds_history_coverage;
        }
        if other.suffix_bounds_observation != OnceLockObservation::NotObserved {
            self.suffix_bounds_observation = other.suffix_bounds_observation;
            self.suffix_bounds_history_available = other.suffix_bounds_history_available;
            self.suffix_bounds_history = other.suffix_bounds_history;
            self.suffix_bounds_history_coverage = other.suffix_bounds_history_coverage;
        }
        if other.coefficient_cache_observation != CoefficientCacheObservation::NotObserved {
            self.coefficient_cache_observation = other.coefficient_cache_observation;
            self.coefficient_cache_key_bits = other.coefficient_cache_key_bits;
            self.coefficient_cache_history = other.coefficient_cache_history;
        }
    }

    fn add_counter(slot: &mut u64, value: u64, overflowed: &mut bool) {
        match slot.checked_add(value) {
            Some(next) => *slot = next,
            None => {
                *slot = u64::MAX;
                *overflowed = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};
    use std::thread;

    #[test]
    fn local_once_lock_charges_only_the_initializer_winner() {
        let cell = Arc::new(OnceLock::new());
        let gate = Arc::new(Barrier::new(8));
        let calls = Arc::new(AtomicUsize::new(0));
        let mut threads = Vec::new();
        for _ in 0..8 {
            let cell = Arc::clone(&cell);
            let gate = Arc::clone(&gate);
            let calls = Arc::clone(&calls);
            threads.push(thread::spawn(move || {
                let mut work = LejaSetupWork::new();
                gate.wait();
                let value =
                    *get_or_init_static(&cell, StaticLejaTable::Points, Some(&mut work), |local| {
                        calls.fetch_add(1, Ordering::SeqCst);
                        let local = local.expect("counted initializer");
                        local.scalar_proxy_units(11);
                        local.allocation_request(3);
                        29_u64
                    });
                (value, work.snapshot())
            }));
        }
        let receipts: Vec<_> = threads
            .into_iter()
            .map(|thread| thread.join().expect("worker thread"))
            .collect();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let charged: Vec<_> = receipts
            .iter()
            .filter(|(_, receipt)| {
                receipt.leja_points_observation() == OnceLockObservation::InitializedByThisCall
            })
            .collect();
        assert_eq!(charged.len(), 1);
        assert_eq!(charged[0].0, 29);
        assert_eq!(charged[0].1.subtotal().scalar_proxy_units(), 11);
        assert_eq!(charged[0].1.subtotal().allocation_requests(), 1);
        assert_eq!(
            charged[0]
                .1
                .leja_points_history()
                .unwrap()
                .scalar_proxy_units(),
            11
        );
        for (_, receipt) in &receipts {
            assert!(receipt.leja_points_history_available());
            assert_eq!(
                receipt.leja_points_history().unwrap().scalar_proxy_units(),
                11
            );
            if receipt.leja_points_observation() == OnceLockObservation::WarmCountedInitializer {
                assert_eq!(receipt.subtotal().scalar_proxy_units(), 0);
            }
        }
        assert_eq!(
            receipts
                .iter()
                .map(|(_, r)| r.subtotal().scalar_proxy_units())
                .sum::<u64>(),
            11
        );
    }

    #[test]
    fn local_legacy_once_lock_is_partial_unattributed_without_refusal() {
        let cell = OnceLock::new();
        let value = get_or_init_static(&cell, StaticLejaTable::Points, None, |_| 17_u64);
        assert_eq!(*value, 17);

        let mut work = LejaSetupWork::new();
        assert_eq!(
            *get_or_init_static(&cell, StaticLejaTable::Points, Some(&mut work), |_| 99),
            17
        );
        let receipt = work.snapshot();
        assert_eq!(
            receipt.leja_points_observation(),
            OnceLockObservation::PreexistingUnattributed
        );
        assert_eq!(receipt.setup_coverage(), SetupCoverage::PartialUnattributed);
        assert!(!receipt.leja_points_history_available());
        assert!(receipt.leja_points_history().is_none());
        assert_eq!(receipt.subtotal().scalar_proxy_units(), 0);
    }

    #[test]
    fn local_counted_table_initializer_records_cold_and_warm_cost_once() {
        let cell = OnceLock::new();
        let mut cold = LejaSetupWork::new();
        let cold_value =
            get_or_init_static(&cell, StaticLejaTable::Points, Some(&mut cold), |work| {
                work.unwrap().allocation_request(4);
                vec![2_u8, 3, 5, 7]
            });
        assert_eq!(cold_value, &[2, 3, 5, 7]);
        let cold = cold.snapshot();
        assert_eq!(
            cold.leja_points_observation(),
            OnceLockObservation::InitializedByThisCall
        );
        assert_eq!(cold.subtotal().allocation_requests(), 1);
        assert_eq!(cold.subtotal().allocation_items_requested(), 4);

        let mut warm = LejaSetupWork::new();
        let warm_value =
            get_or_init_static(&cell, StaticLejaTable::Points, Some(&mut warm), |_| {
                panic!("warm table must not rerun its initializer")
            });
        assert_eq!(warm_value, &[2, 3, 5, 7]);
        let warm = warm.snapshot();
        assert_eq!(
            warm.leja_points_observation(),
            OnceLockObservation::WarmCountedInitializer
        );
        assert_eq!(warm.leja_points_history(), Some(cold.subtotal()));
        assert_eq!(warm.subtotal().allocation_requests(), 0);
        assert_eq!(warm.subtotal().allocation_items_requested(), 0);
    }
}
