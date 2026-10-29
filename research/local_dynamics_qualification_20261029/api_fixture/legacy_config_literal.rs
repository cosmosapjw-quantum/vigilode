use vigilode::{DerivativeFallback, EntryProbeAdaptation, SwitchingConfig};

// Intentionally enumerate every field present in the frozen 3064d453 API.
// Do not add struct-update syntax: this fixture checks downstream full literals.
#[allow(dead_code)]
fn legacy_full_literal() -> SwitchingConfig {
    SwitchingConfig {
        fallback: DerivativeFallback::Forbidden,
        window_count: 8,
        explicit_attempt_budget: 32,
        pressure_steps: 24,
        rejection_fraction: 0.3,
        implicit_residence: 2,
        probe_interval: 4,
        probe_attempt_budget: 16,
        return_cost_ratio: 0.8,
        max_dense_dimension: 64,
        implicit_pilot_attempt_budget: 0,
        implicit_pilot_limit: 1,
        entry_cost_ratio: 0.8,
        carry_explicit_step: false,
        entry_pilot_cooldown_batches: 0,
        reuse_entry_pilot_step: false,
        retry_unresolved_entry_once: false,
        pilot_backoff_factor: 2,
        probe_backoff_factor: 2,
        rearm_rate_ratio: 2.0,
        return_sentinel_interval: 0,
        entry_probe_adaptation: None::<EntryProbeAdaptation>,
    }
}

fn main() {}
